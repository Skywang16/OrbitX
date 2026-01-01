/*!
 * 快捷键动作执行系统
 *
 * 负责：
 * - 动作注册和管理
 * - 动作执行调度
 * - 上下文传递
 */

use super::types::{ActionContext, OperationResult, ShortcutEvent, ShortcutEventType};
use crate::config::error::{ShortcutsActionError, ShortcutsActionResult, ShortcutsResult};
use crate::config::types::ShortcutAction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, warn};

pub type ActionHandler =
    Box<dyn Fn(&ActionContext) -> ShortcutsActionResult<serde_json::Value> + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMetadata {
    pub name: String,
    pub description: String,
    pub requires_terminal: bool,
    pub is_system_action: bool,
    pub supported_platforms: Vec<String>,
}

pub struct ActionRegistry {
    handlers: Arc<RwLock<HashMap<String, ActionHandler>>>,
    metadata: Arc<RwLock<HashMap<String, ActionMetadata>>>,
    event_listeners: Arc<RwLock<Vec<Box<dyn Fn(&ShortcutEvent) + Send + Sync>>>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        let registry = Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            event_listeners: Arc::new(RwLock::new(Vec::new())),
        };

        let mut registry_instance = registry.clone();
        tokio::spawn(async move {
            registry_instance.register_default_actions().await;
        });

        registry
    }

    pub async fn register_action<F>(
        &mut self,
        metadata: ActionMetadata,
        handler: F,
    ) -> ShortcutsResult<()>
    where
        F: Fn(&ActionContext) -> ShortcutsActionResult<serde_json::Value> + Send + Sync + 'static,
    {
        let action_name = metadata.name.clone();

        {
            let handlers = self.handlers.read().await;
            if handlers.contains_key(&action_name) {
                return Err(ShortcutsActionError::AlreadyRegistered {
                    action: action_name,
                }
                .into());
            }
        }

        {
            let mut meta_map = self.metadata.write().await;
            meta_map.insert(action_name.clone(), metadata);
        }

        {
            let mut handler_map = self.handlers.write().await;
            handler_map.insert(action_name, Box::new(handler));
        }
        Ok(())
    }

    pub async fn execute_action(
        &self,
        action: &ShortcutAction,
        context: &ActionContext,
    ) -> OperationResult<serde_json::Value> {
        let action_name = self.extract_action_name(action);

        self.emit_event(ShortcutEvent {
            event_type: ShortcutEventType::KeyPressed,
            key_combination: Some(context.key_combination.clone()),
            action: Some(action_name.clone()),
            data: HashMap::new(),
            timestamp: chrono::Utc::now(),
        })
        .await;

        let handler_exists = {
            let handlers = self.handlers.read().await;
            handlers.contains_key(&action_name)
        };

        if !handler_exists {
            let error_msg = format!("Action not registered: {}", action_name);
            warn!("{}", error_msg);

            self.emit_event(ShortcutEvent {
                event_type: ShortcutEventType::ActionFailed,
                key_combination: Some(context.key_combination.clone()),
                action: Some(action_name),
                data: HashMap::from([(
                    "error".to_string(),
                    serde_json::Value::String(error_msg.clone()),
                )]),
                timestamp: chrono::Utc::now(),
            })
            .await;

            return OperationResult::failure(error_msg);
        }

        let result = {
            let handlers = self.handlers.read().await;
            match handlers.get(&action_name) {
                Some(handler) => handler(context),
                None => Err(ShortcutsActionError::NotRegistered {
                    action: action_name.clone(),
                }),
            }
        };

        match result {
            Ok(value) => {
                self.emit_event(ShortcutEvent {
                    event_type: ShortcutEventType::ActionExecuted,
                    key_combination: Some(context.key_combination.clone()),
                    action: Some(action_name),
                    data: HashMap::from([("result".to_string(), value.clone())]),
                    timestamp: chrono::Utc::now(),
                })
                .await;

                OperationResult::success(value)
            }
            Err(err) => {
                let error_msg = format!("Action execution failed: {}", err);
                error!("{}", error_msg);

                self.emit_event(ShortcutEvent {
                    event_type: ShortcutEventType::ActionFailed,
                    key_combination: Some(context.key_combination.clone()),
                    action: Some(action_name),
                    data: HashMap::from([(
                        "error".to_string(),
                        serde_json::Value::String(error_msg.clone()),
                    )]),
                    timestamp: chrono::Utc::now(),
                })
                .await;

                OperationResult::failure(error_msg)
            }
        }
    }

    pub async fn is_action_registered(&self, action_name: &str) -> bool {
        let handlers = self.handlers.read().await;
        handlers.contains_key(action_name)
    }

    pub async fn shortcuts_get_action_metadata(&self, action_name: &str) -> Option<ActionMetadata> {
        let metadata = self.metadata.read().await;
        metadata.get(action_name).cloned()
    }

    pub async fn shortcuts_get_registered_actions(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }

    pub async fn add_event_listener<F>(&mut self, listener: F)
    where
        F: Fn(&ShortcutEvent) + Send + Sync + 'static,
    {
        let mut listeners = self.event_listeners.write().await;
        listeners.push(Box::new(listener));
    }

    async fn emit_event(&self, event: ShortcutEvent) {
        let listeners = self.event_listeners.read().await;
        for listener in listeners.iter() {
            listener(&event);
        }
    }

    fn extract_action_name(&self, action: &ShortcutAction) -> String {
        match action {
            ShortcutAction::Simple(name) => name.clone(),
            ShortcutAction::Complex { action_type, .. } => action_type.clone(),
        }
    }

    async fn register_default_actions(&mut self) {
        self.register_global_actions().await;
        self.register_terminal_actions().await;
        self.register_system_actions().await;
    }

    async fn register_global_actions(&mut self) {
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "copy_to_clipboard".to_string(),
                    description: "复制选中内容到剪贴板".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| Ok(serde_json::Value::String("🔥 复制功能已触发！".to_string())),
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "paste_from_clipboard".to_string(),
                    description: "从剪贴板粘贴内容".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| Ok(serde_json::Value::String("🔥 粘贴功能已触发！".to_string())),
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "terminal_search".to_string(),
                    description: "终端搜索".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| Ok(serde_json::Value::String("🔥 搜索功能已触发！".to_string())),
            )
            .await;
    }

    async fn register_terminal_actions(&mut self) {
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "new_tab".to_string(),
                    description: "新建终端标签页".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 新建标签页功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "close_tab".to_string(),
                    description: "关闭当前终端标签页".to_string(),
                    requires_terminal: true,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    if let Some(frontend_result) = context.metadata.get("frontendResult") {
                        if let Some(result_bool) = frontend_result.as_bool() {
                            if result_bool {
                                return Ok(serde_json::Value::String(
                                    "前端已处理关闭标签页".to_string(),
                                ));
                            }
                        }
                    }

                    Ok(serde_json::Value::String(
                        "🔥 关闭标签页功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "switch_to_tab_1".to_string(),
                    description: "切换到标签页1".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 切换到标签页1功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "switch_to_tab_2".to_string(),
                    description: "切换到标签页2".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 切换到标签页2功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "switch_to_tab_3".to_string(),
                    description: "切换到标签页3".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 切换到标签页3功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "switch_to_tab_4".to_string(),
                    description: "切换到标签页4".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 切换到标签页4功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "switch_to_tab_5".to_string(),
                    description: "切换到标签页5".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 切换到标签页5功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "switch_to_last_tab".to_string(),
                    description: "切换到最后一个标签页".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 切换到最后一个标签页功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "accept_completion".to_string(),
                    description: "接受当前补全建议".to_string(),

                    requires_terminal: true,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 补全接受功能已触发！".to_string(),
                    ))
                },
            )
            .await;
    }

    async fn register_system_actions(&mut self) {
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "clear_terminal".to_string(),
                    description: "清空终端".to_string(),

                    requires_terminal: true,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 清空终端功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "open_settings".to_string(),
                    description: "打开设置".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 打开设置功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "toggle_theme".to_string(),
                    description: "切换主题".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 切换主题功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "increase_font_size".to_string(),
                    description: "增大字体".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 增大字体功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "decrease_font_size".to_string(),
                    description: "减小字体".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 减小字体功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "toggle_ai_sidebar".to_string(),
                    description: "开启/关闭AI侧边栏".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 AI侧边栏切换功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        let _ = self
            .register_action(
                ActionMetadata {
                    name: "toggle_window_pin".to_string(),
                    description: "钉住/取消钉住窗口".to_string(),

                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |_context| {
                    Ok(serde_json::Value::String(
                        "🔥 窗口钉住切换功能已触发！".to_string(),
                    ))
                },
            )
            .await;
    }
}

impl Clone for ActionRegistry {
    fn clone(&self) -> Self {
        Self {
            handlers: Arc::clone(&self.handlers),
            metadata: Arc::clone(&self.metadata),
            event_listeners: Arc::clone(&self.event_listeners),
        }
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
    use crate::config::shortcuts::KeyCombination;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_action_registration() {
        let mut registry = ActionRegistry::new();

        let metadata = ActionMetadata {
            name: "test_action".to_string(),
            description: "Test action".to_string(),

            requires_terminal: false,
            is_system_action: false,
            supported_platforms: vec!["test".to_string()],
        };

        let result = registry
            .register_action(metadata, |_| {
                Ok(serde_json::Value::String("test".to_string()))
            })
            .await;

        assert!(result.is_ok());
        assert!(registry.is_action_registered("test_action").await);
    }

    #[tokio::test]
    async fn test_action_execution() {
        let mut registry = ActionRegistry::new();

        let metadata = ActionMetadata {
            name: "test_action".to_string(),
            description: "Test action".to_string(),

            requires_terminal: false,
            is_system_action: false,
            supported_platforms: vec!["test".to_string()],
        };

        registry
            .register_action(metadata, |_| {
                Ok(serde_json::Value::String("executed".to_string()))
            })
            .await
            .unwrap();

        let context = ActionContext {
            key_combination: KeyCombination::new("t".to_string(), vec!["cmd".to_string()]),
            active_terminal_id: None,
            metadata: HashMap::new(),
        };

        let action = ShortcutAction::Simple("test_action".to_string());
        let result = registry.execute_action(&action, &context).await;

        assert!(result.success);
        assert_eq!(
            result.data,
            Some(serde_json::Value::String("executed".to_string()))
        );
    }
}
