/*!
 * TOML配置管理模块
 *
 * 提供统一的TOML配置文件管理功能，按功能域组织：
 * - events: 事件系统
 * - reader: 配置读取
 * - writer: 配置写入
 * - validator: 配置验证
 * - manager: 核心管理器
 */

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
