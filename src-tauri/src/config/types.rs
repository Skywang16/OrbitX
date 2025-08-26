/*!
 * 配置系统数据类型定义
 *
 * 定义配置系统中使用的所有数据结构，包括主配置、主题配置、
 * 元数据和事件等。结构与 TOML 配置文件格式保持完全一致。
 */

use crate::config::theme::ThemeConfig;
use serde::{Deserialize, Serialize};

/// 主配置结构
///
/// 包含应用程序的所有配置项，支持版本控制和验证。
/// 结构与 TOML 配置文件格式保持一致。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    /// 配置版本
    pub version: String,

    /// 配置元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ConfigMetadata>,

    /// 应用配置 (对应 TOML 中的 [app])
    pub app: AppConfigApp,

    /// 外观配置 (对应 TOML 中的 [appearance])
    pub appearance: AppearanceConfig,

    /// 终端配置
    pub terminal: TerminalConfig,

    /// 快捷键配置 (对应 TOML 中的 [shortcuts])
    pub shortcuts: ShortcutsConfig,
}

/// 应用配置 (对应 TOML 中的 [app] 节)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfigApp {
    /// 界面语言
    pub language: String,

    /// 退出时确认
    pub confirm_on_exit: bool,

    /// 启动行为
    pub startup_behavior: String,
}

/// 外观配置 (对应 TOML 中的 [appearance] 节)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppearanceConfig {
    /// UI 缩放比例
    pub ui_scale: u32,

    /// 启用动画
    pub animations_enabled: bool,

    /// 主题配置
    pub theme_config: ThemeConfig,

    /// 字体配置
    pub font: FontConfig,

    /// 窗口透明度 (0.0 - 1.0)
    pub opacity: f64,
}

/// 终端配置 (对应 TOML 中的 [terminal] 节)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TerminalConfig {
    /// 滚动缓冲区行数
    pub scrollback: u32,

    /// Shell 配置
    pub shell: ShellConfig,

    /// 光标配置
    pub cursor: CursorConfig,

    /// 终端行为配置
    pub behavior: TerminalBehaviorConfig,
}

/// Shell 配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShellConfig {
    /// 默认 shell
    #[serde(rename = "default")]
    pub default_shell: String,

    /// shell 参数
    pub args: Vec<String>,

    /// 工作目录
    pub working_directory: String,
}

/// 终端行为配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TerminalBehaviorConfig {
    /// 进程退出时关闭
    pub close_on_exit: bool,

    /// 关闭时确认
    pub confirm_close: bool,
}

/// 字体配置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FontConfig {
    /// 字体族
    pub family: String,

    /// 字体大小
    pub size: f32,

    /// 字体粗细
    pub weight: FontWeight,

    /// 字体样式
    pub style: FontStyle,

    /// 行高
    pub line_height: f32,

    /// 字符间距
    pub letter_spacing: f32,
}

/// 光标配置 (对应 TOML 中的 [terminal.cursor] 节)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CursorConfig {
    /// 光标样式
    pub style: CursorStyle,

    /// 光标闪烁
    pub blink: bool,

    /// 光标颜色
    pub color: String,

    /// 光标粗细
    pub thickness: f32,
}

/// 快捷键配置 (对应 TOML 中的 [shortcuts] 节)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShortcutsConfig {
    /// 全局快捷键
    #[serde(default)]
    pub global: Vec<ShortcutBinding>,

    /// 终端快捷键
    #[serde(default)]
    pub terminal: Vec<ShortcutBinding>,

    /// 自定义快捷键
    #[serde(default)]
    pub custom: Vec<ShortcutBinding>,
}

/// 快捷键绑定
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShortcutBinding {
    /// 按键
    pub key: String,

    /// 修饰键
    pub modifiers: Vec<String>,

    /// 动作
    pub action: ShortcutAction,
}

/// 快捷键动作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ShortcutAction {
    /// 简单动作（字符串）
    Simple(String),
    /// 复杂动作（对象）
    Complex {
        #[serde(rename = "type")]
        action_type: String,
        text: Option<String>,
    },
}

// ============================================================================
// 枚举类型定义
// ============================================================================

/// 字体粗细
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

/// 字体样式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

/// 光标样式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CursorStyle {
    Block,
    Underline,
    Beam,
}

// ============================================================================
// 配置元数据和事件
// ============================================================================

/// 配置元数据
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMetadata {
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// 最后修改时间
    pub modified_at: chrono::DateTime<chrono::Utc>,

    /// 配置版本
    pub version: String,

    /// 校验和
    pub checksum: String,

    /// 备份信息
    pub backup_info: Option<BackupInfo>,
}

/// 备份信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    /// 备份路径
    pub backup_path: String,

    /// 备份时间
    pub backup_time: chrono::DateTime<chrono::Utc>,

    /// 原始版本
    pub original_version: String,
}

/// 配置更改事件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChangeEvent {
    /// 更改类型
    pub change_type: ConfigChangeType,

    /// 字段路径
    pub field_path: String,

    /// 旧值
    pub old_value: Option<serde_json::Value>,

    /// 新值
    pub new_value: Option<serde_json::Value>,

    /// 时间戳
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 配置更改类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConfigChangeType {
    Created,
    Updated,
    Deleted,
}
