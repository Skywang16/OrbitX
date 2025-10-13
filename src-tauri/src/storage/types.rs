/*!
 * 存储系统类型定义模块
 *
 * 定义存储系统中使用的核心数据类型和接口
 */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 存储层类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageLayer {
    Config,
    State,
    Data,
}

impl StorageLayer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Config => "config",
            Self::State => "state",
            Self::Data => "data",
        }
    }
}

/// 会话状态数据结构 - 统一 tab 管理
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionState {
    pub version: u32,
    pub tabs: Vec<TabState>,
    pub ui: UiState,
    pub ai: AiState,
    pub timestamp: DateTime<Utc>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            version: 1,
            tabs: Vec::new(),
            ui: UiState::default(),
            ai: AiState::default(),
            timestamp: Utc::now(),
        }
    }
}

/// Tab ID - 统一使用 i32（支持负数，用于特殊 tab 如 Settings）
pub type TabId = i32;

/// Tab 状态 - 支持不同类型的 tab（扁平结构）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TabState {
    #[serde(rename = "terminal", rename_all = "camelCase")]
    Terminal {
        id: i32,  // 改用 i32 支持负数
        #[serde(rename = "isActive")]
        is_active: bool,
        data: TerminalTabData,
    },
    #[serde(rename = "settings", rename_all = "camelCase")]
    Settings {
        id: i32,  // 改用 i32 支持负数
        #[serde(rename = "isActive")]
        is_active: bool,
        data: SettingsTabData,
    },
}

/// Terminal tab 数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalTabData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shell_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// Settings tab 数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SettingsTabData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_section: Option<String>,
}

/// 窗口状态 - 精简版
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: 100,
            y: 100,
            width: 1200,
            height: 800,
            maximized: false,
        }
    }
}

/// 运行时终端状态（用于命令返回）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalRuntimeState {
    pub id: u32,
    pub cwd: String,
    pub shell: Option<String>,
}

/// UI状态 - 精简版
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiState {
    /// 主题名称
    pub theme: String,
    /// 字体大小
    pub font_size: f32,
    /// 侧边栏宽度
    pub sidebar_width: u32,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font_size: 14.0,
            sidebar_width: 300,
        }
    }
}

/// AI状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiState {
    /// 是否可见
    pub visible: bool,
    /// 侧边栏宽度
    pub width: u32,
    /// 聊天模式
    pub mode: String, // "chat" | "agent"
    /// 当前会话ID
    pub conversation_id: Option<i64>,
    /// 选中的模型ID
    pub selected_model_id: Option<String>,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            visible: false,
            width: 350,
            mode: "chat".to_string(),
            conversation_id: None,
            selected_model_id: None,
        }
    }
}

/// 配置节类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigSection {
    /// 应用配置
    App,
    /// 外观配置
    Appearance,
    /// 终端配置
    Terminal,
    /// 快捷键配置
    Shortcuts,
    /// AI配置
    Ai,
    /// 自定义节
    Custom(String),
}

impl ConfigSection {
    pub fn as_str(&self) -> &str {
        match self {
            Self::App => "app",
            Self::Appearance => "appearance",
            Self::Terminal => "terminal",
            Self::Shortcuts => "shortcuts",
            Self::Ai => "ai",
            Self::Custom(name) => name,
        }
    }
}

impl From<&str> for ConfigSection {
    fn from(s: &str) -> Self {
        match s {
            "app" => Self::App,
            "appearance" => Self::Appearance,
            "terminal" => Self::Terminal,
            "shortcuts" => Self::Shortcuts,
            "ai" => Self::Ai,
            custom => Self::Custom(custom.to_string()),
        }
    }
}

impl From<String> for ConfigSection {
    fn from(s: String) -> Self {
        ConfigSection::from(s.as_str())
    }
}

/// 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageEvent {
    /// 配置更改事件
    ConfigChanged {
        section: ConfigSection,
        old_value: Option<serde_json::Value>,
        new_value: serde_json::Value,
    },
    /// 状态保存事件
    StateSaved { timestamp: DateTime<Utc>, size: u64 },
    /// 状态加载事件
    StateLoaded { timestamp: DateTime<Utc>, size: u64 },
    /// 数据更新事件
    DataUpdated {
        table: String,
        operation: String,
        affected_rows: usize,
    },
    /// 缓存事件
    CacheEvent { operation: String, key: String },
    /// 错误事件
    Error {
        layer: StorageLayer,
        error: String,
        timestamp: DateTime<Utc>,
    },
}

/// 存储事件监听器
pub trait StorageEventListener: Send + Sync {
    fn on_event(&self, event: StorageEvent);
}

/// 简单的函数式事件监听器
pub struct FunctionListener<F>
where
    F: Fn(StorageEvent) + Send + Sync,
{
    func: F,
}

impl<F> FunctionListener<F>
where
    F: Fn(StorageEvent) + Send + Sync,
{
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

impl<F> StorageEventListener for FunctionListener<F>
where
    F: Fn(StorageEvent) + Send + Sync,
{
    fn on_event(&self, event: StorageEvent) {
        (self.func)(event);
    }
}
