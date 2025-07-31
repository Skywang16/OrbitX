// 日志系统模块

use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

/// 初始化日志系统
/// 支持通过 RUST_LOG 环境变量控制日志级别，默认为 debug
pub fn init_logging() -> Result<(), String> {
    println!("开始初始化日志系统...");

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        println!("使用默认日志级别: debug");
        EnvFilter::new("debug")
    });

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(true)
        .try_init()
        .map_err(|e| {
            let error_msg = format!("日志系统初始化失败: {}", e);
            eprintln!("{}", error_msg);
            error_msg
        })?;

    println!("日志系统初始化完成");
    info!("日志系统初始化完成");
    Ok(())
}
