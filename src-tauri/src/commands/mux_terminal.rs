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
use std::time::Instant;
use tauri::{AppHandle, Emitter, Runtime, State};
use tracing::{debug, error, info};

use crate::mux::{
    get_mux, MuxNotification, PaneId, PtySize, ShellConfig, ShellInfo, ShellManager,
    ShellManagerStats, TerminalConfig,
};
// 注意：不再需要 AppResult，所有 Tauri 命令都直接返回 Result<T, String>

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
        info!("开始初始化终端状态");

        // 初始化全局 Mux 实例
        let mux = get_mux();
        info!("终端状态初始化完成，Mux 面板数量: {}", mux.pane_count());

        let state = Self { _placeholder: () };

        // 验证状态完整性
        state.validate()?;

        Ok(state)
    }

    /// 验证状态完整性
    pub fn validate(&self) -> Result<(), String> {
        let mux = get_mux();

        // 验证Mux实例是否可访问
        let _pane_count = mux.pane_count();

        info!("终端状态验证通过");
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
    let start_time = Instant::now();
    info!("开始创建终端会话: {}x{}, 初始目录: {:?}", cols, rows, cwd);
    debug!("当前Mux状态 - 面板数量: {}", get_mux().pane_count());

    // 参数验证
    if rows == 0 || cols == 0 {
        let error_msg = format!("参数验证错误: 终端尺寸不能为0 (当前: {}x{})", cols, rows);
        error!("创建终端失败: {}", error_msg);
        return Err(error_msg);
    }

    let mux = get_mux();
    let size = PtySize::new(rows, cols);

    // 如果指定了初始目录，使用配置创建终端
    if let Some(working_dir) = cwd {
        let shell_config = ShellConfig {
            program: ShellManager::get_default_shell().path,
            args: Vec::new(),
            working_directory: Some(working_dir.clone().into()),
            env: None,
        };
        let config = TerminalConfig::with_shell(shell_config);

        match mux.create_pane_with_config(size, &config).await {
            Ok(pane_id) => {
                let processing_time = start_time.elapsed().as_millis();
                info!(
                    "终端创建成功: ID={}, 初始目录: {}, 新的面板数量: {}, 耗时: {}ms",
                    pane_id.as_u32(),
                    working_dir,
                    mux.pane_count(),
                    processing_time
                );
                Ok(pane_id.as_u32())
            }
            Err(e) => {
                let error_msg = format!("创建终端失败: {}", e);
                error!("{}", error_msg);
                Err(error_msg)
            }
        }
    } else {
        // 没有指定初始目录，使用默认方式创建
        match mux.create_pane(size).await {
            Ok(pane_id) => {
                let processing_time = start_time.elapsed().as_millis();
                info!(
                    "终端创建成功: ID={}, 新的面板数量: {}, 耗时: {}ms",
                    pane_id.as_u32(),
                    mux.pane_count(),
                    processing_time
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
    let start_time = Instant::now();
    debug!(
        "开始写入终端数据: ID={}, 数据长度={}, 数据预览: {:?}",
        pane_id,
        data.len(),
        &data[..std::cmp::min(50, data.len())]
    );

    // 参数验证
    if data.is_empty() {
        let error_msg = format!("终端数据验证失败: 写入数据不能为空 (面板ID: {})", pane_id);
        error!("写入终端失败: {}", error_msg);
        return Err(error_msg);
    }

    let mux = get_mux();
    let pane_id = PaneId::from(pane_id);

    match mux.write_to_pane(pane_id, data.as_bytes()) {
        Ok(_) => {
            let processing_time = start_time.elapsed().as_millis();
            debug!(
                "写入终端成功: ID={}, 耗时: {}ms",
                pane_id.as_u32(),
                processing_time
            );
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("写入终端失败: ID={}, 错误: {}", pane_id.as_u32(), e);
            error!("{}", error_msg);
            Err(error_msg)
        }
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
    let start_time = Instant::now();
    info!("开始调整终端大小: ID={}, 大小={}x{}", pane_id, cols, rows);

    // 参数验证
    if rows == 0 || cols == 0 {
        let error_msg = format!(
            "终端尺寸验证失败: 终端尺寸不能为0 (面板ID: {}, 当前: {}x{})",
            pane_id, cols, rows
        );
        error!("调整终端大小失败: {}", error_msg);
        return Err(error_msg);
    }

    let mux = get_mux();
    let pane_id = PaneId::from(pane_id);
    let size = PtySize::new(rows, cols);

    match mux.resize_pane(pane_id, size) {
        Ok(_) => {
            let processing_time = start_time.elapsed().as_millis();
            info!(
                "调整终端大小成功: ID={}, 耗时: {}ms",
                pane_id.as_u32(),
                processing_time
            );
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("调整终端大小失败: ID={}, 错误: {}", pane_id.as_u32(), e);
            error!("{}", error_msg);
            Err(error_msg)
        }
    }
}

/// 关闭终端会话
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn close_terminal(pane_id: u32, _state: State<'_, TerminalState>) -> Result<(), String> {
    let start_time = Instant::now();
    info!(
        "开始关闭终端会话: ID={}, 当前面板数量: {}",
        pane_id,
        get_mux().pane_count()
    );

    let mux = get_mux();
    let pane_id = PaneId::from(pane_id);

    match mux.remove_pane(pane_id) {
        Ok(_) => {
            let processing_time = start_time.elapsed().as_millis();
            info!(
                "关闭终端成功: ID={}, 剩余面板数量: {}, 耗时: {}ms",
                pane_id.as_u32(),
                mux.pane_count(),
                processing_time
            );
            Ok(())
        }
        Err(e) => {
            let error_msg = format!("关闭终端失败: ID={}, 错误: {}", pane_id.as_u32(), e);
            error!("{}", error_msg);
            Err(error_msg)
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
    let start_time = Instant::now();
    info!("开始获取终端列表");

    let mux = get_mux();
    let pane_ids: Vec<u32> = mux.list_panes().into_iter().map(|id| id.as_u32()).collect();

    let processing_time = start_time.elapsed().as_millis();
    info!(
        "获取终端列表成功: count={}, 耗时: {}ms",
        pane_ids.len(),
        processing_time
    );
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
    }
}

/// 设置 Tauri 事件集成
pub fn setup_tauri_integration<R: Runtime>(app_handle: AppHandle<R>) {
    use crate::completion::output_analyzer::OutputAnalyzer;

    let mux = get_mux();

    let subscriber_id = mux.subscribe(move |notification| {
        let (event_name, payload) = notification_to_tauri_payload(&notification);

        // 处理输出分析
        match &notification {
            crate::mux::MuxNotification::PaneOutput { pane_id, data } => {
                let output_text = String::from_utf8_lossy(data);
                if let Err(e) = OutputAnalyzer::global().analyze_output(pane_id.as_u32(), &output_text) {
                    debug!("输出分析失败: {}", e);
                }
            }
            crate::mux::MuxNotification::PaneRemoved(pane_id) => {
                // 清理面板缓冲区
                if let Err(e) = OutputAnalyzer::global().clear_pane_buffer(pane_id.as_u32()) {
                    debug!("清理面板缓冲区失败: {}", e);
                }
            }
            _ => {}
        }

        match app_handle.emit(event_name, payload.clone()) {
            Ok(_) => debug!("Tauri 事件发送成功: {}", event_name),
            Err(e) => error!("Tauri 事件发送失败: {}, 错误: {}", event_name, e),
        }

        true
    });

    info!("Tauri 事件集成完成，订阅者ID: {}", subscriber_id);

    let mux_arc = get_mux();
    mux_arc.start_notification_processor();
    info!("通知处理器已启动");
}

// === Shell 管理命令 ===

/// 获取系统可用的shell列表
///
/// 统一命令处理规范：
/// - 参数顺序：无业务参数，state参数在后（如果需要）
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn get_available_shells() -> Result<Vec<ShellInfo>, String> {
    let start_time = Instant::now();
    info!("开始获取可用shell列表");

    match std::panic::catch_unwind(ShellManager::detect_available_shells) {
        Ok(shells) => {
            let processing_time = start_time.elapsed().as_millis();
            info!(
                "获取可用shell列表成功: count={}, 耗时: {}ms",
                shells.len(),
                processing_time
            );

            for shell in &shells {
                debug!(
                    "可用shell: {} -> {} ({})",
                    shell.name, shell.path, shell.display_name
                );
            }

            Ok(shells)
        }
        Err(_) => {
            let error_msg = "检测可用shell时发生系统错误";
            error!("获取可用shell列表失败: {}", error_msg);
            Err(format!("Shell检测错误: {}", error_msg))
        }
    }
}

/// 获取系统默认shell信息
///
/// 统一命令处理规范：
/// - 参数顺序：无业务参数，state参数在后（如果需要）
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn get_default_shell() -> Result<ShellInfo, String> {
    let start_time = Instant::now();
    info!("开始获取系统默认shell");

    match std::panic::catch_unwind(ShellManager::get_default_shell) {
        Ok(default_shell) => {
            let processing_time = start_time.elapsed().as_millis();
            info!(
                "获取默认shell成功: {} -> {}, 耗时: {}ms",
                default_shell.name, default_shell.path, processing_time
            );

            debug!(
                "默认shell详情: name={}, path={}, display_name={}",
                default_shell.name, default_shell.path, default_shell.display_name
            );

            Ok(default_shell)
        }
        Err(_) => {
            let error_msg = "获取默认shell时发生系统错误";
            error!("获取默认shell失败: {}", error_msg);
            Err(error_msg.to_string())
        }
    }
}

/// 验证shell路径是否有效
///
/// 统一命令处理规范：
/// - 参数顺序：业务参数在前，state参数在后
/// - 日志记录：记录操作开始、成功和失败
/// - 错误处理：统一转换为String类型
#[tauri::command]
pub async fn validate_shell_path(path: String) -> Result<bool, String> {
    let start_time = Instant::now();
    info!("开始验证shell路径: {}", path);

    // 参数验证
    if path.trim().is_empty() {
        let error_msg = "Shell路径验证失败: Shell路径不能为空";
        error!("验证shell路径失败: {}", error_msg);
        return Err(error_msg.to_string());
    }

    match std::panic::catch_unwind(|| ShellManager::validate_shell(&path)) {
        Ok(is_valid) => {
            let processing_time = start_time.elapsed().as_millis();
            info!(
                "验证shell路径完成: path={}, valid={}, 耗时: {}ms",
                path, is_valid, processing_time
            );

            debug!("Shell路径验证详情: {} -> {}", path, is_valid);
            Ok(is_valid)
        }
        Err(_) => {
            let error_msg = format!("验证shell路径时发生系统错误: {path}");
            error!("验证shell路径失败: {}", error_msg);
            Err(error_msg)
        }
    }
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
    let start_time = Instant::now();

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
            info!(
                "开始使用指定shell创建终端: {}, 大小: {}x{}",
                name, cols, rows
            );
            ShellManager::find_shell_by_name(&name)
                .ok_or_else(|| format!("Shell查找错误: 未找到shell '{}'", name))
                .map_err(|e| e.to_string())?
        }
        None => {
            info!("开始使用默认shell创建终端, 大小: {}x{}", cols, rows);
            ShellManager::get_default_shell()
        }
    };

    debug!("使用shell: {} ({})", shell_info.name, shell_info.path);

    let mux = get_mux();
    let size = PtySize::new(rows, cols);

    // 创建 ShellConfig 而不是直接传递 ShellInfo
    let shell_config = ShellConfig {
        program: shell_info.path.clone(),
        args: Vec::new(),
        working_directory: None,
        env: None,
    };
    let config = TerminalConfig::with_shell(shell_config);

    // 使用配置创建面板
    match mux.create_pane_with_config(size, &config).await {
        Ok(pane_id) => {
            let processing_time = start_time.elapsed().as_millis();
            info!(
                "终端创建成功: ID={}, shell={}, 新的面板数量: {}, 耗时: {}ms",
                pane_id.as_u32(),
                config.shell_config.program,
                mux.pane_count(),
                processing_time
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
    let start_time = Instant::now();
    info!("开始查找shell: {}", shell_name);

    // 参数验证
    if shell_name.trim().is_empty() {
        let error_msg = "Shell名称验证失败: Shell名称不能为空";
        error!("查找shell失败: {}", error_msg);
        return Err(error_msg.to_string());
    }

    match std::panic::catch_unwind(|| ShellManager::find_shell_by_name(&shell_name)) {
        Ok(shell_info) => {
            let processing_time = start_time.elapsed().as_millis();

            match &shell_info {
                Some(shell) => {
                    info!(
                        "查找shell成功: name={}, path={}, 耗时: {}ms",
                        shell.name, shell.path, processing_time
                    );
                    debug!("找到shell详情: {:?}", shell);
                }
                None => {
                    info!(
                        "未找到shell: name={}, 耗时: {}ms",
                        shell_name, processing_time
                    );
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
    let start_time = Instant::now();
    info!("开始根据路径查找shell: {}", shell_path);

    // 参数验证
    if shell_path.trim().is_empty() {
        let error_msg = "Shell路径验证失败: Shell路径不能为空";
        error!("根据路径查找shell失败: {}", error_msg);
        return Err(error_msg.to_string());
    }

    match std::panic::catch_unwind(|| ShellManager::find_shell_by_path(&shell_path)) {
        Ok(shell_info) => {
            let processing_time = start_time.elapsed().as_millis();

            match &shell_info {
                Some(shell) => {
                    info!(
                        "根据路径查找shell成功: path={}, name={}, 耗时: {}ms",
                        shell.path, shell.name, processing_time
                    );
                    debug!("找到shell详情: {:?}", shell);
                }
                None => {
                    info!(
                        "根据路径未找到shell: path={}, 耗时: {}ms",
                        shell_path, processing_time
                    );
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
    let start_time = Instant::now();
    info!("开始获取Shell管理器统计信息");

    match std::panic::catch_unwind(|| {
        let manager = ShellManager::new();
        manager.get_stats().clone()
    }) {
        Ok(stats) => {
            let processing_time = start_time.elapsed().as_millis();
            info!(
                "获取Shell统计信息成功: available={}, default={:?}, 耗时: {}ms",
                stats.available_shells, stats.default_shell, processing_time
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
    let start_time = Instant::now();
    info!("开始初始化Shell管理器");

    // ShellManager 不需要单独的初始化方法，创建实例时自动初始化
    match std::panic::catch_unwind(|| {
        let _manager = ShellManager::new();
    }) {
        Ok(()) => {
            let processing_time = start_time.elapsed().as_millis();
            info!("Shell管理器初始化成功, 耗时: {}ms", processing_time);
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
    let start_time = Instant::now();
    info!("开始验证Shell管理器状态");

    // ShellManager 不需要单独的验证方法，创建实例时自动验证
    match std::panic::catch_unwind(|| {
        let manager = ShellManager::new();
        let _stats = manager.get_stats();
    }) {
        Ok(()) => {
            let processing_time = start_time.elapsed().as_millis();
            info!("Shell管理器验证成功, 耗时: {}ms", processing_time);
            Ok(())
        }
        Err(_) => {
            let error_msg = "Shell管理器验证失败";
            error!("{}", error_msg);
            Err(error_msg.to_string())
        }
    }
}
