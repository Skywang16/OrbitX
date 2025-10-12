use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabEntry {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct DockState {
    tabs: Arc<RwLock<Vec<TabEntry>>>,
    active_tab_id: Arc<RwLock<Option<String>>>,
}

impl DockState {
    pub fn new() -> Self {
        Self {
            tabs: Arc::new(RwLock::new(Vec::new())),
            active_tab_id: Arc::new(RwLock::new(None)),
        }
    }

    pub fn update_tabs(&self, tabs: Vec<TabEntry>, active_tab_id: Option<String>) -> Result<(), String> {
        let mut state = self
            .tabs
            .write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        *state = tabs;
        
        let mut active = self
            .active_tab_id
            .write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        *active = active_tab_id;
        
        Ok(())
    }

    pub fn get_tabs(&self) -> Result<Vec<TabEntry>, String> {
        let state = self
            .tabs
            .read()
            .map_err(|e| format!("Failed to acquire read lock: {}", e))?;
        Ok(state.clone())
    }
    
    pub fn get_active_tab_id(&self) -> Result<Option<String>, String> {
        let active = self
            .active_tab_id
            .read()
            .map_err(|e| format!("Failed to acquire read lock: {}", e))?;
        Ok(active.clone())
    }

    pub fn clear(&self) -> Result<(), String> {
        let mut state = self
            .tabs
            .write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        state.clear();
        Ok(())
    }
}

impl Default for DockState {
    fn default() -> Self {
        Self::new()
    }
}
