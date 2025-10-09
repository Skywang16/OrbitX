use super::TerminalContextState;
use crate::mux::PaneId;
use crate::utils::{EmptyData, TauriApiResult};
use crate::{api_error, api_success};
use tauri::State;
use tracing::{debug, error, warn};

/// 设置活跃终端面板
#[tauri::command]
pub async fn terminal_context_set_active_pane(
    pane_id: u32,
    state: State<'_, TerminalContextState>,
) -> TauriApiResult<EmptyData> {
    debug!("设置活跃终端面板: pane_id={}", pane_id);

    if pane_id == 0 {
        warn!("面板ID不能为0");
        return Ok(api_error!("common.invalid_id"));
    }

    let pane_id = PaneId::new(pane_id);

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
#[tauri::command]
pub async fn terminal_context_is_pane_active(
    pane_id: u32,
    state: State<'_, TerminalContextState>,
) -> TauriApiResult<bool> {
    debug!("检查面板是否为活跃终端: pane_id={}", pane_id);

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

        let result = state.registry.terminal_context_get_active_pane();
        assert_eq!(result, None);

        let result = state
            .registry
            .terminal_context_set_active_pane(PaneId::new(pane_id));
        assert!(result.is_ok());

        let result = state.registry.terminal_context_get_active_pane();
        assert_eq!(result, Some(PaneId::new(pane_id)));
    }

    #[tokio::test]
    async fn test_invalid_pane_id_validation() {
        assert!(PaneId::new(0).as_u32() == 0);

        let state = create_test_state();
        let valid_pane_id = PaneId::new(123);

        let result = state
            .registry
            .terminal_context_set_active_pane(valid_pane_id);
        assert!(result.is_ok());

        let is_active = state
            .registry
            .terminal_context_is_pane_active(valid_pane_id);
        assert!(is_active);
    }

    #[tokio::test]
    async fn test_is_pane_active() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        let is_active = state.registry.terminal_context_is_pane_active(pane_id);
        assert!(!is_active);

        state
            .registry
            .terminal_context_set_active_pane(pane_id)
            .unwrap();
        let is_active = state.registry.terminal_context_is_pane_active(pane_id);
        assert!(is_active);

        let other_pane = PaneId::new(456);
        let is_active = state.registry.terminal_context_is_pane_active(other_pane);
        assert!(!is_active);
    }

    #[tokio::test]
    async fn test_clear_active_pane() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        state
            .registry
            .terminal_context_set_active_pane(pane_id)
            .unwrap();
        assert_eq!(
            state.registry.terminal_context_get_active_pane(),
            Some(pane_id)
        );

        let result = state.registry.terminal_context_clear_active_pane();
        assert!(result.is_ok());

        assert_eq!(state.registry.terminal_context_get_active_pane(), None);
    }
}
