/*!
 * 快捷键服务层
 *
 * 提供快捷键系统的核心业务逻辑，包括配置管理、验证、冲突检测等功能。
 * 采用配置驱动的设计模式，与现有TOML配置系统无缝集成。
 */

use crate::config::{
    types::{ShortcutAction, ShortcutBinding, ShortcutsConfig},
    TomlConfigManager,
};
use crate::utils::error::AppResult;
use anyhow::bail;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tracing::{debug, info, warn};

/// 快捷键类别枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShortcutCategory {
    Global,
    Terminal,
    Custom,
}

/// 快捷键验证错误类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutValidationError {
    pub error_type: String,
    pub message: String,
    pub shortcut: Option<ShortcutBinding>,
}

/// 快捷键验证警告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutValidationWarning {
    pub warning_type: String,
    pub message: String,
    pub shortcut: Option<ShortcutBinding>,
}

/// 快捷键验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ShortcutValidationError>,
    pub warnings: Vec<ShortcutValidationWarning>,
}

/// 冲突的快捷键信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictingShortcut {
    pub category: String,
    pub binding: ShortcutBinding,
}

/// 快捷键冲突
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutConflict {
    pub key_combination: String,
    pub conflicting_shortcuts: Vec<ConflictingShortcut>,
}

/// 快捷键冲突检测结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetectionResult {
    pub has_conflicts: bool,
    pub conflicts: Vec<ShortcutConflict>,
}

/// 快捷键统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutStatistics {
    pub global_count: usize,
    pub terminal_count: usize,
    pub custom_count: usize,
    pub total_count: usize,
}

/// 快捷键服务
///
/// 负责快捷键系统的核心逻辑，包括配置管理、验证、冲突检测等。
pub struct ShortcutService {
    /// 配置管理器
    config_manager: Arc<TomlConfigManager>,
    /// 功能注册表
    action_registry: ActionRegistry,
}

impl ShortcutService {
    /// 创建新的快捷键服务
    pub fn new(config_manager: Arc<TomlConfigManager>) -> Self {
        Self {
            config_manager,
            action_registry: ActionRegistry::new(),
        }
    }

    /// 获取快捷键配置
    pub async fn get_shortcuts_config(&self) -> AppResult<ShortcutsConfig> {
        debug!("获取快捷键配置");
        let config = self.config_manager.get_config().await?;
        Ok(config.shortcuts)
    }

    /// 更新快捷键配置
    pub async fn update_shortcuts_config(&self, shortcuts: ShortcutsConfig) -> AppResult<()> {
        debug!("更新快捷键配置");

        // 验证新配置
        let validation_result = self.validate_shortcuts_config(&shortcuts)?;
        if !validation_result.is_valid {
            let error_messages: Vec<String> = validation_result
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect();
            bail!("快捷键配置验证失败: {}", error_messages.join(", "));
        }

        // 检测冲突
        let conflict_result = self.detect_shortcuts_conflicts(&shortcuts)?;
        if conflict_result.has_conflicts {
            warn!("发现快捷键冲突: {}", conflict_result.conflicts.len());
        }

        // 更新配置
        self.config_manager
            .update_config(|config| {
                config.shortcuts = shortcuts;
                Ok(())
            })
            .await?;

        info!("快捷键配置更新成功");
        Ok(())
    }

    /// 添加快捷键
    pub async fn add_shortcut(
        &self,
        category: ShortcutCategory,
        shortcut: ShortcutBinding,
    ) -> AppResult<()> {
        debug!("添加快捷键: {:?} 到类别 {:?}", shortcut, category);

        // 验证单个快捷键
        self.validate_single_shortcut(&shortcut)?;

        // 获取当前配置
        let mut config = self.config_manager.get_config().await?;

        // 检查是否存在冲突
        let key_combination = self.format_key_combination(&shortcut);
        if self.has_key_conflict(&config.shortcuts, &key_combination, &category) {
            bail!("快捷键 {} 已存在冲突", key_combination);
        }

        // 添加到对应类别
        match category {
            ShortcutCategory::Global => config.shortcuts.global.push(shortcut),
            ShortcutCategory::Terminal => config.shortcuts.terminal.push(shortcut),
            ShortcutCategory::Custom => config.shortcuts.custom.push(shortcut),
        }

        // 更新配置
        self.config_manager
            .update_config(|cfg| {
                cfg.shortcuts = config.shortcuts;
                Ok(())
            })
            .await?;

        info!("快捷键添加成功");
        Ok(())
    }

    /// 删除快捷键
    pub async fn remove_shortcut(
        &self,
        category: ShortcutCategory,
        index: usize,
    ) -> AppResult<ShortcutBinding> {
        debug!("删除快捷键: 类别 {:?}, 索引 {}", category, index);

        let mut config = self.config_manager.get_config().await?;

        let removed_shortcut = match category {
            ShortcutCategory::Global => {
                if index >= config.shortcuts.global.len() {
                    bail!("全局快捷键索引超出范围: {}", index);
                }
                config.shortcuts.global.remove(index)
            }
            ShortcutCategory::Terminal => {
                if index >= config.shortcuts.terminal.len() {
                    bail!("终端快捷键索引超出范围: {}", index);
                }
                config.shortcuts.terminal.remove(index)
            }
            ShortcutCategory::Custom => {
                if index >= config.shortcuts.custom.len() {
                    bail!("自定义快捷键索引超出范围: {}", index);
                }
                config.shortcuts.custom.remove(index)
            }
        };

        // 更新配置
        self.config_manager
            .update_config(|cfg| {
                cfg.shortcuts = config.shortcuts;
                Ok(())
            })
            .await?;

        info!("快捷键删除成功");
        Ok(removed_shortcut)
    }

    /// 更新快捷键
    pub async fn update_shortcut(
        &self,
        category: ShortcutCategory,
        index: usize,
        shortcut: ShortcutBinding,
    ) -> AppResult<()> {
        debug!(
            "更新快捷键: 类别 {:?}, 索引 {}, 新快捷键 {:?}",
            category, index, shortcut
        );

        // 验证新快捷键
        self.validate_single_shortcut(&shortcut)?;

        let mut config = self.config_manager.get_config().await?;

        // 更新对应类别的快捷键
        match category {
            ShortcutCategory::Global => {
                if index >= config.shortcuts.global.len() {
                    bail!("全局快捷键索引超出范围: {}", index);
                }
                config.shortcuts.global[index] = shortcut;
            }
            ShortcutCategory::Terminal => {
                if index >= config.shortcuts.terminal.len() {
                    bail!("终端快捷键索引超出范围: {}", index);
                }
                config.shortcuts.terminal[index] = shortcut;
            }
            ShortcutCategory::Custom => {
                if index >= config.shortcuts.custom.len() {
                    bail!("自定义快捷键索引超出范围: {}", index);
                }
                config.shortcuts.custom[index] = shortcut;
            }
        }

        // 检查更新后是否有冲突
        let conflict_result = self.detect_shortcuts_conflicts(&config.shortcuts)?;
        if conflict_result.has_conflicts {
            warn!("更新后发现快捷键冲突");
        }

        // 更新配置
        self.config_manager
            .update_config(|cfg| {
                cfg.shortcuts = config.shortcuts;
                Ok(())
            })
            .await?;

        info!("快捷键更新成功");
        Ok(())
    }

    /// 重置快捷键到默认配置
    pub async fn reset_shortcuts_to_defaults(&self) -> AppResult<()> {
        debug!("重置快捷键到默认配置");

        let default_shortcuts = crate::config::defaults::create_default_shortcuts_config();

        self.config_manager
            .update_config(|config| {
                config.shortcuts = default_shortcuts;
                Ok(())
            })
            .await?;

        info!("快捷键重置成功");
        Ok(())
    }

    /// 验证快捷键配置
    pub fn validate_shortcuts_config(
        &self,
        shortcuts: &ShortcutsConfig,
    ) -> AppResult<ShortcutValidationResult> {
        debug!("验证快捷键配置");

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 验证全局快捷键
        for (index, shortcut) in shortcuts.global.iter().enumerate() {
            if let Err(e) = self.validate_single_shortcut(shortcut) {
                errors.push(ShortcutValidationError {
                    error_type: "invalid_global_shortcut".to_string(),
                    message: format!("全局快捷键 {} 无效: {}", index, e),
                    shortcut: Some(shortcut.clone()),
                });
            }
        }

        // 验证终端快捷键
        for (index, shortcut) in shortcuts.terminal.iter().enumerate() {
            if let Err(e) = self.validate_single_shortcut(shortcut) {
                errors.push(ShortcutValidationError {
                    error_type: "invalid_terminal_shortcut".to_string(),
                    message: format!("终端快捷键 {} 无效: {}", index, e),
                    shortcut: Some(shortcut.clone()),
                });
            }
        }

        // 验证自定义快捷键
        for (index, shortcut) in shortcuts.custom.iter().enumerate() {
            if let Err(e) = self.validate_single_shortcut(shortcut) {
                errors.push(ShortcutValidationError {
                    error_type: "invalid_custom_shortcut".to_string(),
                    message: format!("自定义快捷键 {} 无效: {}", index, e),
                    shortcut: Some(shortcut.clone()),
                });
            }
        }

        // 检查是否有未注册的动作
        let all_shortcuts = shortcuts
            .global
            .iter()
            .chain(shortcuts.terminal.iter())
            .chain(shortcuts.custom.iter());

        for shortcut in all_shortcuts {
            let action_name = self.extract_action_name(&shortcut.action);
            if !self.action_registry.is_action_registered(&action_name) {
                warnings.push(ShortcutValidationWarning {
                    warning_type: "unregistered_action".to_string(),
                    message: format!("动作 '{}' 未注册", action_name),
                    shortcut: Some(shortcut.clone()),
                });
            }
        }

        let is_valid = errors.is_empty();

        Ok(ShortcutValidationResult {
            is_valid,
            errors,
            warnings,
        })
    }

    /// 检测快捷键冲突
    pub fn detect_shortcuts_conflicts(
        &self,
        shortcuts: &ShortcutsConfig,
    ) -> AppResult<ConflictDetectionResult> {
        debug!("检测快捷键冲突");

        let mut key_map: HashMap<String, Vec<ConflictingShortcut>> = HashMap::new();

        // 收集所有快捷键
        let categories = [
            ("global", &shortcuts.global),
            ("terminal", &shortcuts.terminal),
            ("custom", &shortcuts.custom),
        ];

        for (category_name, shortcuts_list) in categories {
            for shortcut in shortcuts_list {
                let key_combination = self.format_key_combination(shortcut);
                let conflicting_shortcut = ConflictingShortcut {
                    category: category_name.to_string(),
                    binding: shortcut.clone(),
                };

                key_map
                    .entry(key_combination)
                    .or_insert_with(Vec::new)
                    .push(conflicting_shortcut);
            }
        }

        // 查找冲突
        let mut conflicts = Vec::new();
        for (key_combination, conflicting_shortcuts) in key_map {
            if conflicting_shortcuts.len() > 1 {
                conflicts.push(ShortcutConflict {
                    key_combination,
                    conflicting_shortcuts,
                });
            }
        }

        let has_conflicts = !conflicts.is_empty();

        Ok(ConflictDetectionResult {
            has_conflicts,
            conflicts,
        })
    }

    /// 获取快捷键统计信息
    pub async fn get_shortcuts_statistics(&self) -> AppResult<ShortcutStatistics> {
        debug!("获取快捷键统计信息");

        let config = self.config_manager.get_config().await?;
        let shortcuts = &config.shortcuts;

        let global_count = shortcuts.global.len();
        let terminal_count = shortcuts.terminal.len();
        let custom_count = shortcuts.custom.len();
        let total_count = global_count + terminal_count + custom_count;

        Ok(ShortcutStatistics {
            global_count,
            terminal_count,
            custom_count,
            total_count,
        })
    }

    // ========================================================================
    // 私有方法
    // ========================================================================

    /// 验证单个快捷键
    fn validate_single_shortcut(&self, shortcut: &ShortcutBinding) -> AppResult<()> {
        // 验证按键
        if shortcut.key.is_empty() {
            bail!("按键不能为空");
        }

        // 验证修饰键
        let valid_modifiers = ["ctrl", "alt", "shift", "cmd", "meta", "super"];
        for modifier in &shortcut.modifiers {
            if !valid_modifiers.contains(&modifier.to_lowercase().as_str()) {
                bail!("无效的修饰键: {}", modifier);
            }
        }

        // 验证动作
        let action_name = self.extract_action_name(&shortcut.action);
        if action_name.is_empty() {
            bail!("动作不能为空");
        }

        Ok(())
    }

    /// 格式化按键组合
    fn format_key_combination(&self, shortcut: &ShortcutBinding) -> String {
        let mut parts = shortcut.modifiers.clone();
        parts.push(shortcut.key.clone());
        parts.join("+")
    }

    /// 检查是否存在按键冲突
    fn has_key_conflict(
        &self,
        shortcuts: &ShortcutsConfig,
        key_combination: &str,
        category: &ShortcutCategory,
    ) -> bool {
        let shortcuts_to_check = match category {
            ShortcutCategory::Global => &shortcuts.global,
            ShortcutCategory::Terminal => &shortcuts.terminal,
            ShortcutCategory::Custom => &shortcuts.custom,
        };

        shortcuts_to_check
            .iter()
            .any(|s| self.format_key_combination(s) == key_combination)
    }

    /// 提取动作名称
    fn extract_action_name(&self, action: &ShortcutAction) -> String {
        match action {
            ShortcutAction::Simple(name) => name.clone(),
            ShortcutAction::Complex { action_type, .. } => action_type.clone(),
        }
    }
}

/// 功能注册表
///
/// 负责管理所有可用的快捷键动作。
pub struct ActionRegistry {
    /// 已注册的动作
    registered_actions: HashSet<String>,
}

impl ActionRegistry {
    /// 创建新的功能注册表
    pub fn new() -> Self {
        let mut registry = Self {
            registered_actions: HashSet::new(),
        };

        // 注册默认动作
        registry.register_default_actions();
        registry
    }

    /// 注册动作
    pub fn register_action(&mut self, action_name: &str) {
        self.registered_actions.insert(action_name.to_string());
    }

    /// 检查动作是否已注册
    pub fn is_action_registered(&self, action_name: &str) -> bool {
        self.registered_actions.contains(action_name)
    }

    /// 获取所有已注册的动作
    pub fn get_registered_actions(&self) -> Vec<String> {
        self.registered_actions.iter().cloned().collect()
    }

    /// 注册默认动作
    fn register_default_actions(&mut self) {
        // 全局动作
        self.register_action("copy_to_clipboard");
        self.register_action("paste_from_clipboard");
        self.register_action("terminal_search");
        self.register_action("open_settings");

        // 终端动作
        self.register_action("new_tab");
        self.register_action("close_tab");
        self.register_action("switch_to_tab_1");
        self.register_action("switch_to_tab_2");
        self.register_action("switch_to_tab_3");
        self.register_action("switch_to_tab_4");
        self.register_action("switch_to_tab_5");
        self.register_action("switch_to_last_tab");
        self.register_action("accept_completion");
        self.register_action("clear_terminal");
        self.register_action("increase_font_size");
        self.register_action("decrease_font_size");
        self.register_action("toggle_theme");
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::defaults::create_default_shortcuts_config;

    fn create_test_service() -> ShortcutService {
        // 这里应该创建一个测试用的配置管理器
        // 为了编译通过，先用一个占位符
        todo!("需要创建测试用的配置管理器")
    }

    #[test]
    fn test_action_registry() {
        let registry = ActionRegistry::new();

        assert!(registry.is_action_registered("copy_to_clipboard"));
        assert!(registry.is_action_registered("new_tab"));
        assert!(!registry.is_action_registered("nonexistent_action"));
    }

    #[test]
    fn test_validate_single_shortcut() {
        let service = create_test_service();

        // 有效的快捷键
        let valid_shortcut = ShortcutBinding {
            key: "c".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("copy_to_clipboard".to_string()),
        };
        assert!(service.validate_single_shortcut(&valid_shortcut).is_ok());

        // 无效的快捷键（空按键）
        let invalid_shortcut = ShortcutBinding {
            key: "".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("copy_to_clipboard".to_string()),
        };
        assert!(service.validate_single_shortcut(&invalid_shortcut).is_err());
    }

    #[test]
    fn test_format_key_combination() {
        let service = create_test_service();

        let shortcut = ShortcutBinding {
            key: "c".to_string(),
            modifiers: vec!["cmd".to_string(), "shift".to_string()],
            action: ShortcutAction::Simple("copy_to_clipboard".to_string()),
        };

        let combination = service.format_key_combination(&shortcut);
        assert_eq!(combination, "cmd+shift+c");
    }

    #[test]
    fn test_detect_conflicts() {
        let service = create_test_service();

        // 创建有冲突的配置
        let mut config = create_default_shortcuts_config();

        // 添加一个与现有快捷键冲突的快捷键
        config.custom.push(ShortcutBinding {
            key: "c".to_string(),
            modifiers: vec!["cmd".to_string()],
            action: ShortcutAction::Simple("some_other_action".to_string()),
        });

        let result = service.detect_shortcuts_conflicts(&config).unwrap();
        assert!(result.has_conflicts);
        assert!(!result.conflicts.is_empty());
    }
}
