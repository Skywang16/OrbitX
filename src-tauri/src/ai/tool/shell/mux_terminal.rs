/*!
 * 终端模块的Tauri命令接口
 *
 * 统一的终端命令处理规范：
 * 1. 参数顺序：业务参数在前，state参数在后
 * 2. 异步处理：所有命令都是async，统一错误转换
 * 3. 日志记录：每个命令都记录调用和结果日志
 * 4. 状态管理：统一使用TerminalState访问各组件
 * 5. 错误处理：使用TerminalError统一错误类型
 */

// 注意：移除未使用的 anyhow 导入，因为所有 Tauri 命令都直接返回 Result<T, String>

use tauri::{AppHandle, Emitter, Runtime, State};
use tracing::{debug, error, warn};

use crate::mux::{
    get_mux, ErrorHandler, MuxNotification, PaneId, PtySize, ShellConfig, ShellInfo, ShellManager,
    ShellManagerStats, TerminalConfig, TerminalError, TerminalResult,
};

/// 统一的错误处理函数（使用新的错误系统）
fn handle_terminal_error(error: TerminalError) -> String {
    ErrorHandler::to_tauri_error(error)
}

/// 参数验证辅助函数（使用新的错误系统）
fn validate_terminal_size(rows: u16, cols: u16) -> TerminalResult<()> {
    if rows == 0 || cols == 0 {
        return Err(TerminalError::validation(
            "terminal_size",
            format!("终端尺寸不能为0 (当前: {}x{})", cols, rows),
        ));
    }
    Ok(())
}

fn validate_non_empty_string(value: &str, field_name: &str) -> TerminalResult<()> {
    if value.trim().is_empty() {
        return Err(TerminalError::validation(
            field_name,
            format!("{}不能为空", field_name),
        ));
    }
    Ok(())
}

/// 终端状态管理
///
/// 统一状态管理规范：
/// 1. 使用全局单例模式访问TerminalMux
/// 2. 提供统一的初始化和验证方法
/// 3. 包含配置验证和错误处理
/// 4. 支持组件间的依赖注入
pub struct TerminalState {
    // 注意：我们使用全局单例，所以这里不需要存储 Arc<TerminalMux>
    // 但保留这个结构体以便将来扩展其他状态
    _placeholder: (),
}

impl TerminalState {
    /// 统一的初始化方法
    pub fn new() -> Result<Self, String> {
        let state = Self { _placeholder: () };

        // 验证状态完整性
        state.validate()?;

        Ok(state)
    }

    /// 验证状态完整性
    pub fn validate(&self) -> Result<(), String> {
        let mux = get_mux();

        // 验证Mux实例是否可访问
        mux.pane_count();

        Ok(())
    }
}

/// 创建新终端会话
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn create_terminal<R: Runtime>(
    rows: u16,
    cols: u16,
    cwd: Option<String>,
    _app: AppHandle<R>,
    _state: State<'_, TerminalState>,
) -> Result<u32, String> {
    debug!("创建终端会话: {}x{}, 初始目录: {:?}", cols, rows, cwd);
    debug!("当前Mux状态 - 面板数量: {}", get_mux().pane_count());

    // 参数验证
    validate_terminal_size(rows, cols).map_err(handle_terminal_error)?;

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
            let dir_info = working_dir
                .map(|dir| format!(", 初始目录: {}", dir))
                .unwrap_or_default();

            debug!(
                "终端创建成功: ID={}{}, 新的面板数量: {}",
                pane_id.as_u32(),
                dir_info,
                mux.pane_count()
            );
            Ok(pane_id.as_u32())
        }
        Err(e) => Err(handle_terminal_error(TerminalError::pane(
            "创建终端",
            0,
            e.to_string(),
        ))),
    }
}

/// 向终端写入数据
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn write_to_terminal(
    pane_id: u32,
    data: String,
    _state: State<'_, TerminalState>,
) -> Result<(), String> {
    debug!(
        "写入终端数据: ID={}, 数据长度={}, 数据预览: {:?}",
        pane_id,
        data.len(),
        &data[..std::cmp::min(50, data.len())]
    );

    // 参数验证
    if data.is_empty() {
        return Err(handle_terminal_error(TerminalError::validation(
            "terminal_data",
            format!("写入数据不能为空 (面板ID: {})", pane_id),
        )));
    }

    let mux = get_mux();
    let pane_id_obj = PaneId::from(pane_id);

    match mux.write_to_pane(pane_id_obj, data.as_bytes()) {
        Ok(_) => {
            debug!("写入终端成功: ID={}", pane_id);
            Ok(())
        }
        Err(e) => Err(handle_terminal_error(TerminalError::pane(
            "写入终端",
            pane_id,
            e.to_string(),
        ))),
    }
}

/// 调整终端大小
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn resize_terminal(
    pane_id: u32,
    rows: u16,
    cols: u16,
    _state: State<'_, TerminalState>,
) -> Result<(), String> {
    debug!("调整终端大小: ID={}, 大小={}x{}", pane_id, cols, rows);

    // 参数验证
    validate_terminal_size(rows, cols).map_err(handle_terminal_error)?;

    let mux = get_mux();
    let pane_id_obj = PaneId::from(pane_id);
    let size = PtySize::new(rows, cols);

    match mux.resize_pane(pane_id_obj, size) {
        Ok(_) => {
            debug!("调整终端大小成功: ID={}", pane_id);
            Ok(())
        }
        Err(e) => Err(handle_terminal_error(TerminalError::pane(
            "调整终端大小",
            pane_id,
            e.to_string(),
        ))),
    }
}

/// 关闭终端会话
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
/// - 防御性编程：优雅处理面板不存在的情况
#[tauri::command]
pub async fn close_terminal(pane_id: u32, _state: State<'_, TerminalState>) -> Result<(), String> {
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
            Ok(())
        }
        Err(e) => {
            // 检查是否是"面板不存在"的错误
            let error_str = e.to_string();
            if error_str.contains("not found") || error_str.contains("不存在") {
                // 面板不存在，认为操作成功
                warn!("尝试关闭不存在的面板: ID={}, 可能已被其他操作关闭", pane_id);
                Ok(())
            } else {
                // 其他错误，返回失败
                Err(handle_terminal_error(TerminalError::pane(
                    "关闭终端",
                    pane_id,
                    e.to_string(),
                )))
            }
        }
    }
}

/// 获取终端列表（调试用）
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn list_terminals(_state: State<'_, TerminalState>) -> Result<Vec<u32>, String> {
    debug!("获取终端列表");

    let mux = get_mux();
    let pane_ids: Vec<u32> = mux.list_panes().into_iter().map(|id| id.as_u32()).collect();

    debug!("获取终端列表成功: count={}", pane_ids.len());
    debug!("当前终端列表: {:?}", pane_ids);
    Ok(pane_ids)
}

/// 将 MuxNotification 转换为 Tauri 事件负载
/// 将 MuxNotification 转换为 Tauri 事件负载
///
/// 统一事件命名规范：
/// - 使用下划线格式 (terminal_output)
/// - 确保事件命名的一致性
fn notification_to_tauri_payload(
    notification: &MuxNotification,
) -> (&'static str, serde_json::Value) {
    use crate::mux::{
        TerminalClosedEvent, TerminalCreatedEvent, TerminalExitEvent, TerminalOutputEvent,
        TerminalResizedEvent,
    };

    match notification {
        MuxNotification::PaneOutput { pane_id, data } => {
            let event = TerminalOutputEvent {
                pane_id: *pane_id,
                data: String::from_utf8_lossy(data).to_string(),
            };
            ("terminal_output", serde_json::to_value(event).unwrap())
        }
        MuxNotification::PaneAdded(pane_id) => {
            let event = TerminalCreatedEvent { pane_id: *pane_id };
            ("terminal_created", serde_json::to_value(event).unwrap())
        }
        MuxNotification::PaneRemoved(pane_id) => {
            let event = TerminalClosedEvent { pane_id: *pane_id };
            ("terminal_closed", serde_json::to_value(event).unwrap())
        }
        MuxNotification::PaneResized { pane_id, size } => {
            let event = TerminalResizedEvent {
                pane_id: *pane_id,
                rows: size.rows,
                cols: size.cols,
            };
            ("terminal_resized", serde_json::to_value(event).unwrap())
        }
        MuxNotification::PaneExited { pane_id, exit_code } => {
            let event = TerminalExitEvent {
                pane_id: *pane_id,
                exit_code: *exit_code,
            };
            ("terminal_exit", serde_json::to_value(event).unwrap())
        }
        MuxNotification::PaneCwdChanged { pane_id, cwd } => {
            // 暂时忽略CWD变化事件，或者可以创建相应的事件结构
            (
                "pane_cwd_changed",
                serde_json::json!({
                    "pane_id": pane_id,
                    "cwd": cwd
                }),
            )
        }
    }
}

/// 输出处理器trait，用于解耦模块依赖
pub trait OutputProcessor: Send + Sync {
    fn process_output(&self, pane_id: u32, data: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn cleanup_pane(&self, pane_id: u32) -> Result<(), Box<dyn std::error::Error>>;
}

/// 默认输出处理器实现
struct DefaultOutputProcessor;

impl OutputProcessor for DefaultOutputProcessor {
    fn process_output(&self, pane_id: u32, data: &str) -> Result<(), Box<dyn std::error::Error>> {
        use crate::completion::output_analyzer::OutputAnalyzer;
        OutputAnalyzer::global().analyze_output(pane_id, data)?;
        Ok(())
    }

    fn cleanup_pane(&self, pane_id: u32) -> Result<(), Box<dyn std::error::Error>> {
        use crate::completion::output_analyzer::OutputAnalyzer;
        OutputAnalyzer::global().cleanup_pane_buffer(pane_id)?;
        Ok(())
    }
}

/// 设置 Tauri 事件集成（解耦版本）
pub fn setup_tauri_integration<R: Runtime>(app_handle: AppHandle<R>) {
    setup_tauri_integration_with_processor(app_handle, Box::new(DefaultOutputProcessor));
}

/// 设置 Tauri 事件集成，支持自定义输出处理器
pub fn setup_tauri_integration_with_processor<R: Runtime>(
    app_handle: AppHandle<R>,
    output_processor: Box<dyn OutputProcessor>,
) {
    let mux = get_mux();

    let subscriber_id = mux.subscribe(move |notification| {
        let (event_name, payload) = notification_to_tauri_payload(&notification);

        // 处理输出分析（解耦）
        match &notification {
            crate::mux::MuxNotification::PaneOutput { pane_id, data } => {
                let output_text = String::from_utf8_lossy(data);
                if let Err(e) = output_processor.process_output(pane_id.as_u32(), &output_text) {
                    debug!("输出处理失败: {}", e);
                }
            }
            crate::mux::MuxNotification::PaneRemoved(pane_id) => {
                // 清理面板缓冲区
                if let Err(e) = output_processor.cleanup_pane(pane_id.as_u32()) {
                    debug!("清理面板失败: {}", e);
                }
            }
            _ => {}
        }

        // 发送Tauri事件
        match app_handle.emit(event_name, payload.clone()) {
            Ok(_) => debug!("Tauri 事件发送成功: {}", event_name),
            Err(e) => error!("Tauri 事件发送失败: {}, 错误: {}", event_name, e),
        }

        true
    });

    debug!("Tauri 事件集成完成，订阅者ID: {}", subscriber_id);

    let mux_arc = get_mux();
    mux_arc.start_notification_processor();
    debug!("通知处理器已启动");
}

// === Shell 管理命令 ===

/// 获取终端缓冲区内容
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn get_terminal_buffer(pane_id: u32) -> Result<String, String> {
    debug!("开始获取终端缓冲区内容: ID={}", pane_id);

    use crate::completion::output_analyzer::OutputAnalyzer;

    match OutputAnalyzer::global().get_pane_buffer(pane_id) {
        Ok(content) => {
            debug!(
                "获取终端缓冲区成功: ID={}, 内容长度={}",
                pane_id,
                content.len()
            );
            Ok(content)
        }
        Err(e) => {
            let error_msg = format!("获取终端缓冲区失败: ID={}, 错误: {}", pane_id, e);
            error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 设置终端缓冲区内容
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn set_terminal_buffer(pane_id: u32, content: String) -> Result<(), String> {
    debug!(
        "开始设置终端缓冲区内容: ID={}, 内容长度={}",
        pane_id,
        content.len()
    );

    use crate::completion::output_analyzer::OutputAnalyzer;

    match OutputAnalyzer::global().set_pane_buffer(pane_id, content) {
        Ok(_) => {
            debug!("设置终端缓冲区成功: ID={}", pane_id);
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("设置终端缓冲区失败: ID={}, 错误: {}", pane_id, e);
            error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 获取系统可用的shell列表
///
/// 统一命令处理规范：
/// - 参数顺序：无业务参数，state参数在后（如果需要）
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn get_available_shells() -> Result<Vec<ShellInfo>, String> {
    debug!("获取可用shell列表");

    let shells = ErrorHandler::handle_panic("获取可用shell列表", || {
        ShellManager::detect_available_shells()
    })
    .map_err(handle_terminal_error)?;

    debug!("获取可用shell列表成功: count={}", shells.len());

    for shell in &shells {
        debug!(
            "可用shell: {} -> {} ({})",
            shell.name, shell.path, shell.display_name
        );
    }

    Ok(shells)
}

/// 获取系统默认shell信息
///
/// 统一命令处理规范：
/// - 参数顺序：无业务参数，state参数在后（如果需要）
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn get_default_shell() -> Result<ShellInfo, String> {
    debug!("获取系统默认shell");

    let default_shell =
        ErrorHandler::handle_panic("获取默认shell", || ShellManager::get_default_shell())
            .map_err(handle_terminal_error)?;

    debug!(
        "获取默认shell成功: {} -> {}",
        default_shell.name, default_shell.path
    );

    debug!(
        "默认shell详情: name={}, path={}, display_name={}",
        default_shell.name, default_shell.path, default_shell.display_name
    );

    Ok(default_shell)
}

/// 验证shell路径是否有效
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state参数在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn validate_shell_path(path: String) -> Result<bool, String> {
    // 参数验证
    validate_non_empty_string(&path, "Shell路径").map_err(handle_terminal_error)?;

    let is_valid =
        ErrorHandler::handle_panic("验证shell路径", || ShellManager::validate_shell(&path))
            .map_err(handle_terminal_error)?;

    debug!("验证shell路径: path={}, valid={}", path, is_valid);
    debug!("Shell路径验证详情: {} -> {}", path, is_valid);
    Ok(is_valid)
}

/// 使用指定shell创建终端
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn create_terminal_with_shell<R: Runtime>(
    shell_name: Option<String>,
    rows: u16,
    cols: u16,
    _app: AppHandle<R>,
    _state: State<'_, TerminalState>,
) -> Result<u32, String> {
    // 参数验证
    if rows == 0 || cols == 0 {
        let error_msg = format!(
            "终端尺寸验证失败: 终端尺寸不能为0 (当前: {}x{})",
            cols, rows
        );
        error!("创建终端失败: {}", error_msg);
        return Err(error_msg);
    }

    let shell_info = match shell_name {
        Some(name) => {
            debug!("使用指定shell创建终端: {}, 大小: {}x{}", name, cols, rows);
            ShellManager::find_shell_by_name(&name)
                .ok_or_else(|| format!("Shell查找错误: 未找到shell '{}'", name))?
        }
        None => {
            debug!("使用默认shell创建终端, 大小: {}x{}", cols, rows);
            ShellManager::get_default_shell()
        }
    };

    debug!("使用shell: {} ({})", shell_info.name, shell_info.path);

    let mux = get_mux();
    let size = PtySize::new(rows, cols);

    // 创建 ShellConfig 而不是直接传递 ShellInfo
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
            Ok(pane_id.as_u32())
        }
        Err(e) => {
            let error_msg = format!("创建终端失败: {}", e);
            error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 根据名称查找shell
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state参数在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn find_shell_by_name(shell_name: String) -> Result<Option<ShellInfo>, String> {
    debug!("查找shell: {}", shell_name);

    // 参数验证
    if shell_name.trim().is_empty() {
        let error_msg = "Shell名称验证失败: Shell名称不能为空";
        error!("查找shell失败: {}", error_msg);
        return Err(error_msg.to_string());
    }

    match std::panic::catch_unwind(|| ShellManager::find_shell_by_name(&shell_name)) {
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

            Ok(shell_info)
        }
        Err(_) => {
            let error_msg = format!("查找shell时发生系统错误: {shell_name}");
            error!("查找shell失败: {}", error_msg);
            Err(error_msg)
        }
    }
}

/// 根据路径查找shell
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state参数在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn find_shell_by_path(shell_path: String) -> Result<Option<ShellInfo>, String> {
    debug!("根据路径查找shell: {}", shell_path);

    // 参数验证
    if shell_path.trim().is_empty() {
        let error_msg = "Shell路径验证失败: Shell路径不能为空";
        error!("根据路径查找shell失败: {}", error_msg);
        return Err(error_msg.to_string());
    }

    match std::panic::catch_unwind(|| ShellManager::find_shell_by_path(&shell_path)) {
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

            Ok(shell_info)
        }
        Err(_) => {
            let error_msg = format!("根据路径查找shell时发生系统错误: {shell_path}");
            error!("根据路径查找shell失败: {}", error_msg);
            Err(error_msg)
        }
    }
}

/// 获取Shell管理器统计信息
///
/// 统一命令处理规范：
/// - 参数顺序：无业务参数，state参数在后（如果需要）
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn get_shell_stats() -> Result<ShellManagerStats, String> {
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
            Ok(stats)
        }
        Err(_) => {
            let error_msg = "获取Shell统计信息时发生系统错误";
            error!("获取Shell统计信息失败: {}", error_msg);
            Err(error_msg.to_string())
        }
    }
}

/// 初始化Shell管理器
///
/// 统一命令处理规范：
/// - 参数顺序：无业务参数，state参数在后（如果需要）
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn initialize_shell_manager() -> Result<(), String> {
    debug!("初始化Shell管理器");

    // ShellManager 不需要单独的初始化方法，创建实例时自动初始化
    match std::panic::catch_unwind(|| {
        ShellManager::new();
    }) {
        Ok(()) => {
            debug!("Shell管理器初始化成功");
            Ok(())
        }
        Err(_) => {
            let error_msg = "Shell管理器初始化失败";
            error!("{}", error_msg);
            Err(error_msg.to_string())
        }
    }
}

/// 验证Shell管理器状态
///
/// 统一命令处理规范：
/// - 参数顺序：无业务参数，state参数在后（如果需要）
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn validate_shell_manager() -> Result<(), String> {
    debug!("验证Shell管理器状态");

    // ShellManager 不需要单独的验证方法，创建实例时自动验证
    match std::panic::catch_unwind(|| {
        let manager = ShellManager::new();
        manager.get_stats();
    }) {
        Ok(()) => {
            debug!("Shell管理器验证成功");
            Ok(())
        }
        Err(_) => {
            let error_msg = "Shell管理器验证失败";
            error!("{}", error_msg);
            Err(error_msg.to_string())
        }
    }
}
