/*!
 * 存储系统类型定义模块
 *
 * 定义存储系统中使用的核心数据类型和接口
 */

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 存储层类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageLayer {
    /// TOML配置层
    Config,
    /// MessagePack状态层
    State,
    /// SQLite数据层
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







/// 会话状态数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionState {
    /// 版本号
    pub version: u32,
    /// 窗口状态
    pub window_state: WindowState,
    /// 标签页状态
    pub tabs: Vec<TabState>,
    /// 终端会话状态
    pub terminal_sessions: HashMap<String, TerminalSession>,
    /// UI状态
    pub ui_state: UiState,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 校验和
    pub checksum: Option<String>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            version: 1,
            window_state: WindowState::default(),
            tabs: Vec::new(),
            terminal_sessions: HashMap::new(),
            ui_state: UiState::default(),
            created_at: Utc::now(),
            checksum: None,
        }
    }
}

/// 窗口状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    /// 窗口位置 (x, y)
    pub position: (i32, i32),
    /// 窗口大小 (width, height)
    pub size: (u32, u32),
    /// 是否最大化
    pub is_maximized: bool,
    /// 是否全屏
    pub is_fullscreen: bool,
    /// 是否置顶
    pub is_always_on_top: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            position: (100, 100),
            size: (1200, 800),
            is_maximized: false,
            is_fullscreen: false,
            is_always_on_top: false,
        }
    }
}

/// 标签页状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabState {
    /// 标签页ID
    pub id: String,
    /// 标签页标题
    pub title: String,
    /// 是否激活
    pub is_active: bool,
    /// 工作目录
    pub working_directory: String,
    /// 终端会话ID
    pub terminal_session_id: Option<String>,
    /// 自定义数据
    pub custom_data: HashMap<String, serde_json::Value>,
}

/// 终端会话状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSession {
    /// 会话ID
    pub id: String,
    /// 会话标题
    pub title: String,
    /// 工作目录
    pub working_directory: String,
    /// 环境变量
    pub environment: HashMap<String, String>,
    /// 命令历史
    pub command_history: Vec<String>,
    /// 是否活跃
    pub is_active: bool,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active: DateTime<Utc>,
}

/// OrbitX AI 聊天状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrbitxChatState {
    /// 是否可见
    pub is_visible: bool,
    /// 侧边栏宽度
    pub sidebar_width: u32,
    /// 当前模式
    pub chat_mode: String, // "chat" | "agent"
    /// 当前会话ID
    pub current_conversation_id: Option<i64>,
}

impl Default for OrbitxChatState {
    fn default() -> Self {
        Self {
            is_visible: false,
            sidebar_width: 350,
            chat_mode: "chat".to_string(),
            current_conversation_id: None,
        }
    }
}

/// UI状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiState {
    /// 侧边栏是否可见
    pub sidebar_visible: bool,
    /// 侧边栏宽度
    pub sidebar_width: u32,
    /// 当前主题
    pub current_theme: String,
    /// 字体大小
    pub font_size: f32,
    /// 缩放级别
    pub zoom_level: f32,
    /// 面板布局
    pub panel_layout: HashMap<String, serde_json::Value>,
    /// OrbitX AI 聊天状态
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orbitx_chat: Option<OrbitxChatState>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            sidebar_visible: true,
            sidebar_width: 300,
            current_theme: "dark".to_string(),
            font_size: 14.0,
            zoom_level: 1.0,
            panel_layout: HashMap::new(),
            orbitx_chat: Some(OrbitxChatState::default()),
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
