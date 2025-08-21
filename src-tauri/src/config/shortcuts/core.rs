/*!
 * 快捷键系统核心
 * 
 * 负责：
 * - 快捷键配置管理
 * - 验证和冲突检测
 * - 与配置系统集成
 */

use super::actions::ActionRegistry;
use super::types::*;
use crate::config::{
    types::{ShortcutBinding, ShortcutsConfig},
    TomlConfigManager,
};
use crate::utils::error::AppResult;
use anyhow::bail;
use std::{
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// 快捷键管理器
/// 
/// 整个快捷键系统的核心组件，负责统一管理所有快捷键相关功能
pub struct ShortcutManager {
    /// 配置管理器
    config_manager: Arc<TomlConfigManager>,
    /// 动作注册表
    action_registry: Arc<RwLock<ActionRegistry>>,
    /// 缓存的配置
    cached_config: Arc<RwLock<Option<ShortcutsConfig>>>,
    /// 缓存的验证结果
    cached_validation: Arc<RwLock<Option<ValidationResult>>>,
    /// 缓存的冲突检测结果
    cached_conflicts: Arc<RwLock<Option<ConflictResult>>>,
}

impl ShortcutManager {
    /// 创建新的快捷键管理器
    pub async fn new(config_manager: Arc<TomlConfigManager>) -> AppResult<Self> {
        debug!("创建快捷键管理器");

        let action_registry = Arc::new(RwLock::new(ActionRegistry::new()));

        let manager = Self {
            config_manager,
            action_registry,
            cached_config: Arc::new(RwLock::new(None)),
            cached_validation: Arc::new(RwLock::new(None)),
            cached_conflicts: Arc::new(RwLock::new(None)),
        };

        // 初始化时加载配置
        manager.reload_config().await?;

        info!("快捷键管理器创建成功");
        Ok(manager)
    }

    /// 获取快捷键配置
    pub async fn get_config(&self) -> AppResult<ShortcutsConfig> {
        debug!("获取快捷键配置");

        // 先检查缓存
        {
            let cached = self.cached_config.read().await;
            if let Some(config) = cached.as_ref() {
                return Ok(config.clone());
            }
        }

        // 从配置管理器加载
        self.reload_config().await
    }

    /// 更新快捷键配置
    pub async fn update_config(&self, new_config: ShortcutsConfig) -> AppResult<()> {
        debug!("更新快捷键配置");

        // 验证新配置
        let validation_result = self.validate_config(&new_config).await?;
        if !validation_result.is_valid {
            let error_messages: Vec<String> = validation_result
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect();
            bail!("快捷键配置验证失败: {}", error_messages.join(", "));
        }

        // 检测冲突
        let conflict_result = self.detect_conflicts(&new_config).await?;
        if conflict_result.has_conflicts {
            warn!("发现 {} 个快捷键冲突", conflict_result.conflicts.len());
        }

        // 更新到配置文件
        self.config_manager
            .update_config(|config| {
                config.shortcuts = new_config.clone();
                Ok(())
            })
            .await?;

        // 更新缓存
        {
            let mut cached = self.cached_config.write().await;
            *cached = Some(new_config);
        }

        // 清除验证和冲突缓存
        self.clear_cache().await;

        info!("快捷键配置更新成功");
        Ok(())
    }

    /// 添加快捷键
    pub async fn add_shortcut(&self, category: ShortcutCategory, binding: ShortcutBinding) -> AppResult<()> {
        debug!("添加快捷键: {:?} 到类别 {:?}", binding, category);

        let mut config = self.get_config().await?;

        // 检查冲突
        let key_combo = KeyCombination::from_binding(&binding);
        if self.has_conflict_in_config(&config, &key_combo).await {
            bail!("快捷键 {} 已存在冲突", key_combo.to_string());
        }

        // 验证单个快捷键
        self.validate_single_binding(&binding).await?;

        // 添加到相应类别
        match category {
            ShortcutCategory::Global => config.global.push(binding),
            ShortcutCategory::Terminal => config.terminal.push(binding),
            ShortcutCategory::Custom => config.custom.push(binding),
        }

        // 更新配置
        self.update_config(config).await?;

        info!("快捷键添加成功");
        Ok(())
    }

    /// 删除快捷键
    pub async fn remove_shortcut(&self, category: ShortcutCategory, index: usize) -> AppResult<ShortcutBinding> {
        debug!("删除快捷键: 类别 {:?}, 索引 {}", category, index);

        let mut config = self.get_config().await?;

        let removed_binding = match category {
            ShortcutCategory::Global => {
                if index >= config.global.len() {
                    bail!("全局快捷键索引超出范围: {}", index);
                }
                config.global.remove(index)
            }
            ShortcutCategory::Terminal => {
                if index >= config.terminal.len() {
                    bail!("终端快捷键索引超出范围: {}", index);
                }
                config.terminal.remove(index)
            }
            ShortcutCategory::Custom => {
                if index >= config.custom.len() {
                    bail!("自定义快捷键索引超出范围: {}", index);
                }
                config.custom.remove(index)
            }
        };

        // 更新配置
        self.update_config(config).await?;

        info!("快捷键删除成功");
        Ok(removed_binding)
    }

    /// 更新指定快捷键
    pub async fn update_shortcut(
        &self,
        category: ShortcutCategory,
        index: usize,
        new_binding: ShortcutBinding,
    ) -> AppResult<()> {
        debug!("更新快捷键: 类别 {:?}, 索引 {}, 新绑定 {:?}", category, index, new_binding);

        let mut config = self.get_config().await?;

        // 验证新绑定
        self.validate_single_binding(&new_binding).await?;

        // 更新对应位置的快捷键
        match category {
            ShortcutCategory::Global => {
                if index >= config.global.len() {
                    bail!("全局快捷键索引超出范围: {}", index);
                }
                config.global[index] = new_binding;
            }
            ShortcutCategory::Terminal => {
                if index >= config.terminal.len() {
                    bail!("终端快捷键索引超出范围: {}", index);
                }
                config.terminal[index] = new_binding;
            }
            ShortcutCategory::Custom => {
                if index >= config.custom.len() {
                    bail!("自定义快捷键索引超出范围: {}", index);
                }
                config.custom[index] = new_binding;
            }
        }

        // 更新配置
        self.update_config(config).await?;

        info!("快捷键更新成功");
        Ok(())
    }

    /// 重置到默认配置
    pub async fn reset_to_defaults(&self) -> AppResult<()> {
        debug!("重置快捷键配置到默认值");

        let default_config = crate::config::defaults::create_default_shortcuts_config();
        self.update_config(default_config).await?;

        info!("快捷键配置重置成功");
        Ok(())
    }

    /// 验证快捷键配置
    pub async fn validate_config(&self, config: &ShortcutsConfig) -> AppResult<ValidationResult> {
        debug!("验证快捷键配置");

        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 验证各类别的快捷键
        let categories = [
            (ShortcutCategory::Global, &config.global),
            (ShortcutCategory::Terminal, &config.terminal),
            (ShortcutCategory::Custom, &config.custom),
        ];

        for (category, bindings) in categories {
            for (index, binding) in bindings.iter().enumerate() {
                if let Err(e) = self.validate_single_binding(binding).await {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::InvalidAction,
                        message: format!("{}类别第{}个快捷键无效: {}", category, index + 1, e),
                        key_combination: Some(KeyCombination::from_binding(binding)),
                    });
                }

                // 检查动作是否已注册
                let action_name = self.extract_action_name(&binding.action);
                let registry = self.action_registry.read().await;
                if !registry.is_action_registered(&action_name).await {
                    warnings.push(ValidationWarning {
                        warning_type: ValidationWarningType::UnregisteredAction,
                        message: format!("动作 '{}' 未注册", action_name),
                        key_combination: Some(KeyCombination::from_binding(binding)),
                    });
                }
            }
        }

        let result = ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        };

        // 更新缓存
        {
            let mut cached = self.cached_validation.write().await;
            *cached = Some(result.clone());
        }

        Ok(result)
    }

    /// 检测快捷键冲突
    pub async fn detect_conflicts(&self, config: &ShortcutsConfig) -> AppResult<ConflictResult> {
        debug!("检测快捷键冲突");

        let mut key_map: HashMap<String, Vec<ConflictingBinding>> = HashMap::new();

        // 收集所有快捷键组合
        let categories = [
            (ShortcutCategory::Global, &config.global),
            (ShortcutCategory::Terminal, &config.terminal),
            (ShortcutCategory::Custom, &config.custom),
        ];

        for (category, bindings) in categories {
            for (index, binding) in bindings.iter().enumerate() {
                let key_combo = KeyCombination::from_binding(binding);
                let key_str = key_combo.to_string();
                let action_name = self.extract_action_name(&binding.action);

                let conflicting_binding = ConflictingBinding {
                    category: category.clone(),
                    action: action_name,
                    index,
                };

                key_map
                    .entry(key_str)
                    .or_insert_with(Vec::new)
                    .push(conflicting_binding);
            }
        }

        // 查找冲突
        let conflicts: Vec<ConflictInfo> = key_map
            .into_iter()
            .filter_map(|(key_str, bindings)| {
                if bindings.len() > 1 {
                    Some(ConflictInfo {
                        key_combination: KeyCombination::new(
                            key_str.split('+').last().unwrap_or("").to_string(),
                            key_str.split('+').take(key_str.split('+').count() - 1).map(|s| s.to_string()).collect(),
                        ),
                        conflicting_bindings: bindings,
                    })
                } else {
                    None
                }
            })
            .collect();

        let result = ConflictResult {
            has_conflicts: !conflicts.is_empty(),
            conflicts,
        };

        // 更新缓存
        {
            let mut cached = self.cached_conflicts.write().await;
            *cached = Some(result.clone());
        }

        Ok(result)
    }

    /// 获取快捷键统计信息
    pub async fn get_statistics(&self) -> AppResult<ShortcutStatistics> {
        debug!("获取快捷键统计信息");

        let config = self.get_config().await?;

        let mut category_counts = HashMap::new();
        category_counts.insert(ShortcutCategory::Global, config.global.len());
        category_counts.insert(ShortcutCategory::Terminal, config.terminal.len());
        category_counts.insert(ShortcutCategory::Custom, config.custom.len());

        let total_count = config.global.len() + config.terminal.len() + config.custom.len();

        // 统计最常用的修饰键
        let mut modifier_counts: HashMap<String, usize> = HashMap::new();
        for binding in config.global.iter().chain(config.terminal.iter()).chain(config.custom.iter()) {
            for modifier in &binding.modifiers {
                *modifier_counts.entry(modifier.clone()).or_insert(0) += 1;
            }
        }

        let mut popular_modifiers: Vec<(String, usize)> = modifier_counts.into_iter().collect();
        popular_modifiers.sort_by(|a, b| b.1.cmp(&a.1));
        let popular_modifiers: Vec<String> = popular_modifiers.into_iter().take(5).map(|(k, _)| k).collect();

        // 获取冲突数量
        let conflict_result = self.detect_conflicts(&config).await?;
        let conflict_count = conflict_result.conflicts.len();

        Ok(ShortcutStatistics {
            category_counts,
            total_count,
            conflict_count,
            popular_modifiers,
        })
    }

    /// 搜索快捷键
    pub async fn search_shortcuts(&self, options: SearchOptions) -> AppResult<SearchResult> {
        debug!("搜索快捷键: {:?}", options);

        let config = self.get_config().await?;
        let mut matches = Vec::new();

        let categories = [
            (ShortcutCategory::Global, &config.global),
            (ShortcutCategory::Terminal, &config.terminal),
            (ShortcutCategory::Custom, &config.custom),
        ];

        for (category, bindings) in categories {
            // 如果指定了类别过滤，检查是否匹配
            if let Some(ref filter_categories) = options.categories {
                if !filter_categories.contains(&category) {
                    continue;
                }
            }

            for (index, binding) in bindings.iter().enumerate() {
                let mut score = 0.0f32;
                let mut matches_criteria = true;

                // 检查按键匹配
                if let Some(ref key) = options.key {
                    if binding.key.to_lowercase().contains(&key.to_lowercase()) {
                        score += 0.3;
                    } else {
                        matches_criteria = false;
                    }
                }

                // 检查修饰键匹配
                if let Some(ref modifiers) = options.modifiers {
                    let matching_modifiers = modifiers.iter().filter(|m| binding.modifiers.contains(m)).count();
                    if matching_modifiers > 0 {
                        score += 0.2 * (matching_modifiers as f32 / modifiers.len() as f32);
                    } else if !modifiers.is_empty() {
                        matches_criteria = false;
                    }
                }

                // 检查动作匹配
                if let Some(ref action) = options.action {
                    let action_name = self.extract_action_name(&binding.action);
                    if action_name.to_lowercase().contains(&action.to_lowercase()) {
                        score += 0.3;
                    } else {
                        matches_criteria = false;
                    }
                }

                // 检查查询字符串匹配
                if let Some(ref query) = options.query {
                    let query_lower = query.to_lowercase();
                    let action_name = self.extract_action_name(&binding.action);
                    
                    if binding.key.to_lowercase().contains(&query_lower) ||
                       binding.modifiers.iter().any(|m| m.to_lowercase().contains(&query_lower)) ||
                       action_name.to_lowercase().contains(&query_lower) {
                        score += 0.2;
                    } else {
                        matches_criteria = false;
                    }
                }

                // 如果没有任何过滤条件，给所有结果一个基础分数
                if options.query.is_none() && options.key.is_none() && 
                   options.modifiers.is_none() && options.action.is_none() {
                    score = 1.0;
                    matches_criteria = true;
                }

                if matches_criteria {
                    matches.push(SearchMatch {
                        category: category.clone(),
                        index,
                        binding: binding.clone(),
                        score: score.max(0.1), // 确保至少有一个最小分数
                    });
                }
            }
        }

        // 按分数排序
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        let total = matches.len();

        Ok(SearchResult { matches, total })
    }

    /// 执行动作
    pub async fn execute_action(&self, action: &crate::config::types::ShortcutAction, context: &ActionContext) -> OperationResult<serde_json::Value> {
        debug!("执行快捷键动作");

        let registry = self.action_registry.read().await;
        registry.execute_action(action, context).await
    }

    /// 获取动作注册表
    pub async fn get_action_registry(&self) -> Arc<RwLock<ActionRegistry>> {
        Arc::clone(&self.action_registry)
    }

    // ========================================================================
    // 私有方法
    // ========================================================================

    /// 重新加载配置
    async fn reload_config(&self) -> AppResult<ShortcutsConfig> {
        let config = self.config_manager.get_config().await?;
        let shortcuts_config = config.shortcuts;

        // 更新缓存
        {
            let mut cached = self.cached_config.write().await;
            *cached = Some(shortcuts_config.clone());
        }

        Ok(shortcuts_config)
    }

    /// 验证单个快捷键绑定
    async fn validate_single_binding(&self, binding: &ShortcutBinding) -> AppResult<()> {
        // 验证按键
        if binding.key.is_empty() {
            bail!("按键不能为空");
        }

        // 验证修饰键
        let valid_modifiers = ["ctrl", "alt", "shift", "cmd", "meta", "super"];
        for modifier in &binding.modifiers {
            if !valid_modifiers.contains(&modifier.to_lowercase().as_str()) {
                bail!("无效的修饰键: {}", modifier);
            }
        }

        // 验证动作
        let action_name = self.extract_action_name(&binding.action);
        if action_name.is_empty() {
            bail!("动作不能为空");
        }

        Ok(())
    }

    /// 检查配置中是否存在按键冲突
    async fn has_conflict_in_config(&self, config: &ShortcutsConfig, key_combo: &KeyCombination) -> bool {
        let all_bindings = config.global.iter()
            .chain(config.terminal.iter())
            .chain(config.custom.iter());

        for binding in all_bindings {
            let existing_combo = KeyCombination::from_binding(binding);
            if existing_combo == *key_combo {
                return true;
            }
        }

        false
    }

    /// 提取动作名称
    fn extract_action_name(&self, action: &crate::config::types::ShortcutAction) -> String {
        match action {
            crate::config::types::ShortcutAction::Simple(name) => name.clone(),
            crate::config::types::ShortcutAction::Complex { action_type, .. } => action_type.clone(),
        }
    }

    /// 清除缓存
    async fn clear_cache(&self) {
        {
            let mut cached = self.cached_validation.write().await;
            *cached = None;
        }
        {
            let mut cached = self.cached_conflicts.write().await;
            *cached = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 注意：实际测试需要创建mock的TomlConfigManager
    // 这里只提供测试框架

    #[tokio::test]
    async fn test_key_combination_equality() {
        let combo1 = KeyCombination::new("c".to_string(), vec!["cmd".to_string(), "shift".to_string()]);
        let combo2 = KeyCombination::new("c".to_string(), vec!["shift".to_string(), "cmd".to_string()]);
        
        // 因为我们在构造函数中对修饰键进行了排序，所以这两个应该相等
        assert_eq!(combo1, combo2);
    }

    #[test]
    fn test_shortcut_category_display() {
        assert_eq!(ShortcutCategory::Global.to_string(), "global");
        assert_eq!(ShortcutCategory::Terminal.to_string(), "terminal");
        assert_eq!(ShortcutCategory::Custom.to_string(), "custom");
    }
}