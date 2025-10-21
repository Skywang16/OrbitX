use crate::dock::state::DockState;
use tauri::{AppHandle, Runtime};

pub struct LinuxDockMenu<R: Runtime> {
    _state: DockState,
    _app_handle: AppHandle<R>,
}

impl<R: Runtime> LinuxDockMenu<R> {
    pub fn new(state: DockState, app_handle: &AppHandle<R>) -> Result<Self, String> {
        tracing::info!("Linux dock menu is not yet implemented");
        Ok(Self {
            _state: state,
            _app_handle: app_handle.clone(),
        })
    }

    pub fn refresh_menu(&self) -> Result<(), String> {
        Ok(())
    }
}
