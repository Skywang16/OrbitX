//! TOML配置管理模块

pub mod events;
pub mod manager;
pub mod reader;
pub mod validator;
pub mod writer;

// 重新导出主要类型和功能
pub use events::{ConfigEvent, ConfigEventSender};
pub use manager::TomlConfigManager;
pub use reader::TomlConfigReader;
pub use validator::TomlConfigValidator;
pub use writer::TomlConfigWriter;

// 为了保持向后兼容，重新导出原来的主要类型
pub use manager::TomlConfigManager as TomlManager;

#[cfg(test)]
mod tests;
