/*!
 * 终端上下文管理命令
 *
 * 提供终端上下文信息的获取功能，包括：
 * - 获取指定终端的上下文
 * - 获取活跃终端的上下文
 * - 支持回退逻辑处理
 */

use super::TerminalContextState;
use crate::mux::PaneId;
use crate::terminal::TerminalContext;
use crate::utils::TauriApiResult;
use crate::{api_error, api_success};
use tauri::State;
use tracing::{debug, error, warn};

/// 获取指定终端的上下文信息
///
/// 根据提供的面板ID获取终端的完整上下文信息，包括当前工作目录、
/// Shell类型、命令历史等。如果不提供面板ID，则获取当前活跃终端的上下文。
///
/// # Arguments
/// * `pane_id` - 可选的面板ID，如果为None则使用活跃终端
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(TerminalContext)` - 终端上下文信息
/// * `Err(String)` - 获取失败的错误信息
#[tauri::command]
pub async fn terminal_context_get(
    pane_id: Option<u32>,
    state: State<'_, TerminalContextState>,
) -> TauriApiResult<TerminalContext> {
    debug!("获取终端上下文: pane_id={:?}", pane_id);

    if let Some(id) = pane_id {
        if id == 0 {
            warn!("面板ID不能为0");
            return Ok(api_error!("common.invalid_id"));
        }
    }

    let pane_id = pane_id.map(PaneId::new);

    // 使用上下文服务获取终端上下文，支持回退逻辑
    match state
        .context_service
        .get_context_with_fallback(pane_id)
        .await
    {
        Ok(context) => {
            debug!(
                "成功获取终端上下文: pane_id={:?}, cwd={:?}",
                context.pane_id, context.current_working_directory
            );
            Ok(api_success!(context))
        }
        Err(e) => {
            error!("获取终端上下文失败: {}", e);
            Ok(api_error!("terminal.get_context_failed"))
        }
    }
}

/// 获取当前活跃终端的上下文信息
///
///
/// # Arguments
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(TerminalContext)` - 活跃终端的上下文信息
/// * `Err(String)` - 获取失败的错误信息
#[tauri::command]
pub async fn terminal_context_get_active(
    state: State<'_, TerminalContextState>,
) -> TauriApiResult<TerminalContext> {
    debug!("获取活跃终端上下文");

    match state.context_service.get_active_context().await {
        Ok(context) => {
            debug!(
                "成功获取活跃终端上下文: pane_id={:?}, cwd={:?}",
                context.pane_id, context.current_working_directory
            );
            Ok(api_success!(context))
        }
        Err(e) if e.to_string().contains("No active terminal pane") => {
            // 没有活跃终端时，使用回退逻辑
            debug!("没有活跃终端，使用回退逻辑");
            match state.context_service.get_context_with_fallback(None).await {
                Ok(context) => {
                    debug!("使用回退逻辑成功获取终端上下文");
                    Ok(api_success!(context))
                }
                Err(e) => {
                    error!("获取活跃终端上下文失败（回退也失败）: {}", e);
                    Ok(api_error!("terminal.get_active_context_failed"))
                }
            }
        }
        Err(e) => {
            error!("获取活跃终端上下文失败: {}", e);
            Ok(api_error!("terminal.get_active_context_failed"))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::mux::PaneId;
    use crate::terminal::commands::tests::create_test_state;

    #[tokio::test]
    async fn test_get_terminal_context_fallback() {
        let state = create_test_state();

        // 没有活跃终端时，应该返回默认上下文
        let result = state.context_service.get_context_with_fallback(None).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.current_working_directory, Some("~".to_string()));
        assert!(matches!(
            context.shell_type,
            Some(crate::terminal::ShellType::Bash)
        ));
    }

    #[tokio::test]
    async fn test_get_active_terminal_context_fallback() {
        let state = create_test_state();

        // 没有活跃终端时，get_active_context应该返回错误
        let result = state.context_service.get_active_context().await;
        assert!(result.is_err() && result.unwrap_err().to_string().contains("No active terminal pane"));

        // 但是get_context_with_fallback应该返回默认上下文
        let result = state.context_service.get_context_with_fallback(None).await;
        assert!(result.is_ok());

        let context = result.unwrap();
        assert_eq!(context.current_working_directory, Some("~".to_string()));
        assert!(!context.shell_integration_enabled);
    }

    #[tokio::test]
    async fn test_context_service_integration() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        state
            .registry
            .terminal_context_set_active_pane(pane_id)
            .unwrap();

        // 测试获取活跃终端上下文（应该失败，因为面板不存在于mux中）
        let result = state.context_service.get_active_context().await;
        if result.is_err() {
            let error_msg = result.unwrap_err().to_string();
            println!("实际错误消息: '{}'", error_msg);
            assert!(
                error_msg.contains("面板不存在")
                    || error_msg.contains("pane")
                    || error_msg.contains("active")
                    || error_msg.contains("查询终端上下文失败")
            );
        } else {
            panic!("Expected error for non-existent pane");
        }

        // 测试使用回退逻辑
        let result = state
            .context_service
            .get_context_with_fallback(Some(pane_id))
            .await;
        assert!(result.is_ok());

        let context = result.unwrap();
        // 由于面板不存在，应该回退到默认上下文
        assert_eq!(context.current_working_directory, Some("~".to_string()));
    }
}
