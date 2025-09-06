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
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// await invoke('set_active_pane', { paneId: 123 });
/// ```
#[tauri::command]
pub async fn set_active_pane(
    pane_id: u32,
    state: State<'_, TerminalContextState>,
) -> Result<(), String> {
    debug!("设置活跃终端面板: pane_id={}", pane_id);

    // 参数验证
    if pane_id == 0 {
        let error_msg = "面板ID不能为0".to_string();
        warn!("{}", error_msg);
        return Err(error_msg);
    }

    let pane_id = PaneId::new(pane_id);

    // 调用注册表设置活跃终端
    match state.registry.set_active_pane(pane_id) {
        Ok(()) => {
            debug!("成功设置活跃终端面板: pane_id={:?}", pane_id);
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("设置活跃终端面板失败: {}", e);
            error!("{}", error_msg);
            Err(error_msg)
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
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// const activePaneId = await invoke('get_active_pane');
/// if (activePaneId !== null) {
///     console.log('当前活跃终端:', activePaneId);
/// }
/// ```
#[tauri::command]
pub async fn get_active_pane(
    state: State<'_, TerminalContextState>,
) -> Result<Option<u32>, String> {
    debug!("获取当前活跃终端面板");

    let active_pane = state.registry.get_active_pane();
    let result = active_pane.map(|pane_id| pane_id.as_u32());

    debug!("当前活跃终端面板: {:?}", result);
    Ok(result)
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
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// await invoke('clear_active_pane');
/// ```
#[tauri::command]
pub async fn clear_active_pane(state: State<'_, TerminalContextState>) -> Result<(), String> {
    debug!("清除活跃终端面板");

    match state.registry.clear_active_pane() {
        Ok(()) => {
            debug!("成功清除活跃终端面板");
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("清除活跃终端面板失败: {}", e);
            error!("{}", error_msg);
            Err(error_msg)
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
///
/// # Examples
/// ```javascript
/// // 前端调用示例
/// const isActive = await invoke('is_pane_active', { paneId: 123 });
/// if (isActive) {
///     console.log('面板123是活跃终端');
/// }
/// ```
#[tauri::command]
pub async fn is_pane_active(
    pane_id: u32,
    state: State<'_, TerminalContextState>,
) -> Result<bool, String> {
    debug!("检查面板是否为活跃终端: pane_id={}", pane_id);

    // 参数验证
    if pane_id == 0 {
        let error_msg = "面板ID不能为0".to_string();
        warn!("{}", error_msg);
        return Err(error_msg);
    }

    let pane_id = PaneId::new(pane_id);
    let is_active = state.registry.is_pane_active(pane_id);

    debug!(
        "面板活跃状态检查结果: pane_id={:?}, is_active={}",
        pane_id, is_active
    );
    Ok(is_active)
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
        let result = state.registry.get_active_pane();
        assert_eq!(result, None);

        // 设置活跃终端
        let result = state.registry.set_active_pane(PaneId::new(pane_id));
        assert!(result.is_ok());

        // 验证活跃终端已设置
        let result = state.registry.get_active_pane();
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
        let result = state.registry.set_active_pane(valid_pane_id);
        assert!(result.is_ok());

        // 测试检查面板活跃状态
        let is_active = state.registry.is_pane_active(valid_pane_id);
        assert!(is_active);
    }

    #[tokio::test]
    async fn test_is_pane_active() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        // 初始状态下面板不应该是活跃的
        let is_active = state.registry.is_pane_active(pane_id);
        assert!(!is_active);

        // 设置活跃终端后应该返回true
        state.registry.set_active_pane(pane_id).unwrap();
        let is_active = state.registry.is_pane_active(pane_id);
        assert!(is_active);

        // 其他面板应该不是活跃的
        let other_pane = PaneId::new(456);
        let is_active = state.registry.is_pane_active(other_pane);
        assert!(!is_active);
    }

    #[tokio::test]
    async fn test_clear_active_pane() {
        let state = create_test_state();
        let pane_id = PaneId::new(123);

        // 设置活跃终端
        state.registry.set_active_pane(pane_id).unwrap();
        assert_eq!(state.registry.get_active_pane(), Some(pane_id));

        // 清除活跃终端
        let result = state.registry.clear_active_pane();
        assert!(result.is_ok());

        // 验证活跃终端已清除
        assert_eq!(state.registry.get_active_pane(), None);
    }
}
