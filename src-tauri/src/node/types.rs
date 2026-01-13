use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
}

impl FromStr for NodeVersionManager {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "nvm" => NodeVersionManager::Nvm,
            "fnm" => NodeVersionManager::Fnm,
            "volta" => NodeVersionManager::Volta,
            "n" => NodeVersionManager::N,
            "asdf" => NodeVersionManager::Asdf,
            _ => NodeVersionManager::Unknown,
        })
    }
}

/// Node.js 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeVersionInfo {
    pub version: String,
    pub is_current: bool,
}
