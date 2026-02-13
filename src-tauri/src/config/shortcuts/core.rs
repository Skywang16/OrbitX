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
    error::{ShortcutsError, ShortcutsResult},
    types::{ShortcutBinding, ShortcutsConfig},
    TomlConfigManager,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::warn;

pub struct ShortcutManager {
    config_manager: Arc<TomlConfigManager>,
    action_registry: Arc<RwLock<ActionRegistry>>,
    cached_config: Arc<RwLock<Option<ShortcutsConfig>>>,
    cached_validation: Arc<RwLock<Option<ValidationResult>>>,
    cached_conflicts: Arc<RwLock<Option<ConflictResult>>>,
}

impl ShortcutManager {
    pub async fn new(config_manager: Arc<TomlConfigManager>) -> ShortcutsResult<Self> {
        let action_registry = Arc::new(RwLock::new(ActionRegistry::new()));

        let manager = Self {
            config_manager,
            action_registry,
            cached_config: Arc::new(RwLock::new(None)),
            cached_validation: Arc::new(RwLock::new(None)),
            cached_conflicts: Arc::new(RwLock::new(None)),
        };

        manager.reload_config().await?;

        Ok(manager)
    }

    pub async fn config_get(&self) -> ShortcutsResult<ShortcutsConfig> {
        {
            let cached = self.cached_config.read().await;
            if let Some(config) = cached.as_ref() {
                return Ok(config.clone());
            }
        }
        self.reload_config().await
    }

    pub async fn config_update(&self, new_config: ShortcutsConfig) -> ShortcutsResult<()> {
        let validation_result = self.config_validate(&new_config).await?;
        if !validation_result.is_valid {
            let error_messages: Vec<String> = validation_result
                .errors
                .iter()
                .map(|e| e.message.clone())
                .collect();
            let reason = format!(
                "Shortcut configuration validation failed: {}",
                error_messages.join(", ")
            );
            return Err(ShortcutsError::Validation { reason });
        }

        let conflict_result = self.detect_conflicts(&new_config).await?;
        if conflict_result.has_conflicts {
            warn!(
                "Detected {} shortcut conflicts",
                conflict_result.conflicts.len()
            );
        }

        self.config_manager
            .config_update(|config| {
                config.shortcuts = new_config.clone();
                Ok(())
            })
            .await?;

        {
            let mut cached = self.cached_config.write().await;
            *cached = Some(new_config);
        }

        self.clear_cache().await;

        Ok(())
    }

    pub async fn shortcuts_add(&self, binding: ShortcutBinding) -> ShortcutsResult<()> {
        let mut config = self.config_get().await?;

        let key_combo = KeyCombination::from_binding(&binding);
        if self.has_conflict_in_config(&config, &key_combo).await {
            let detail = format!("Shortcut {} already conflicts", key_combo.to_string());
            return Err(ShortcutsError::Conflict { detail });
        }

        self.validate_single_binding(&binding).await?;
        config.push(binding);
        self.config_update(config).await?;

        Ok(())
    }

    pub async fn shortcuts_remove(&self, index: usize) -> ShortcutsResult<ShortcutBinding> {
        let mut config = self.config_get().await?;

        if index >= config.len() {
            let reason = format!("Shortcut index out of bounds: {}", index);
            return Err(ShortcutsError::Validation { reason });
        }

        let removed_binding = config.remove(index);
        self.config_update(config).await?;

        Ok(removed_binding)
    }

    pub async fn shortcuts_update(
        &self,
        index: usize,
        new_binding: ShortcutBinding,
    ) -> ShortcutsResult<()> {
        let mut config = self.config_get().await?;
        self.validate_single_binding(&new_binding).await?;

        if index >= config.len() {
            let reason = format!("Shortcut index out of bounds: {}", index);
            return Err(ShortcutsError::Validation { reason });
        }

        config[index] = new_binding;
        self.config_update(config).await?;

        Ok(())
    }

    pub async fn reset_to_defaults(&self) -> ShortcutsResult<()> {
        let default_config = crate::config::defaults::create_default_shortcuts_config();
        self.config_update(default_config).await?;

        Ok(())
    }

    pub async fn config_validate(
        &self,
        config: &ShortcutsConfig,
    ) -> ShortcutsResult<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        for (index, binding) in config.iter().enumerate() {
            if let Err(e) = self.validate_single_binding(binding).await {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::InvalidAction,
                    message: format!("Shortcut {} is invalid: {}", index + 1, e),
                    key_combination: Some(KeyCombination::from_binding(binding)),
                });
            }

            let action_name = self.extract_action_name(&binding.action);
            let registry = self.action_registry.read().await;
            if !registry.is_action_registered(&action_name).await {
                warnings.push(ValidationWarning {
                    warning_type: ValidationWarningType::UnregisteredAction,
                    message: format!("Action '{}' is not registered", action_name),
                    key_combination: Some(KeyCombination::from_binding(binding)),
                });
            }
        }

        let result = ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        };

        {
            let mut cached = self.cached_validation.write().await;
            *cached = Some(result.clone());
        }

        Ok(result)
    }

    pub async fn detect_conflicts(
        &self,
        config: &ShortcutsConfig,
    ) -> ShortcutsResult<ConflictResult> {
        let mut key_map: HashMap<String, Vec<ConflictingBinding>> = HashMap::new();

        for (index, binding) in config.iter().enumerate() {
            let key_combo = KeyCombination::from_binding(binding);
            let key_str = key_combo.to_string();
            let action_name = self.extract_action_name(&binding.action);

            let conflicting_binding = ConflictingBinding {
                action: action_name,
                index,
            };

            key_map
                .entry(key_str)
                .or_insert_with(Vec::new)
                .push(conflicting_binding);
        }

        let conflicts: Vec<ConflictInfo> = key_map
            .into_iter()
            .filter_map(|(key_str, bindings)| {
                if bindings.len() > 1 {
                    Some(ConflictInfo {
                        key_combination: KeyCombination::new(
                            key_str.split('+').last().unwrap_or("").to_string(),
                            key_str
                                .split('+')
                                .take(key_str.split('+').count() - 1)
                                .map(|s| s.to_string())
                                .collect(),
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

        {
            let mut cached = self.cached_conflicts.write().await;
            *cached = Some(result.clone());
        }

        Ok(result)
    }

    pub async fn get_statistics(&self) -> ShortcutsResult<ShortcutStatistics> {
        let config = self.config_get().await?;
        let total_count = config.len();
        let mut modifier_counts: HashMap<String, usize> = HashMap::new();
        for binding in config.iter() {
            for modifier in &binding.modifiers {
                *modifier_counts.entry(modifier.clone()).or_insert(0) += 1;
            }
        }

        let mut popular_modifiers: Vec<(String, usize)> = modifier_counts.into_iter().collect();
        popular_modifiers.sort_by(|a, b| b.1.cmp(&a.1));
        let popular_modifiers: Vec<String> = popular_modifiers
            .into_iter()
            .take(5)
            .map(|(k, _)| k)
            .collect();

        let conflict_result = self.detect_conflicts(&config).await?;
        let conflict_count = conflict_result.conflicts.len();

        Ok(ShortcutStatistics {
            total_count,
            conflict_count,
            popular_modifiers,
        })
    }

    pub async fn shortcuts_search(&self, options: SearchOptions) -> ShortcutsResult<SearchResult> {
        let config = self.config_get().await?;
        let mut matches = Vec::new();

        for (index, binding) in config.iter().enumerate() {
            let mut score = 0.0f32;
            let mut matches_criteria = true;

            if let Some(ref key) = options.key {
                if binding.key.to_lowercase().contains(&key.to_lowercase()) {
                    score += 0.3;
                } else {
                    matches_criteria = false;
                }
            }

            if let Some(ref modifiers) = options.modifiers {
                let matching_modifiers = modifiers
                    .iter()
                    .filter(|m| binding.modifiers.contains(m))
                    .count();
                if matching_modifiers > 0 {
                    score += 0.2 * (matching_modifiers as f32 / modifiers.len() as f32);
                } else if !modifiers.is_empty() {
                    matches_criteria = false;
                }
            }

            if let Some(ref action) = options.action {
                let action_name = self.extract_action_name(&binding.action);
                if action_name.to_lowercase().contains(&action.to_lowercase()) {
                    score += 0.3;
                } else {
                    matches_criteria = false;
                }
            }

            if let Some(ref query) = options.query {
                let query_lower = query.to_lowercase();
                let action_name = self.extract_action_name(&binding.action);

                if binding.key.to_lowercase().contains(&query_lower)
                    || binding
                        .modifiers
                        .iter()
                        .any(|m| m.to_lowercase().contains(&query_lower))
                    || action_name.to_lowercase().contains(&query_lower)
                {
                    score += 0.2;
                } else {
                    matches_criteria = false;
                }
            }

            if options.query.is_none()
                && options.key.is_none()
                && options.modifiers.is_none()
                && options.action.is_none()
            {
                score = 1.0;
                matches_criteria = true;
            }

            if matches_criteria {
                matches.push(SearchMatch {
                    index,
                    binding: binding.clone(),
                    score: score.max(0.1),
                });
            }
        }

        matches.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let total = matches.len();

        Ok(SearchResult { matches, total })
    }

    pub async fn execute_action(
        &self,
        action: &crate::config::types::ShortcutAction,
        context: &ActionContext,
    ) -> OperationResult<serde_json::Value> {
        let registry = self.action_registry.read().await;
        registry.execute_action(action, context).await
    }

    pub async fn get_action_registry(&self) -> Arc<RwLock<ActionRegistry>> {
        Arc::clone(&self.action_registry)
    }

    // 私有方法

    async fn reload_config(&self) -> ShortcutsResult<ShortcutsConfig> {
        let config = self.config_manager.config_get().await?;
        let shortcuts_config = config.shortcuts;

        {
            let mut cached = self.cached_config.write().await;
            *cached = Some(shortcuts_config.clone());
        }

        Ok(shortcuts_config)
    }

    async fn validate_single_binding(&self, binding: &ShortcutBinding) -> ShortcutsResult<()> {
        if binding.key.trim().is_empty() {
            return Err(ShortcutsError::Validation {
                reason: "Shortcut key cannot be empty".to_string(),
            });
        }

        let valid_modifiers = ["ctrl", "alt", "shift", "cmd", "meta", "super"];
        for modifier in &binding.modifiers {
            if !valid_modifiers.contains(&modifier.to_lowercase().as_str()) {
                return Err(ShortcutsError::Validation {
                    reason: format!("Unsupported modifier: {}", modifier),
                });
            }
        }

        let action_name = self.extract_action_name(&binding.action);
        if action_name.is_empty() {
            return Err(ShortcutsError::Validation {
                reason: "Action name cannot be empty".to_string(),
            });
        }

        Ok(())
    }

    async fn has_conflict_in_config(
        &self,
        config: &ShortcutsConfig,
        key_combo: &KeyCombination,
    ) -> bool {
        let all_bindings = config.iter();

        for binding in all_bindings {
            let existing_combo = KeyCombination::from_binding(binding);
            if existing_combo == *key_combo {
                return true;
            }
        }

        false
    }

    fn extract_action_name(&self, action: &crate::config::types::ShortcutAction) -> String {
        match action {
            crate::config::types::ShortcutAction::Simple(name) => name.clone(),
            crate::config::types::ShortcutAction::Complex { action_type, .. } => {
                action_type.clone()
            }
        }
    }

    async fn clear_cache(&self) {
        {
            let mut cached = self.cached_config.write().await;
            *cached = None;
        }
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

    // 测试需要创建mock的TomlConfigManager

    #[tokio::test]
    async fn test_key_combination_equality() {
        let combo1 = KeyCombination::new(
            "c".to_string(),
            vec!["cmd".to_string(), "shift".to_string()],
        );
        let combo2 = KeyCombination::new(
            "c".to_string(),
            vec!["shift".to_string(), "cmd".to_string()],
        );

        assert_eq!(combo1, combo2);
    }
}
