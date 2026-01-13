/*!
 * 终端模块的Tauri命令接口
 *
 * Note: Event handling has been moved to terminal::event_handler for unified event management.
 * This module now focuses solely on terminal command implementations.
 */

use tauri::{AppHandle, Runtime, State};
use tracing::error;

use crate::mux::{get_mux, PaneId, PtySize, ShellConfig, ShellInfo, ShellManager, TerminalConfig};
use crate::utils::{ApiResponse, EmptyData, TauriApiResult};
use crate::{api_error, api_success};

/// 参数验证辅助函数
fn terminal_size_valid(rows: u16, cols: u16) -> bool {
    rows > 0 && cols > 0
}

/// 终端状态管理
///
pub struct TerminalState {
    // 但保留这个结构体以便将来扩展其他状态
    _placeholder: (),
}

impl TerminalState {
    /// 初始化方法
    ///
    /// 注意：不在此时验证 Mux，因为 Mux 需要在 setup 中才会被初始化
    pub fn new() -> Result<Self, String> {
        let state = Self { _placeholder: () };
        Ok(state)
    }

    /// 验证状态完整性
    /// 只在调用时验证，不在初始化时验证
    pub fn validate(&self) -> TauriApiResult<EmptyData> {
        let mux = get_mux();

        // 验证Mux实例是否可访问
        mux.pane_count();

        Ok(ApiResponse::ok(EmptyData))
    }
}

/// 创建新终端会话
///
#[tauri::command]
pub async fn terminal_create<R: Runtime>(
    rows: u16,
    cols: u16,
    cwd: Option<String>,
    _app: AppHandle<R>,
    _state: State<'_, TerminalState>,
) -> TauriApiResult<u32> {
    if !terminal_size_valid(rows, cols) {
        return Ok(api_error!("shell.terminal_size_invalid"));
    }

    let mux = get_mux();
    let size = PtySize::new(rows, cols);

    // 根据是否指定初始目录选择创建方式
    let result = if let Some(working_dir) = cwd {
        let mut shell_config = ShellConfig::with_default_shell();
        shell_config.working_directory = Some(working_dir.clone().into());
        let config = TerminalConfig::with_shell(shell_config);

        mux.create_pane_with_config(size, &config)
            .await
            .map(|pane_id| (pane_id, Some(working_dir)))
    } else {
        mux.create_pane(size).await.map(|pane_id| (pane_id, None))
    };

    match result {
        Ok((pane_id, working_dir)) => {
            // 立即同步初始 CWD 到 ShellIntegration，避免冷启动空窗期
            if let Some(initial_cwd) = &working_dir {
                mux.shell_update_pane_cwd(pane_id, initial_cwd.clone());
            }

            Ok(api_success!(pane_id.as_u32()))
        }
        Err(_) => Ok(api_error!("shell.create_terminal_failed")),
    }
}

/// 向终端写入数据
///
#[tauri::command]
pub async fn terminal_write(
    pane_id: u32,
    data: String,
    _state: State<'_, TerminalState>,
) -> TauriApiResult<EmptyData> {
    if data.is_empty() {
        return Ok(api_error!("common.empty_content"));
    }

    let mux = get_mux();
    let pane_id_obj = PaneId::from(pane_id);

    match mux.write_to_pane(pane_id_obj, data.as_bytes()) {
        Ok(_) => Ok(api_success!()),
        Err(_) => Ok(api_error!("shell.write_terminal_failed")),
    }
}

/// 调整终端大小
///
#[tauri::command]
pub async fn terminal_resize(
    pane_id: u32,
    rows: u16,
    cols: u16,
    _state: State<'_, TerminalState>,
) -> TauriApiResult<EmptyData> {
    if !terminal_size_valid(rows, cols) {
        return Ok(api_error!("shell.terminal_size_invalid"));
    }

    let mux = get_mux();
    let pane_id_obj = PaneId::from(pane_id);
    let size = PtySize::new(rows, cols);

    match mux.resize_pane(pane_id_obj, size) {
        Ok(_) => Ok(api_success!()),
        Err(err) => match err {
            crate::mux::error::TerminalMuxError::PaneNotFound { .. } => Ok(api_success!()),
            _ => Ok(api_error!("shell.resize_terminal_failed")),
        },
    }
}

/// 关闭终端会话
///
#[tauri::command]
pub async fn terminal_close(
    pane_id: u32,
    _state: State<'_, TerminalState>,
) -> TauriApiResult<EmptyData> {
    let mux = get_mux();
    let pane_id_obj = PaneId::from(pane_id);

    // 原子操作：直接尝试删除面板，避免检查和删除之间的竞态条件
    match mux.remove_pane(pane_id_obj) {
        Ok(_) => Ok(api_success!()),
        Err(err) => {
            match err {
                crate::mux::error::TerminalMuxError::PaneNotFound { .. } => {
                    // 面板不存在，认为操作成功
                    Ok(api_success!())
                }
                _ => {
                    // 其他错误，返回失败
                    Ok(api_error!("shell.close_terminal_failed"))
                }
            }
        }
    }
}

/// 获取终端列表
///
#[tauri::command]
pub async fn terminal_list() -> TauriApiResult<Vec<u32>> {
    let mux = get_mux();
    let pane_ids: Vec<u32> = mux.list_panes().into_iter().map(|id| id.as_u32()).collect();
    Ok(api_success!(pane_ids))
}

/// 获取系统可用的shell列表
///
#[tauri::command]
pub async fn terminal_get_available_shells() -> TauriApiResult<Vec<ShellInfo>> {
    let shells = ShellManager::detect_available_shells();
    Ok(api_success!(shells))
}

/// 获取系统默认shell信息
///
#[tauri::command]
pub async fn terminal_get_default_shell() -> TauriApiResult<ShellInfo> {
    let default_shell = ShellManager::terminal_get_default_shell();
    Ok(api_success!(default_shell))
}

/// 验证shell路径是否有效
///
#[tauri::command]
pub async fn terminal_validate_shell_path(path: String) -> TauriApiResult<bool> {
    if path.trim().is_empty() {
        return Ok(api_error!("shell.command_empty"));
    }

    let is_valid = ShellManager::validate_shell(&path);
    Ok(api_success!(is_valid))
}

/// 使用指定shell创建终端
///
#[tauri::command]
pub async fn terminal_create_with_shell<R: Runtime>(
    shell_name: Option<String>,
    rows: u16,
    cols: u16,
    _app: AppHandle<R>,
    _state: State<'_, TerminalState>,
) -> TauriApiResult<u32> {
    if rows == 0 || cols == 0 {
        return Ok(api_error!("shell.terminal_size_invalid"));
    }

    let shell_info = match shell_name {
        Some(name) => match ShellManager::terminal_find_shell_by_name(&name) {
            Some(shell) => shell,
            None => {
                error!("未找到指定shell: {}", name);
                return Ok(api_error!("shell.shell_not_found"));
            }
        },
        None => ShellManager::terminal_get_default_shell(),
    };

    let mux = get_mux();
    let size = PtySize::new(rows, cols);

    let shell_config = ShellConfig::with_shell(shell_info);
    let config = TerminalConfig::with_shell(shell_config);

    // 使用配置创建面板
    match mux.create_pane_with_config(size, &config).await {
        Ok(pane_id) => Ok(api_success!(pane_id.as_u32())),
        Err(_) => {
            error!("创建终端失败");
            Ok(api_error!("shell.create_terminal_failed"))
        }
    }
}
