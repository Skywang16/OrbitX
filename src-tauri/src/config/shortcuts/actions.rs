/*!
 * 快捷键动作执行系统
 *
 * 负责：
 * - 动作注册和管理
 * - 动作执行调度
 * - 上下文传递
 */

use super::types::{ActionContext, OperationResult, ShortcutEvent, ShortcutEventType};
use crate::config::types::ShortcutAction;
use anyhow::Result as AnyResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// 动作执行器函数类型
pub type ActionHandler = Box<dyn Fn(&ActionContext) -> AnyResult<serde_json::Value> + Send + Sync>;

/// 动作元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionMetadata {
    /// 动作名称
    pub name: String,
    /// 动作描述
    pub description: String,
    /// 动作类别
    pub category: String,
    /// 是否需要终端上下文
    pub requires_terminal: bool,
    /// 是否为系统级动作
    pub is_system_action: bool,
    /// 支持的平台
    pub supported_platforms: Vec<String>,
}

/// 动作注册表
pub struct ActionRegistry {
    /// 已注册的动作处理器
    handlers: Arc<RwLock<HashMap<String, ActionHandler>>>,
    /// 动作元数据
    metadata: Arc<RwLock<HashMap<String, ActionMetadata>>>,
    /// 事件监听器
    event_listeners: Arc<RwLock<Vec<Box<dyn Fn(&ShortcutEvent) + Send + Sync>>>>,
}

impl ActionRegistry {
    /// 创建新的动作注册表
    pub fn new() -> Self {
        let registry = Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            event_listeners: Arc::new(RwLock::new(Vec::new())),
        };

        let mut registry_instance = registry.clone();
        // 注册默认动作
        tokio::spawn(async move {
            registry_instance.register_default_actions().await;
        });

        registry
    }

    /// 注册动作
    pub async fn register_action<F>(
        &mut self,
        metadata: ActionMetadata,
        handler: F,
    ) -> AnyResult<()>
    where
        F: Fn(&ActionContext) -> AnyResult<serde_json::Value> + Send + Sync + 'static,
    {
        debug!("注册动作: {}", metadata.name);

        let action_name = metadata.name.clone();

        // 存储元数据
        {
            let mut meta_map = self.metadata.write().await;
            meta_map.insert(action_name.clone(), metadata);
        }

        // 存储处理器
        {
            let mut handler_map = self.handlers.write().await;
            handler_map.insert(action_name.clone(), Box::new(handler));
        }

        info!("动作注册成功: {}", action_name);
        Ok(())
    }

    /// 执行动作
    pub async fn execute_action(
        &self,
        action: &ShortcutAction,
        context: &ActionContext,
    ) -> OperationResult<serde_json::Value> {
        let action_name = self.extract_action_name(action);
        debug!("执行动作: {}", action_name);

        // 发送按键事件
        self.emit_event(ShortcutEvent {
            event_type: ShortcutEventType::KeyPressed,
            key_combination: Some(context.key_combination.clone()),
            action: Some(action_name.clone()),
            data: HashMap::new(),
            timestamp: chrono::Utc::now(),
        })
        .await;

        // 检查动作是否已注册
        let handler_exists = {
            let handlers = self.handlers.read().await;
            handlers.contains_key(&action_name)
        };

        if !handler_exists {
            let error_msg = format!("未注册的动作: {}", action_name);
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

        // 执行动作
        let result = {
            let handlers = self.handlers.read().await;
            if let Some(handler) = handlers.get(&action_name) {
                handler(context)
            } else {
                Err(anyhow::anyhow!("动作处理器未找到"))
            }
        };

        match result {
            Ok(value) => {
                info!("动作执行成功: {}", action_name);

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
            Err(e) => {
                let error_msg = format!("动作执行失败: {}", e);
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

    /// 检查动作是否已注册
    pub async fn is_action_registered(&self, action_name: &str) -> bool {
        let handlers = self.handlers.read().await;
        handlers.contains_key(action_name)
    }

    /// 获取动作元数据
    pub async fn get_action_metadata(&self, action_name: &str) -> Option<ActionMetadata> {
        let metadata = self.metadata.read().await;
        metadata.get(action_name).cloned()
    }

    /// 获取所有已注册的动作
    pub async fn get_registered_actions(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }

    /// 添加事件监听器
    pub async fn add_event_listener<F>(&mut self, listener: F)
    where
        F: Fn(&ShortcutEvent) + Send + Sync + 'static,
    {
        let mut listeners = self.event_listeners.write().await;
        listeners.push(Box::new(listener));
    }

    /// 发送事件
    async fn emit_event(&self, event: ShortcutEvent) {
        let listeners = self.event_listeners.read().await;
        for listener in listeners.iter() {
            listener(&event);
        }
    }

    /// 提取动作名称
    fn extract_action_name(&self, action: &ShortcutAction) -> String {
        match action {
            ShortcutAction::Simple(name) => name.clone(),
            ShortcutAction::Complex { action_type, .. } => action_type.clone(),
        }
    }

    /// 注册默认动作
    async fn register_default_actions(&mut self) {
        // 全局动作
        self.register_global_actions().await;
        // 终端动作
        self.register_terminal_actions().await;
        // 系统动作
        self.register_system_actions().await;
    }

    /// 注册全局动作
    async fn register_global_actions(&mut self) {
        // 复制到剪贴板
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "copy_to_clipboard".to_string(),
                    description: "复制选中内容到剪贴板".to_string(),
                    category: "global".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行复制到剪贴板动作");
                    debug!("复制动作上下文: {:?}", context);
                    // 这里应该实现实际的复制逻辑
                    Ok(serde_json::Value::String("🔥 复制功能已触发！".to_string()))
                },
            )
            .await;

        // 从剪贴板粘贴
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "paste_from_clipboard".to_string(),
                    description: "从剪贴板粘贴内容".to_string(),
                    category: "global".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行从剪贴板粘贴动作");
                    debug!("粘贴动作上下文: {:?}", context);
                    // 这里应该实现实际的粘贴逻辑
                    Ok(serde_json::Value::String("🔥 粘贴功能已触发！".to_string()))
                },
            )
            .await;

        // 搜索
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "terminal_search".to_string(),
                    description: "终端搜索".to_string(),
                    category: "global".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行搜索动作");
                    debug!("搜索动作上下文: {:?}", context);
                    // 这里应该实现搜索逻辑
                    Ok(serde_json::Value::String("🔥 搜索功能已触发！".to_string()))
                },
            )
            .await;
    }

    /// 注册终端动作
    async fn register_terminal_actions(&mut self) {
        // 新建标签页
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "new_tab".to_string(),
                    description: "新建终端标签页".to_string(),
                    category: "terminal".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行新建标签页动作");
                    debug!("新建标签页上下文: {:?}", context);
                    Ok(serde_json::Value::String(
                        "🔥 新建标签页功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        // 关闭标签页
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "close_tab".to_string(),
                    description: "关闭当前终端标签页".to_string(),
                    category: "terminal".to_string(),
                    requires_terminal: true,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行关闭标签页动作");
                    debug!("关闭标签页上下文: {:?}", context);

                    // 检查前端执行结果，如果前端成功处理了关闭操作，就不继续处理
                    if let Some(frontend_result) = context.metadata.get("frontendResult") {
                        if let Some(result_bool) = frontend_result.as_bool() {
                            if result_bool {
                                debug!("前端已成功处理关闭标签页，后端跳过处理");
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

        // 标签页切换
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "switch_to_tab_1".to_string(),
                    description: "切换到标签页1".to_string(),
                    category: "terminal".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行切换到标签页1动作");
                    debug!("标签页切换上下文: {:?}", context);
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
                    category: "terminal".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行切换到标签页2动作");
                    debug!("标签页切换上下文: {:?}", context);
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
                    category: "terminal".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行切换到标签页3动作");
                    debug!("标签页切换上下文: {:?}", context);
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
                    category: "terminal".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行切换到标签页4动作");
                    debug!("标签页切换上下文: {:?}", context);
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
                    category: "terminal".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行切换到标签页5动作");
                    debug!("标签页切换上下文: {:?}", context);
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
                    category: "terminal".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行切换到最后一个标签页动作");
                    debug!("标签页切换上下文: {:?}", context);
                    Ok(serde_json::Value::String(
                        "🔥 切换到最后一个标签页功能已触发！".to_string(),
                    ))
                },
            )
            .await;

        // 补全接受
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "accept_completion".to_string(),
                    description: "接受当前补全建议".to_string(),
                    category: "terminal".to_string(),
                    requires_terminal: true,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行接受补全动作");
                    debug!("补全接受上下文: {:?}", context);
                    Ok(serde_json::Value::String(
                        "🔥 补全接受功能已触发！".to_string(),
                    ))
                },
            )
            .await;
    }

    /// 注册系统动作
    async fn register_system_actions(&mut self) {
        // 清空终端
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "clear_terminal".to_string(),
                    description: "清空终端".to_string(),
                    category: "system".to_string(),
                    requires_terminal: true,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行清空终端动作");
                    debug!("清空终端上下文: {:?}", context);
                    Ok(serde_json::Value::String("🔥 清空终端功能已触发！".to_string()))
                },
            )
            .await;

        // 打开设置
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "open_settings".to_string(),
                    description: "打开设置".to_string(),
                    category: "system".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行打开设置动作");
                    debug!("打开设置上下文: {:?}", context);
                    Ok(serde_json::Value::String("🔥 打开设置功能已触发！".to_string()))
                },
            )
            .await;

        // 切换主题
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "toggle_theme".to_string(),
                    description: "切换主题".to_string(),
                    category: "system".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行切换主题动作");
                    debug!("切换主题上下文: {:?}", context);
                    Ok(serde_json::Value::String("🔥 切换主题功能已触发！".to_string()))
                },
            )
            .await;

        // 增大字体
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "increase_font_size".to_string(),
                    description: "增大字体".to_string(),
                    category: "system".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行增大字体动作");
                    debug!("增大字体上下文: {:?}", context);
                    Ok(serde_json::Value::String("🔥 增大字体功能已触发！".to_string()))
                },
            )
            .await;

        // 减小字体
        let _ = self
            .register_action(
                ActionMetadata {
                    name: "decrease_font_size".to_string(),
                    description: "减小字体".to_string(),
                    category: "system".to_string(),
                    requires_terminal: false,
                    is_system_action: false,
                    supported_platforms: vec![
                        "windows".to_string(),
                        "macos".to_string(),
                        "linux".to_string(),
                    ],
                },
                |context| {
                    info!("🔥 执行减小字体动作");
                    debug!("减小字体上下文: {:?}", context);
                    Ok(serde_json::Value::String("🔥 减小字体功能已触发！".to_string()))
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
    use std::collections::HashMap;
    use crate::config::shortcuts::KeyCombination;

    #[tokio::test]
    async fn test_action_registration() {
        let mut registry = ActionRegistry::new();

        let metadata = ActionMetadata {
            name: "test_action".to_string(),
            description: "Test action".to_string(),
            category: "test".to_string(),
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
            category: "test".to_string(),
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
