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

/// 向后兼容的类型别名
pub type CompletionEngineState = CompletionState;

/// 获取补全建议命令
#[tauri::command]
pub async fn get_completions(
    input: String,
    cursor_position: usize,
    working_directory: String,
    max_results: Option<usize>,
    state: State<'_, CompletionEngineState>,
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
pub async fn init_completion_engine(state: State<'_, CompletionEngineState>) -> Result<(), String> {
    let config = CompletionEngineConfig::default();

    match CompletionEngine::with_default_providers(config).await {
        Ok(engine) => {
            state.set_engine(Arc::new(engine)).await?;
            Ok(())
        }
        Err(e) => Err(format!("配置错误: 初始化失败: {e}")),
    }
}

/// 清理缓存命令
#[tauri::command]
pub async fn clear_completion_cache(state: State<'_, CompletionEngineState>) -> Result<(), String> {
    let engine = state.get_engine().await?;

    match engine.clear_cache() {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("[缓存错误] 清理缓存失败: {e}")),
    }
}

/// 获取统计信息命令
#[tauri::command]
pub async fn get_completion_stats(
    state: State<'_, CompletionEngineState>,
) -> Result<String, String> {
    let engine = state.get_engine().await?;

    match engine.get_stats() {
        Ok(stats) => {
            let stats_json = serde_json::json!({
                "provider_count": stats.provider_count,
                "cache_stats": stats.cache_stats.map(|cs| serde_json::json!({
                    "total_entries": cs.total_entries,
                    "capacity": cs.capacity,
                    "expired_entries": cs.expired_entries,
                    "hit_rate": cs.hit_rate,
                }))
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
    state: State<'_, CompletionEngineState>,
) -> Result<EnhancedCompletionResponse, String> {
    let engine = state.get_engine().await?;

    // 创建补全上下文
    let working_directory = PathBuf::from(&working_directory);
    let context = CompletionContext::new(current_line, cursor_position, working_directory);

    // 获取标准补全建议
    let response = engine
        .get_completions(&context)
        .await
        .map_err(|e| e.to_string())?;

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
