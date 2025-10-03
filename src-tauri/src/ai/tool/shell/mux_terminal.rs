/*!
 * 终端模块的Tauri命令接口
 *
 * Note: Event handling has been moved to terminal::event_handler for unified event management.
 * This module now focuses solely on terminal command implementations.
 */

use anyhow::{anyhow, Context};
use tauri::{AppHandle, Runtime, State};
use tracing::{debug, error, warn};

use crate::mux::{
    get_mux, PaneId, PtySize, ShellConfig, ShellInfo, ShellManager, ShellManagerStats,
    TerminalConfig,
};
use crate::utils::error::{AppResult, ToTauriResult};
use crate::utils::{ApiResponse, EmptyData, TauriApiResult};
use crate::{api_error, api_success};

/// 参数验证辅助函数
fn validate_terminal_size(rows: u16, cols: u16) -> AppResult<()> {
    if rows == 0 || cols == 0 {
        return Err(anyhow!("终端尺寸不能为0 (当前: {}x{})", cols, rows));
    }
    Ok(())
}

fn validate_non_empty_string(value: &str, field_name: &str) -> AppResult<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{}不能为空", field_name));
    }
    Ok(())
}

/// 终端状态管理
///
pub struct TerminalState {
    // 但保留这个结构体以便将来扩展其他状态
    _placeholder: (),
}

impl TerminalState {
    /// 初始化方法
    pub fn new() -> Result<Self, String> {
        let state = Self { _placeholder: () };

        // 验证状态完整性
        state.validate()?;

        Ok(state)
    }

    /// 验证状态完整性
    pub fn validate(&self) -> TauriApiResult<EmptyData> {
        let mux = get_mux();

        // 验证Mux实例是否可访问
        mux.pane_count();

        Ok(ApiResponse::ok(EmptyData::default()))
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
    debug!("创建终端会话: {}x{}, 初始目录: {:?}", cols, rows, cwd);
    debug!("当前Mux状态 - 面板数量: {}", get_mux().pane_count());

    if let Err(_) = validate_terminal_size(rows, cols) {
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
                debug!("初始化 ShellIntegration CWD: pane_id={}, cwd={}", pane_id.as_u32(), initial_cwd);
            }

            let dir_info = working_dir
                .map(|dir| format!(", 初始目录: {}", dir))
                .unwrap_or_default();

            debug!(
                "终端创建成功: ID={}{}, 新的面板数量: {}",
                pane_id.as_u32(),
                dir_info,
                mux.pane_count()
            );
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
    debug!(
        "写入终端数据: ID={}, 数据长度={}, 数据预览: {:?}",
        pane_id,
        data.len(),
        &data[..std::cmp::min(50, data.len())]
    );

    if data.is_empty() {
        return Ok(api_error!("common.empty_content"));
    }

    let mux = get_mux();
    let pane_id_obj = PaneId::from(pane_id);

    match mux.write_to_pane(pane_id_obj, data.as_bytes()) {
        Ok(_) => {
            debug!("写入终端成功: ID={}", pane_id);
            Ok(api_success!())
        }
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
    debug!("调整终端大小: ID={}, 大小={}x{}", pane_id, cols, rows);

    if let Err(_) = validate_terminal_size(rows, cols) {
        return Ok(api_error!("shell.terminal_size_invalid"));
    }

    let mux = get_mux();
    let pane_id_obj = PaneId::from(pane_id);
    let size = PtySize::new(rows, cols);

    match mux.resize_pane(pane_id_obj, size) {
        Ok(_) => {
            debug!("调整终端大小成功: ID={}", pane_id);
            Ok(api_success!())
        }
        Err(_) => Ok(api_error!("shell.resize_terminal_failed")),
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

    debug!(
        "关闭终端会话: ID={}, 当前面板数量: {}",
        pane_id,
        mux.pane_count()
    );

    // 原子操作：直接尝试删除面板，避免检查和删除之间的竞态条件
    match mux.remove_pane(pane_id_obj) {
        Ok(_) => {
            debug!(
                "关闭终端成功: ID={}, 剩余面板数量: {}",
                pane_id,
                mux.pane_count()
            );
            Ok(api_success!())
        }
        Err(e) => {
            let error_str = e.to_string();
            if error_str.contains("not found") || error_str.contains("不存在") {
                // 面板不存在，认为操作成功
                warn!("尝试关闭不存在的面板: ID={}, 可能已被其他操作关闭", pane_id);
                Ok(api_success!())
            } else {
                // 其他错误，返回失败
                Ok(api_error!("shell.close_terminal_failed"))
            }
        }
    }
}

/// 获取终端列表
///
#[tauri::command]
pub async fn terminal_list(_state: State<'_, TerminalState>) -> TauriApiResult<Vec<u32>> {
    debug!("获取终端列表");

    let mux = get_mux();
    let pane_ids: Vec<u32> = mux.list_panes().into_iter().map(|id| id.as_u32()).collect();

    debug!("获取终端列表成功: count={}", pane_ids.len());
    debug!("当前终端列表: {:?}", pane_ids);
    Ok(api_success!(pane_ids))
}

/// 获取终端缓冲区内容
///
#[tauri::command]
pub async fn terminal_get_buffer(pane_id: u32) -> TauriApiResult<String> {
    debug!("开始获取终端缓冲区内容: ID={}", pane_id);

    use crate::completion::output_analyzer::OutputAnalyzer;

    match OutputAnalyzer::global().get_pane_buffer(pane_id) {
        Ok(content) => {
            debug!(
                "获取终端缓冲区成功: ID={}, 内容长度={}",
                pane_id,
                content.len()
            );
            Ok(api_success!(content))
        }
        Err(e) => {
            let error_msg = format!("获取终端缓冲区失败: ID={}, 错误: {}", pane_id, e);
            error!("{}", error_msg);
            Ok(api_error!("shell.get_buffer_failed"))
        }
    }
}

/// 设置终端缓冲区内容
///
#[tauri::command]
pub async fn terminal_set_buffer(pane_id: u32, content: String) -> TauriApiResult<EmptyData> {
    debug!(
        "开始设置终端缓冲区内容: ID={}, 内容长度={}",
        pane_id,
        content.len()
    );

    use crate::completion::output_analyzer::OutputAnalyzer;

    match OutputAnalyzer::global().set_pane_buffer(pane_id, content) {
        Ok(_) => {
            debug!("设置终端缓冲区成功: ID={}", pane_id);
            Ok(api_success!())
        }
        Err(_) => Ok(api_error!("shell.set_buffer_failed")),
    }
}

/// 获取系统可用的shell列表
///
#[tauri::command]
pub async fn terminal_get_available_shells() -> TauriApiResult<Vec<ShellInfo>> {
    debug!("获取可用shell列表");

    let shells = ShellManager::detect_available_shells();

    debug!("获取可用shell列表成功: count={}", shells.len());

    for shell in &shells {
        debug!(
            "可用shell: {} -> {} ({})",
            shell.name, shell.path, shell.display_name
        );
    }

    Ok(api_success!(shells))
}

/// 获取系统默认shell信息
///
#[tauri::command]
pub async fn terminal_get_default_shell() -> TauriApiResult<ShellInfo> {
    debug!("获取系统默认shell");

    let default_shell = ShellManager::terminal_get_default_shell();

    debug!(
        "获取默认shell成功: {} -> {}",
        default_shell.name, default_shell.path
    );

    debug!(
        "默认shell详情: name={}, path={}, display_name={}",
        default_shell.name, default_shell.path, default_shell.display_name
    );

    Ok(api_success!(default_shell))
}

/// 验证shell路径是否有效
///
#[tauri::command]
pub async fn terminal_validate_shell_path(path: String) -> TauriApiResult<bool> {
    validate_non_empty_string(&path, "Shell路径")
        .context("Shell路径验证失败")
        .to_tauri()?;

    let is_valid = ShellManager::validate_shell(&path);

    debug!("验证shell路径: path={}, valid={}", path, is_valid);
    debug!("Shell路径验证详情: {} -> {}", path, is_valid);
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
    debug!(
        "使用指定shell创建终端: {:?}, 大小: {}x{}",
        shell_name, cols, rows
    );

    if let Err(_) = validate_terminal_size(rows, cols) {
        return Ok(api_error!("shell.terminal_size_invalid"));
    }

    let shell_info = match shell_name {
        Some(name) => {
            debug!("查找指定shell: {}", name);
            match ShellManager::terminal_find_shell_by_name(&name) {
                Some(shell) => shell,
                None => {
                    error!("未找到指定shell: {}", name);
                    return Ok(api_error!("shell.shell_not_found"));
                }
            }
        }
        None => {
            debug!("使用默认shell");
            ShellManager::terminal_get_default_shell()
        }
    };

    debug!("使用shell: {} ({})", shell_info.name, shell_info.path);

    let mux = get_mux();
    let size = PtySize::new(rows, cols);

    let shell_config = ShellConfig::with_shell(&shell_info);
    let config = TerminalConfig::with_shell(shell_config);

    // 使用配置创建面板
    match mux.create_pane_with_config(size, &config).await {
        Ok(pane_id) => {
            debug!(
                "终端创建成功: ID={}, shell={}, 新的面板数量: {}",
                pane_id.as_u32(),
                config.shell_config.program,
                mux.pane_count()
            );
            Ok(api_success!(pane_id.as_u32()))
        }
        Err(_) => {
            error!("创建终端失败");
            Ok(api_error!("shell.create_terminal_failed"))
        }
    }
}

/// 根据名称查找shell
///
#[tauri::command]
pub async fn terminal_find_shell_by_name(shell_name: String) -> TauriApiResult<Option<ShellInfo>> {
    debug!("查找shell: {}", shell_name);

    if shell_name.trim().is_empty() {
        return Ok(api_error!("common.empty_content"));
    }

    match std::panic::catch_unwind(|| ShellManager::terminal_find_shell_by_name(&shell_name)) {
        Ok(shell_info) => {
            match &shell_info {
                Some(shell) => {
                    debug!("查找shell成功: name={}, path={}", shell.name, shell.path);
                    debug!("找到shell详情: {:?}", shell);
                }
                None => {
                    debug!("未找到shell: name={}", shell_name);
                }
            }

            Ok(api_success!(shell_info))
        }
        Err(_) => Ok(api_error!("shell.find_shell_failed")),
    }
}

/// 根据路径查找shell
///
#[tauri::command]
pub async fn terminal_find_shell_by_path(shell_path: String) -> TauriApiResult<Option<ShellInfo>> {
    debug!("根据路径查找shell: {}", shell_path);

    if shell_path.trim().is_empty() {
        return Ok(api_error!("common.empty_content"));
    }

    match std::panic::catch_unwind(|| ShellManager::terminal_find_shell_by_path(&shell_path)) {
        Ok(shell_info) => {
            match &shell_info {
                Some(shell) => {
                    debug!(
                        "根据路径查找shell成功: path={}, name={}",
                        shell.path, shell.name
                    );
                    debug!("找到shell详情: {:?}", shell);
                }
                None => {
                    debug!("根据路径未找到shell: path={}", shell_path);
                }
            }

            Ok(api_success!(shell_info))
        }
        Err(_) => Ok(api_error!("shell.find_shell_failed")),
    }
}

/// 获取Shell管理器统计信息
///
#[tauri::command]
pub async fn terminal_get_shell_stats() -> TauriApiResult<ShellManagerStats> {
    debug!("获取Shell管理器统计信息");

    match std::panic::catch_unwind(|| {
        let manager = ShellManager::new();
        manager.get_stats().clone()
    }) {
        Ok(stats) => {
            debug!(
                "获取Shell统计信息成功: available={}, default={:?}",
                stats.available_shells, stats.default_shell
            );

            debug!("Shell统计详情: {:?}", stats);
            Ok(api_success!(stats))
        }
        Err(_) => {
            let error_msg = "获取Shell统计信息时发生系统错误";
            error!("获取Shell统计信息失败: {}", error_msg);
            Ok(api_error!("shell.get_buffer_failed"))
        }
    }
}

/// 初始化Shell管理器
///
#[tauri::command]
pub async fn terminal_initialize_shell_manager() -> TauriApiResult<EmptyData> {
    debug!("初始化Shell管理器");

    // ShellManager 不需要单独的初始化方法，创建实例时自动初始化
    match std::panic::catch_unwind(|| {
        ShellManager::new();
    }) {
        Ok(()) => {
            debug!("Shell管理器初始化成功");
            Ok(api_success!())
        }
        Err(_) => {
            let error_msg = "Shell管理器初始化失败";
            error!("{}", error_msg);
            Ok(api_error!("shell.get_buffer_failed"))
        }
    }
}

/// 验证Shell管理器状态
///
#[tauri::command]
pub async fn terminal_validate_shell_manager() -> TauriApiResult<EmptyData> {
    debug!("验证Shell管理器状态");

    // ShellManager 不需要单独的验证方法，创建实例时自动验证
    match std::panic::catch_unwind(|| {
        let manager = ShellManager::new();
        manager.get_stats();
    }) {
        Ok(()) => {
            debug!("Shell管理器验证成功");
            Ok(api_success!())
        }
        Err(_) => {
            let error_msg = "Shell管理器验证失败";
            error!("{}", error_msg);
            Ok(api_error!("shell.get_buffer_failed"))
        }
    }
}
