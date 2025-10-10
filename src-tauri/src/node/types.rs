//! Node.js 版本管理相关数据结构

use serde::{Deserialize, Serialize};

/// Node.js 版本管理器类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NodeVersionManager {
    Nvm,
    Fnm,
    Volta,
    N,
    Asdf,
    Unknown,
}

impl NodeVersionManager {
    pub fn as_str(&self) -> &str {
        match self {
            NodeVersionManager::Nvm => "nvm",
            NodeVersionManager::Fnm => "fnm",
            NodeVersionManager::Volta => "volta",
            NodeVersionManager::N => "n",
            NodeVersionManager::Asdf => "asdf",
            NodeVersionManager::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "nvm" => NodeVersionManager::Nvm,
            "fnm" => NodeVersionManager::Fnm,
            "volta" => NodeVersionManager::Volta,
            "n" => NodeVersionManager::N,
            "asdf" => NodeVersionManager::Asdf,
            _ => NodeVersionManager::Unknown,
        }
    }
}

/// Node.js 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeVersionInfo {
    pub version: String,
    pub is_current: bool,
}
