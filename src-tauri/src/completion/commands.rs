//! 补全功能的Tauri命令接口
//!
//! 提供前端调用的补全API
//!
//! 统一的补全命令处理规范：
//! 1. 参数顺序：业务参数在前，state参数在后
//! 2. 异步处理：所有命令都是async，统一错误转换
//! 3. 日志记录：每个命令都记录调用和结果日志
//! 4. 状态管理：统一使用CompletionState访问各组件

use crate::completion::engine::{CompletionEngine, CompletionEngineConfig};
use crate::completion::types::{
    CompletionContext, CompletionResponse, EnhancedCompletionItem, EnhancedCompletionResponse,
};
use crate::storage::commands::StorageCoordinatorState;
use crate::utils::error::ToTauriResult;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;

/// 补全模块状态管理
pub struct CompletionState {
    /// 补全引擎实例
    pub engine: Arc<Mutex<Option<Arc<CompletionEngine>>>>,
}

impl Default for CompletionState {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionState {
    /// 创建新的补全状态
    pub fn new() -> Self {
        Self {
            engine: Arc::new(Mutex::new(None)),
        }
    }

    /// 验证状态完整性
    pub async fn validate(&self) -> Result<(), String> {
        let engine_state = self
            .engine
            .lock()
            .map_err(|e| format!("[配置错误] 获取引擎状态锁失败: {e}"))?;

        match engine_state.as_ref() {
            Some(_) => Ok(()),
            None => Err("[配置错误] 补全引擎未初始化".to_string()),
        }
    }

    /// 获取引擎实例
    pub async fn get_engine(&self) -> Result<Arc<CompletionEngine>, String> {
        let engine_state = self
            .engine
            .lock()
            .map_err(|e| format!("[配置错误] 获取引擎状态锁失败: {e}"))?;

        match engine_state.as_ref() {
            Some(engine) => Ok(Arc::clone(engine)),
            None => Err("[配置错误] 补全引擎未初始化".to_string()),
        }
    }

    /// 设置引擎实例
    pub async fn set_engine(&self, engine: Arc<CompletionEngine>) -> Result<(), String> {
        let mut engine_state = self
            .engine
            .lock()
            .map_err(|e| format!("[配置错误] 获取引擎状态锁失败: {e}"))?;

        *engine_state = Some(engine);
        Ok(())
    }
}

/// 获取补全建议命令
#[tauri::command]
pub async fn get_completions(
    input: String,
    cursor_position: usize,
    working_directory: String,
    max_results: Option<usize>,
    state: State<'_, CompletionState>,
) -> Result<CompletionResponse, String> {
    let engine = state.get_engine().await?;

    // 创建补全上下文
    let working_directory = PathBuf::from(&working_directory);
    let context = CompletionContext::new(input, cursor_position, working_directory);

    // 获取补全建议
    match engine.get_completions(&context).await {
        Ok(mut response) => {
            // 如果指定了最大结果数，进行限制
            if let Some(max_results) = max_results {
                if response.items.len() > max_results {
                    response.items.truncate(max_results);
                    response.has_more = true;
                }
            }

            Ok(response)
        }
        Err(e) => Err(format!("提供者错误: 获取补全失败: {e}")),
    }
}

/// 初始化补全引擎命令
#[tauri::command]
pub async fn init_completion_engine(
    state: State<'_, CompletionState>,
    storage_state: State<'_, StorageCoordinatorState>,
) -> Result<(), String> {
    let config = CompletionEngineConfig::default();
    let cache = storage_state.coordinator.cache();

    match CompletionEngine::with_default_providers(config, cache).await {
        Ok(engine) => {
            state.set_engine(Arc::new(engine)).await?;
            Ok(())
        }
        Err(e) => Err(format!("配置错误: 初始化失败: {e}")),
    }
}

/// 清理缓存命令（已简化，无缓存）
#[tauri::command]
pub async fn clear_completion_cache(_state: State<'_, CompletionState>) -> Result<(), String> {
    // 缓存已删除，直接返回成功
    Ok(())
}

/// 获取统计信息命令
#[tauri::command]
pub async fn get_completion_stats(state: State<'_, CompletionState>) -> Result<String, String> {
    let engine = state.get_engine().await?;

    match engine.get_stats() {
        Ok(stats) => {
            let stats_json = serde_json::json!({
                "provider_count": stats.provider_count
            });

            Ok(stats_json.to_string())
        }
        Err(e) => Err(format!("[提供者错误] 获取统计信息失败: {e}")),
    }
}

/// 获取增强补全建议命令
#[tauri::command]
pub async fn get_enhanced_completions(
    current_line: String,
    cursor_position: usize,
    working_directory: String,
    state: State<'_, CompletionState>,
) -> Result<EnhancedCompletionResponse, String> {
    let engine = state.get_engine().await?;

    // 创建补全上下文
    let working_directory = PathBuf::from(&working_directory);
    let context = CompletionContext::new(current_line, cursor_position, working_directory);

    // 获取标准补全建议
    let response = engine.get_completions(&context).await.to_tauri()?;

    // 转换为增强补全格式
    let enhanced_items: Vec<EnhancedCompletionItem> = response
        .items
        .into_iter()
        .map(|item| {
            let icon = match item.completion_type.as_str() {
                "file" => "📄",
                "directory" => "📁",
                "command" => "⚡",
                "history" => "🕒",
                "environment" => "🌍",
                "alias" => "🔗",
                "function" => "⚙️",
                "option" => "🔧",
                "subcommand" => "📋",
                "value" => "💎",
                _ => "📝",
            };

            let category = match item.completion_type.as_str() {
                "file" | "directory" => "filesystem",
                "command" | "subcommand" => "command",
                "history" => "history",
                "environment" => "environment",
                _ => "general",
            };

            EnhancedCompletionItem {
                text: item.text,
                display_text: item.display_text,
                description: item.description.unwrap_or_default(),
                icon: icon.to_string(),
                category: category.to_string(),
                priority: item.score as i32,
                metadata: item.metadata,
            }
        })
        .collect();

    // 计算补全位置
    let position = crate::completion::types::EnhancedCompletionPosition {
        x: (cursor_position as i32) * 8, // 假设每个字符8像素宽
        y: 20,                           // 固定高度
    };

    Ok(EnhancedCompletionResponse {
        completions: enhanced_items,
        position,
        has_shell_completions: true,
    })
}
