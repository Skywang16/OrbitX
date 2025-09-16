//! TOML配置事件系统

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// 配置事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigEvent {
    Loaded {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    Updated {
        timestamp: chrono::DateTime<chrono::Utc>,
        section: Option<String>,
    },
    Saved {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    ValidationFailed {
        timestamp: chrono::DateTime<chrono::Utc>,
        errors: Vec<String>,
    },
}

/// 配置事件发送器
pub struct ConfigEventSender {
    sender: broadcast::Sender<ConfigEvent>,
}

impl ConfigEventSender {
    /// 创建新的事件发送器
    pub fn new() -> (Self, broadcast::Receiver<ConfigEvent>) {
        let (sender, receiver) = broadcast::channel(1000);
        (Self { sender }, receiver)
    }

    /// 发送配置加载事件
    pub fn send_loaded(&self) {
        let event = ConfigEvent::Loaded {
            timestamp: chrono::Utc::now(),
        };
        let _ = self.sender.send(event);
    }

    /// 发送配置更新事件
    pub fn send_updated(&self, section: Option<String>) {
        let event = ConfigEvent::Updated {
            timestamp: chrono::Utc::now(),
            section,
        };
        let _ = self.sender.send(event);
    }

    /// 发送配置保存事件
    pub fn send_saved(&self) {
        let event = ConfigEvent::Saved {
            timestamp: chrono::Utc::now(),
        };
        let _ = self.sender.send(event);
    }

    /// 发送验证失败事件
    pub fn send_validation_failed(&self, errors: Vec<String>) {
        let event = ConfigEvent::ValidationFailed {
            timestamp: chrono::Utc::now(),
            errors,
        };
        let _ = self.sender.send(event);
    }

    /// 订阅配置变更事件
    pub fn subscribe(&self) -> broadcast::Receiver<ConfigEvent> {
        self.sender.subscribe()
    }
}

impl Default for ConfigEventSender {
    fn default() -> Self {
        Self::new().0
    }
}
