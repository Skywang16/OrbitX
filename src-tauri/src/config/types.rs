//! 配置系统数据类型定义

use crate::config::theme::ThemeConfig;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ConfigMetadata>,
    pub app: AppConfigApp,
    pub appearance: AppearanceConfig,
    pub terminal: TerminalConfig,
    pub shortcuts: ShortcutsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfigApp {
    pub language: String,
    pub confirm_on_exit: bool,
    pub startup_behavior: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppearanceConfig {
    pub ui_scale: u32,
    pub animations_enabled: bool,
    pub theme_config: ThemeConfig,
    pub font: FontConfig,
    pub opacity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TerminalConfig {
    pub scrollback: u32,
    pub shell: ShellConfig,
    pub cursor: CursorConfig,
    pub behavior: TerminalBehaviorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShellConfig {
    #[serde(rename = "default")]
    pub default_shell: String,
    pub args: Vec<String>,
    pub working_directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TerminalBehaviorConfig {
    pub close_on_exit: bool,
    pub confirm_close: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FontConfig {
    pub family: String,
    pub size: f32,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub line_height: f32,
    pub letter_spacing: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CursorConfig {
    pub style: CursorStyle,
    pub blink: bool,
    pub color: String,
    pub thickness: f32,
}

pub type ShortcutsConfig = Vec<ShortcutBinding>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShortcutBinding {
    pub key: String,
    pub modifiers: Vec<String>,
    pub action: ShortcutAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ShortcutAction {
    Simple(String),
    Complex {
        #[serde(rename = "type")]
        action_type: String,
        text: Option<String>,
    },
}

// 枚举类型定义

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    Thin,
    Light,
    Normal,
    Medium,
    Bold,
    Black,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    Block,
    Underline,
    Beam,
}

// 配置元数据和事件

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMetadata {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub checksum: String,
    pub backup_info: Option<BackupInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub backup_path: String,
    pub backup_time: chrono::DateTime<chrono::Utc>,
    pub original_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChangeEvent {
    pub change_type: ConfigChangeType,
    pub field_path: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigChangeType {
    Created,
    Updated,
    Deleted,
}
