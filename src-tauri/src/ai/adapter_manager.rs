/*!
 * 简化的AI适配器管理器
 *
 * 管理多个AI客户端实例
 */

use crate::ai::AIAdapter;
use std::collections::HashMap;
use std::sync::Arc;

/// 简化的AI适配器管理器
pub struct AIAdapterManager {
    adapters: HashMap<String, Arc<dyn AIAdapter>>,
}

impl Default for AIAdapterManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AIAdapterManager {
    /// 创建新的适配器管理器
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    /// 注册适配器
    pub fn register_adapter(&mut self, model_id: String, adapter: Arc<dyn AIAdapter>) {
        self.adapters.insert(model_id, adapter);
    }

    /// 获取适配器
    pub fn get_adapter(&self, model_id: &str) -> Option<Arc<dyn AIAdapter>> {
        self.adapters.get(model_id).cloned()
    }

    /// 移除适配器
    pub fn remove_adapter(&mut self, model_id: &str) -> bool {
        self.adapters.remove(model_id).is_some()
    }

    /// 获取所有适配器ID
    pub fn get_adapter_ids(&self) -> Vec<String> {
        self.adapters.keys().cloned().collect()
    }

    /// 检查适配器是否存在
    pub fn has_adapter(&self, model_id: &str) -> bool {
        self.adapters.contains_key(model_id)
    }

    /// 获取适配器数量
    pub fn adapter_count(&self) -> usize {
        self.adapters.len()
    }

    /// 清空所有适配器
    pub fn clear(&mut self) {
        self.adapters.clear();
    }
}
