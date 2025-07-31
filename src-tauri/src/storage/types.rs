/*!
 * 存储系统类型定义模块
 *
 * 定义存储系统中使用的核心数据类型和接口
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

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

/// 缓存层类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CacheLayer {
    /// 内存缓存
    Memory,
    /// LRU缓存
    Lru,
    /// 磁盘缓存
    Disk,
}

impl CacheLayer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Memory => "memory",
            Self::Lru => "lru",
            Self::Disk => "disk",
        }
    }
}

/// 数据查询结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataQuery {
    /// 查询语句或条件
    pub query: String,
    /// 查询参数
    pub params: HashMap<String, serde_json::Value>,
    /// 限制结果数量
    pub limit: Option<usize>,
    /// 偏移量
    pub offset: Option<usize>,
    /// 排序字段
    pub order_by: Option<String>,
    /// 是否降序
    pub desc: bool,
}

impl DataQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            params: HashMap::new(),
            limit: None,
            offset: None,
            order_by: None,
            desc: false,
        }
    }

    pub fn with_param(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.params.insert(key.into(), value);
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_order_by(mut self, field: impl Into<String>, desc: bool) -> Self {
        self.order_by = Some(field.into());
        self.desc = desc;
        self
    }
}

/// 保存选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveOptions {
    /// 目标表或集合名称
    pub table: Option<String>,
    /// 是否覆盖现有数据
    pub overwrite: bool,
    /// 是否创建备份
    pub backup: bool,
    /// 是否验证数据
    pub validate: bool,
    /// 自定义元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for SaveOptions {
    fn default() -> Self {
        Self {
            table: None,
            overwrite: false,
            backup: true,
            validate: true,
            metadata: HashMap::new(),
        }
    }
}

impl SaveOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn table(mut self, table: impl Into<String>) -> Self {
        self.table = Some(table.into());
        self
    }

    pub fn overwrite(mut self, overwrite: bool) -> Self {
        self.overwrite = overwrite;
        self
    }

    pub fn backup(mut self, backup: bool) -> Self {
        self.backup = backup;
        self
    }

    pub fn validate(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

/// 会话状态数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub created_at: SystemTime,
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
            created_at: SystemTime::now(),
            checksum: None,
        }
    }
}

/// 窗口状态
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub created_at: SystemTime,
    /// 最后活跃时间
    pub last_active: SystemTime,
}

/// UI状态
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        }
    }
}

/// 缓存统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// 缓存层统计
    pub layers: HashMap<CacheLayer, LayerStats>,
    /// 总命中率
    pub total_hit_rate: f64,
    /// 总内存使用量（字节）
    pub total_memory_usage: u64,
    /// 总条目数
    pub total_entries: usize,
}

/// 单层缓存统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStats {
    /// 命中次数
    pub hits: u64,
    /// 未命中次数
    pub misses: u64,
    /// 条目数量
    pub entries: usize,
    /// 内存使用量（字节）
    pub memory_usage: u64,
    /// 平均访问时间（纳秒）
    pub avg_access_time: Duration,
}

impl LayerStats {
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

/// 存储统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    /// 总大小（字节）
    pub total_size: u64,
    /// 配置层大小
    pub config_size: u64,
    /// 状态层大小
    pub state_size: u64,
    /// 数据层大小
    pub data_size: u64,
    /// 缓存层大小
    pub cache_size: u64,
    /// 备份大小
    pub backups_size: u64,
    /// 日志大小
    pub logs_size: u64,
}

impl StorageStats {
    /// 格式化大小为人类可读的字符串
    pub fn format_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_index])
    }

    /// 获取格式化的总大小
    pub fn total_size_formatted(&self) -> String {
        Self::format_size(self.total_size)
    }

    /// 获取格式化的配置大小
    pub fn config_size_formatted(&self) -> String {
        Self::format_size(self.config_size)
    }

    /// 获取格式化的状态大小
    pub fn state_size_formatted(&self) -> String {
        Self::format_size(self.state_size)
    }

    /// 获取格式化的数据大小
    pub fn data_size_formatted(&self) -> String {
        Self::format_size(self.data_size)
    }

    /// 获取格式化的缓存大小
    pub fn cache_size_formatted(&self) -> String {
        Self::format_size(self.cache_size)
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
    StateSaved { timestamp: SystemTime, size: u64 },
    /// 状态加载事件
    StateLoaded { timestamp: SystemTime, size: u64 },
    /// 数据更新事件
    DataUpdated {
        table: String,
        operation: String,
        affected_rows: usize,
    },
    /// 缓存事件
    CacheEvent {
        layer: CacheLayer,
        operation: String,
        key: String,
    },
    /// 错误事件
    Error {
        layer: StorageLayer,
        error: String,
        timestamp: SystemTime,
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
