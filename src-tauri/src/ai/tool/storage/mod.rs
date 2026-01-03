pub mod commands;

// 导出所有 storage commands
pub use commands::{
    storage_get_terminal_cwd, storage_get_terminals_state, storage_load_session_state,
    storage_save_session_state,
};

// 重导出 __cmd__ 函数（Tauri 生成的）
pub use commands::{
    __cmd__storage_get_terminal_cwd, __cmd__storage_get_terminals_state,
    __cmd__storage_load_session_state, __cmd__storage_save_session_state,
};
