/*!
 * 终端面板管理命令
 *
 * 提供活跃终端面板的管理功能，包括：
 * - 设置活跃面板
 * - 获取活跃面板
 * - 清除活跃面板
 * - 检查面板活跃状态
 */

use super::TerminalContextState;
use crate::mux::PaneId;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use tauri::State;
use tracing::{debug, error, warn};

/// 设置活跃终端面板
///
/// 将指定的面板ID设置为当前活跃的终端。这会更新后端的活跃终端注册表，
/// 并触发相应的事件通知前端。
///
/// # Arguments
/// * `pane_id` - 要设置为活跃的面板ID
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(())` - 设置成功
/// * `Err(String)` - 设置失败的错误信息
#[tauri::command]
pub async fn terminal_context_set_active_pane(
    pane_id: u32,
    state: State<'_, TerminalContextState>,
) -> TauriApiResult<EmptyData> {
    debug!("设置活跃终端面板: pane_id={}", pane_id);

    // 参数验证
    if pane_id == 0 {
        warn!("面板ID不能为0");
        return Ok(api_error!("common.invalid_id"));
    }

    let pane_id = PaneId::new(pane_id);

    // 调用注册表设置活跃终端
    match state.registry.terminal_context_set_active_pane(pane_id) {
        Ok(()) => {
            debug!("成功设置活跃终端面板: pane_id={:?}", pane_id);
            Ok(api_success!())
        }
        Err(e) => {
            error!("设置活跃终端面板失败: {}", e);
            Ok(api_error!("terminal.set_active_pane_failed"))
        }
    }
}

/// 获取当前活跃终端面板ID
///
/// 返回当前活跃的终端面板ID。如果没有活跃的终端，返回None。
///
/// # Arguments
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(Some(u32))` - 当前活跃的面板ID
/// * `Ok(None)` - 没有活跃的终端
/// * `Err(String)` - 获取失败的错误信息
#[tauri::command]
pub async fn terminal_context_get_active_pane(
    state: State<'_, TerminalContextState>,
) -> TauriApiResult<Option<u32>> {
    debug!("获取当前活跃终端面板");

    let active_pane = state.registry.terminal_context_get_active_pane();
    let result = active_pane.map(|pane_id| pane_id.as_u32());

    debug!("当前活跃终端面板: {:?}", result);
    Ok(api_success!(result))
}

/// 清除当前活跃终端
///
/// 清除当前设置的活跃终端，使系统回到没有活跃终端的状态。
/// 这通常在所有终端都关闭时调用。
///
/// # Arguments
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(())` - 清除成功
/// * `Err(String)` - 清除失败的错误信息
#[tauri::command]
pub async fn terminal_context_clear_active_pane(
    state: State<'_, TerminalContextState>,
) -> TauriApiResult<EmptyData> {
    debug!("清除活跃终端面板");

    match state.registry.terminal_context_clear_active_pane() {
        Ok(()) => {
            debug!("成功清除活跃终端面板");
            Ok(api_success!())
        }
        Err(e) => {
            error!("清除活跃终端面板失败: {}", e);
            Ok(api_error!("terminal.clear_active_pane_failed"))
        }
    }
}

/// 检查指定面板是否为活跃终端
///
/// 检查给定的面板ID是否是当前活跃的终端。
///
/// # Arguments
/// * `pane_id` - 要检查的面板ID
/// * `state` - 终端上下文状态
///
/// # Returns
/// * `Ok(bool)` - true表示是活跃终端，false表示不是
/// * `Err(String)` - 检查失败的错误信息
#[tauri::command]
pub async fn terminal_context_is_pane_active(
    pane_id: u32,
    state: State<'_, TerminalContextState>,
) -> TauriApiResult<bool> {
    debug!("检查面板是否为活跃终端: pane_id={}", pane_id);

    // 参数验证
    if pane_id == 0 {
        warn!("面板ID不能为0");
        return Ok(api_error!("common.invalid_id"));
    }

    let pane_id = PaneId::new(pane_id);
    let is_active = state.registry.terminal_context_is_pane_active(pane_id);

    debug!(
        "面板活跃状态检查结果: pane_id={:?}, is_active={}",
        pane_id, is_active
    );
    Ok(api_success!(is_active))
}

#[cfg(test)]
mod tests {
    use crate::mux::PaneId;
    use crate::terminal::commands::tests::create_test_state;

    #[tokio::test]
    async fn test_set_and_get_active_pane() {
        let state = create_test_state();
        let pane_id = 123u32;

        // 初始状态应该没有活跃终端
        let result = state.registry.terminal_context_get_active_pane();
        assert_eq!(result, None);

        // 设置活跃终端
        let result = state.registry.terminal_context_set_active_pane(PaneId::new(pane_id));
        assert!(result.is_ok());

        // 验证活跃终端已设置
        let result = state.registry.terminal_context_get_active_pane();
        assert_eq!(result, Some(PaneId::new(pane_id)));
    }

    #[tokio::test]
    async fn test_invalid_pane_id_validation() {
        // 测试参数验证逻辑
        assert!(PaneId::new(0).as_u32() == 0); // 验证0是有效的PaneId值

        // 实际的验证逻辑在命令函数中，这里测试基础功能
        let state = create_test_state();
        let valid_pane_id = PaneId::new(123);

        // 测试设置有效的面板ID
        let result = state.registry.terminal_context_set_active_pane(valid_pane_id);
        assert!(result.is_ok());

        // 测试检查面板活跃状态
        let is_active = state.registry.terminal_context_is_pane_active(valid_pane_id);
        assert!(is_active);
    }

    #[tokio::test]
    async fn test_is_pane_active() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        // 初始状态下面板不应该是活跃的
        let is_active = state.registry.terminal_context_is_pane_active(pane_id);
        assert!(!is_active);

        // 设置活跃终端后应该返回true
        state.registry.terminal_context_set_active_pane(pane_id).unwrap();
        let is_active = state.registry.terminal_context_is_pane_active(pane_id);
        assert!(is_active);

        // 其他面板应该不是活跃的
        let other_pane = PaneId::new(456);
        let is_active = state.registry.terminal_context_is_pane_active(other_pane);
        assert!(!is_active);
    }

    #[tokio::test]
    async fn test_clear_active_pane() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        // 设置活跃终端
        state.registry.terminal_context_set_active_pane(pane_id).unwrap();
        assert_eq!(state.registry.terminal_context_get_active_pane(), Some(pane_id));

        // 清除活跃终端
        let result = state.registry.terminal_context_clear_active_pane();
        assert!(result.is_ok());

        // 验证活跃终端已清除
        assert_eq!(state.registry.terminal_context_get_active_pane(), None);
    }
}
