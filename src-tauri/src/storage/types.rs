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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionState {
    pub version: u32,
    pub workspace: WorkspaceState,
    pub ui: UiState,
    pub ai: AiState,
    pub timestamp: DateTime<Utc>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            version: 1,
            workspace: WorkspaceState::default(),
            ui: UiState::default(),
            ai: AiState::default(),
            timestamp: Utc::now(),
        }
    }
}

pub type TabId = String;
pub type GroupId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    pub root: GroupNode,
    pub groups: HashMap<GroupId, TabGroupState>,
    pub active_group_id: GroupId,
}

impl Default for WorkspaceState {
    fn default() -> Self {
        let group_id: GroupId = "group:default".to_string();
        let root = GroupNode::Leaf {
            id: "leaf:default".to_string(),
            group_id: group_id.clone(),
        };

        let mut groups = HashMap::new();
        groups.insert(
            group_id.clone(),
            TabGroupState {
                id: group_id.clone(),
                tabs: Vec::new(),
                active_tab_id: None,
            },
        );

        Self {
            root,
            groups,
            active_group_id: group_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum GroupNode {
    #[serde(rename = "leaf", rename_all = "camelCase")]
    Leaf { id: String, group_id: GroupId },

    #[serde(rename = "split")]
    Split {
        id: String,
        direction: String,
        ratio: f64,
        first: Box<GroupNode>,
        second: Box<GroupNode>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabGroupState {
    pub id: GroupId,
    pub tabs: Vec<TabState>,
    pub active_tab_id: Option<TabId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TabContext {
    #[serde(rename = "none")]
    None,

    #[serde(rename = "terminal", rename_all = "camelCase")]
    Terminal { pane_id: u32 },

    #[serde(rename = "workspace")]
    Workspace { path: String },

    #[serde(rename = "git", rename_all = "camelCase")]
    Git { repo_path: String },
}

impl Default for TabContext {
    fn default() -> Self {
        Self::None
    }
}

/// 通用的 Tab 状态结构
/// 后端只负责存储和传输，不关心具体的 tab 类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabState {
    /// Tab ID
    pub id: String,
    /// Tab 类型（任意字符串：terminal, diff, settings, ai-chat, ...）
    #[serde(rename = "type")]
    pub tab_type: String,
    /// 是否为活跃 tab
    #[serde(rename = "isActive")]
    pub is_active: bool,
    /// Tab 的上下文载荷（前后端强约定）
    #[serde(default)]
    pub context: TabContext,
    /// Tab 的数据载荷（JSON 格式，后端不解析具体内容）
    pub data: serde_json::Value,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalRuntimeState {
    pub id: u32,
    pub cwd: String,
    pub shell: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiState {
    pub theme: String,
    pub font_size: f32,
    pub sidebar_width: u32,
    #[serde(default)]
    pub left_sidebar_visible: bool,
    #[serde(default = "default_left_sidebar_width")]
    pub left_sidebar_width: u32,
    #[serde(default)]
    pub left_sidebar_active_panel: Option<String>,
    #[serde(default)]
    pub onboarding_completed: bool,
}

fn default_left_sidebar_width() -> u32 {
    280
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            font_size: 14.0,
            sidebar_width: 300,
            left_sidebar_visible: false,
            left_sidebar_width: default_left_sidebar_width(),
            left_sidebar_active_panel: Some("workspace".to_string()),
            onboarding_completed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiState {
    pub visible: bool,
    pub width: u32,
    pub mode: String,
    #[serde(default)]
    pub session_id: Option<i64>,
    #[serde(default)]
    pub workspace_path: Option<String>,
    #[serde(default)]
    pub selected_model_id: Option<String>,
}

impl Default for AiState {
    fn default() -> Self {
        Self {
            visible: false,
            width: 350,
            mode: "chat".to_string(),
            session_id: None,
            workspace_path: None,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_node_leaf_accepts_camel_case_group_id() {
        let value = serde_json::json!({
            "type": "leaf",
            "id": "leaf:0",
            "groupId": "group:0"
        });
        let node: GroupNode = serde_json::from_value(value).unwrap();
        match node {
            GroupNode::Leaf { id, group_id } => {
                assert_eq!(id, "leaf:0");
                assert_eq!(group_id, "group:0");
            }
            _ => panic!("expected leaf"),
        }
    }

    #[test]
    fn tab_context_terminal_accepts_camel_case_pane_id() {
        let value = serde_json::json!({
            "kind": "terminal",
            "paneId": 123
        });
        let ctx: TabContext = serde_json::from_value(value).unwrap();
        match ctx {
            TabContext::Terminal { pane_id } => assert_eq!(pane_id, 123),
            _ => panic!("expected terminal"),
        }
    }

    #[test]
    fn tab_context_git_accepts_camel_case_repo_path() {
        let value = serde_json::json!({
            "kind": "git",
            "repoPath": "/tmp/repo"
        });
        let ctx: TabContext = serde_json::from_value(value).unwrap();
        match ctx {
            TabContext::Git { repo_path } => assert_eq!(repo_path, "/tmp/repo"),
            _ => panic!("expected git"),
        }
    }

    #[test]
    fn session_state_accepts_frontend_camel_case_shape() {
        let group_id = "group:0";
        let value = serde_json::json!({
            "version": 1,
            "workspace": {
                "root": { "type": "leaf", "id": "leaf:0", "groupId": group_id },
                "groups": {
                    group_id: {
                        "id": group_id,
                        "tabs": [
                            {
                                "id": "terminal:1",
                                "type": "terminal",
                                "isActive": true,
                                "context": { "kind": "terminal", "paneId": 1 },
                                "data": { "cwd": "/tmp", "shellName": "zsh" }
                            }
                        ],
                        "activeTabId": "terminal:1"
                    }
                },
                "activeGroupId": group_id
            },
            "ui": {
                "theme": "dark",
                "fontSize": 14,
                "sidebarWidth": 300,
                "leftSidebarVisible": false,
                "leftSidebarWidth": 280,
                "leftSidebarActivePanel": "workspace",
                "onboardingCompleted": false
            },
            "ai": {
                "visible": false,
                "width": 350,
                "mode": "chat"
            },
            "timestamp": "2026-01-08T00:00:00Z"
        });

        let state: SessionState = serde_json::from_value(value).unwrap();
        assert_eq!(state.workspace.active_group_id, group_id);
        assert_eq!(state.workspace.groups.get(group_id).unwrap().id, group_id);
    }
}
