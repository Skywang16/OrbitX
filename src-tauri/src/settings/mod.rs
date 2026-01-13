pub mod commands;
pub mod error;
pub mod manager;
pub mod types;

pub use commands::{
    get_effective_settings, get_global_settings, get_workspace_settings, update_global_settings,
    update_workspace_settings,
};
pub use error::{SettingsError, SettingsResult};
pub use manager::SettingsManager;
pub use types::{
    AgentConfig, AgentConfigPatch, EffectiveSettings, McpServerConfig, PermissionRules, RulesConfig,
    Settings,
};

