/*!
 * 命令处理模块
 *
 * 这个模块包含所有的 Tauri 命令处理函数，这些函数可以被前端 JavaScript 代码调用。
 * 模块按功能分类组织，便于维护和扩展。
 */

/// 基于 TerminalMux 的终端命令处理
pub mod mux_terminal;

/// 网络请求命令处理
pub mod web_fetch;

// 重新导出所有命令函数和状态类型，使它们可以在 lib.rs 中直接使用
pub use mux_terminal::*;
pub use web_fetch::*;
