/*!
 * 快捷键管理器模块
 *
 * 提供快捷键的增删改查、导入导出、重置等管理功能。
 */

use crate::config::types::{ShortcutBinding, ShortcutsConfig};
use crate::utils::error::AppResult;
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

/// 快捷键管理器
pub struct ShortcutManager {
    /// 当前快捷键配置
    config: ShortcutsConfig,
}

/// 快捷键类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShortcutCategory {
    Global,
    Terminal,
    Custom,
}

impl ShortcutManager {
    /// 创建新的快捷键管理器
    pub fn new(config: ShortcutsConfig) -> Self {
        Self { config }
    }

    /// 获取当前配置
    pub fn get_config(&self) -> &ShortcutsConfig {
        &self.config
    }

    /// 更新整个配置
    pub fn update_config(&mut self, config: ShortcutsConfig) {
        self.config = config;
    }

    /// 添加快捷键
    pub fn add_shortcut(
        &mut self,
        category: ShortcutCategory,
        shortcut: ShortcutBinding,
    ) -> AppResult<()> {
        // 检查是否已存在相同的快捷键组合
        if self.shortcut_exists(&shortcut, category) {
            bail!("快捷键组合已存在: {}", self.format_shortcut(&shortcut));
        }

        let shortcuts = self.get_shortcuts_mut(category);
        shortcuts.push(shortcut);
        Ok(())
    }

    /// 删除快捷键
    pub fn remove_shortcut(
        &mut self,
        category: ShortcutCategory,
        index: usize,
    ) -> AppResult<ShortcutBinding> {
        let shortcuts = self.get_shortcuts_mut(category);

        if index >= shortcuts.len() {
            bail!("快捷键索引超出范围: {}", index);
        }

        Ok(shortcuts.remove(index))
    }

    /// 更新快捷键
    pub fn update_shortcut(
        &mut self,
        category: ShortcutCategory,
        index: usize,
        new_shortcut: ShortcutBinding,
    ) -> AppResult<()> {
        // 检查索引是否有效
        if index >= self.get_shortcuts(category).len() {
            bail!("快捷键索引超出范围: {}", index);
        }

        // 检查新快捷键是否与其他快捷键冲突（排除当前位置）
        if self.shortcut_exists_excluding(&new_shortcut, category, index) {
            bail!("快捷键组合已存在: {}", self.format_shortcut(&new_shortcut));
        }

        let shortcuts = self.get_shortcuts_mut(category);
        shortcuts[index] = new_shortcut;
        Ok(())
    }

    /// 获取指定类别的快捷键列表
    pub fn get_shortcuts(&self, category: ShortcutCategory) -> &[ShortcutBinding] {
        match category {
            ShortcutCategory::Global => &self.config.global,
            ShortcutCategory::Terminal => &self.config.terminal,
            ShortcutCategory::Custom => &self.config.custom,
        }
    }

    /// 查找快捷键
    pub fn find_shortcut(
        &self,
        key: &str,
        modifiers: &[String],
    ) -> Option<(ShortcutCategory, usize, &ShortcutBinding)> {
        // 在全局快捷键中查找
        for (index, shortcut) in self.config.global.iter().enumerate() {
            if shortcut.key == key && shortcut.modifiers == modifiers {
                return Some((ShortcutCategory::Global, index, shortcut));
            }
        }

        // 在终端快捷键中查找
        for (index, shortcut) in self.config.terminal.iter().enumerate() {
            if shortcut.key == key && shortcut.modifiers == modifiers {
                return Some((ShortcutCategory::Terminal, index, shortcut));
            }
        }

        // 在自定义快捷键中查找
        for (index, shortcut) in self.config.custom.iter().enumerate() {
            if shortcut.key == key && shortcut.modifiers == modifiers {
                return Some((ShortcutCategory::Custom, index, shortcut));
            }
        }

        None
    }

    /// 重置到默认配置
    pub fn reset_to_defaults(&mut self) {
        self.config = crate::config::defaults::create_default_shortcuts_config();
    }

    /// 重置指定类别到默认配置
    pub fn reset_category_to_defaults(&mut self, category: ShortcutCategory) {
        let default_config = crate::config::defaults::create_default_shortcuts_config();

        match category {
            ShortcutCategory::Global => {
                self.config.global = default_config.global;
            }
            ShortcutCategory::Terminal => {
                self.config.terminal = default_config.terminal;
            }
            ShortcutCategory::Custom => {
                self.config.custom = default_config.custom;
            }
        }
    }

    /// 清空指定类别的快捷键
    pub fn clear_category(&mut self, category: ShortcutCategory) {
        let shortcuts = self.get_shortcuts_mut(category);
        shortcuts.clear();
    }

    /// 获取快捷键统计信息
    pub fn get_statistics(&self) -> ShortcutStatistics {
        ShortcutStatistics {
            global_count: self.config.global.len(),
            terminal_count: self.config.terminal.len(),
            custom_count: self.config.custom.len(),
            total_count: self.config.global.len()
                + self.config.terminal.len()
                + self.config.custom.len(),
        }
    }

    /// 导出配置为 JSON
    pub fn export_to_json(&self) -> AppResult<String> {
        serde_json::to_string_pretty(&self.config).context("导出快捷键配置为 JSON 失败")
    }

    /// 从 JSON 导入配置
    pub fn import_from_json(&mut self, json: &str) -> AppResult<()> {
        let config: ShortcutsConfig =
            serde_json::from_str(json).context("从 JSON 导入快捷键配置失败")?;

        self.config = config;
        Ok(())
    }

    /// 获取可变的快捷键列表引用
    fn get_shortcuts_mut(&mut self, category: ShortcutCategory) -> &mut Vec<ShortcutBinding> {
        match category {
            ShortcutCategory::Global => &mut self.config.global,
            ShortcutCategory::Terminal => &mut self.config.terminal,
            ShortcutCategory::Custom => &mut self.config.custom,
        }
    }

    /// 检查快捷键是否已存在
    fn shortcut_exists(&self, shortcut: &ShortcutBinding, category: ShortcutCategory) -> bool {
        let shortcuts = self.get_shortcuts(category);
        shortcuts
            .iter()
            .any(|s| s.key == shortcut.key && s.modifiers == shortcut.modifiers)
    }

    /// 检查快捷键是否已存在（排除指定索引）
    fn shortcut_exists_excluding(
        &self,
        shortcut: &ShortcutBinding,
        category: ShortcutCategory,
        exclude_index: usize,
    ) -> bool {
        let shortcuts = self.get_shortcuts(category);
        shortcuts.iter().enumerate().any(|(i, s)| {
            i != exclude_index && s.key == shortcut.key && s.modifiers == shortcut.modifiers
        })
    }

    /// 格式化快捷键为字符串
    fn format_shortcut(&self, shortcut: &ShortcutBinding) -> String {
        let mut parts = shortcut.modifiers.clone();
        parts.push(shortcut.key.clone());
        parts.join("+")
    }
}

/// 快捷键统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutStatistics {
    /// 全局快捷键数量
    pub global_count: usize,
    /// 终端快捷键数量
    pub terminal_count: usize,
    /// 自定义快捷键数量
    pub custom_count: usize,
    /// 总快捷键数量
    pub total_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::{ShortcutAction, ShortcutBinding, ShortcutsConfig};

    fn create_test_shortcut(key: &str, modifiers: Vec<&str>, action: &str) -> ShortcutBinding {
        ShortcutBinding {
            key: key.to_string(),
            modifiers: modifiers.into_iter().map(|s| s.to_string()).collect(),
            action: ShortcutAction::Simple(action.to_string()),
        }
    }

    #[test]
    fn test_manager_creation() {
        let config = ShortcutsConfig {
            global: vec![],
            terminal: vec![],
            custom: vec![],
        };
        let manager = ShortcutManager::new(config);
        assert_eq!(manager.get_statistics().total_count, 0);
    }

    #[test]
    fn test_add_shortcut() {
        let config = ShortcutsConfig {
            global: vec![],
            terminal: vec![],
            custom: vec![],
        };
        let mut manager = ShortcutManager::new(config);

        let shortcut = create_test_shortcut("c", vec!["cmd"], "copy");
        manager
            .add_shortcut(ShortcutCategory::Global, shortcut)
            .unwrap();

        assert_eq!(manager.get_shortcuts(ShortcutCategory::Global).len(), 1);
        assert_eq!(manager.get_statistics().global_count, 1);
    }

    #[test]
    fn test_remove_shortcut() {
        let shortcut = create_test_shortcut("c", vec!["cmd"], "copy");
        let config = ShortcutsConfig {
            global: vec![shortcut.clone()],
            terminal: vec![],
            custom: vec![],
        };
        let mut manager = ShortcutManager::new(config);

        let removed = manager
            .remove_shortcut(ShortcutCategory::Global, 0)
            .unwrap();
        assert_eq!(removed.key, "c");
        assert_eq!(manager.get_shortcuts(ShortcutCategory::Global).len(), 0);
    }

    #[test]
    fn test_update_shortcut() {
        let shortcut = create_test_shortcut("c", vec!["cmd"], "copy");
        let config = ShortcutsConfig {
            global: vec![shortcut],
            terminal: vec![],
            custom: vec![],
        };
        let mut manager = ShortcutManager::new(config);

        let new_shortcut = create_test_shortcut("v", vec!["cmd"], "paste");
        manager
            .update_shortcut(ShortcutCategory::Global, 0, new_shortcut)
            .unwrap();

        let updated = &manager.get_shortcuts(ShortcutCategory::Global)[0];
        assert_eq!(updated.key, "v");
    }

    #[test]
    fn test_find_shortcut() {
        let shortcut = create_test_shortcut("c", vec!["cmd"], "copy");
        let config = ShortcutsConfig {
            global: vec![shortcut],
            terminal: vec![],
            custom: vec![],
        };
        let manager = ShortcutManager::new(config);

        let found = manager.find_shortcut("c", &["cmd".to_string()]);
        assert!(found.is_some());

        let (category, index, _) = found.unwrap();
        assert_eq!(category, ShortcutCategory::Global);
        assert_eq!(index, 0);
    }

    #[test]
    fn test_duplicate_shortcut_prevention() {
        let shortcut = create_test_shortcut("c", vec!["cmd"], "copy");
        let config = ShortcutsConfig {
            global: vec![shortcut.clone()],
            terminal: vec![],
            custom: vec![],
        };
        let mut manager = ShortcutManager::new(config);

        // 尝试添加相同的快捷键应该失败
        let result = manager.add_shortcut(ShortcutCategory::Global, shortcut);
        assert!(result.is_err());
    }
}
