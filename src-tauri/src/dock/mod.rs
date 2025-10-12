pub mod commands;
pub mod state;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

use state::{DockState, TabEntry};

pub struct DockManager<R: tauri::Runtime = tauri::Wry> {
    state: DockState,
    #[cfg(target_os = "macos")]
    _macos_impl: macos::MacOSDockMenu<R>,
    #[cfg(target_os = "windows")]
    _windows_impl: windows::WindowsJumpList<R>,
    #[cfg(target_os = "linux")]
    _linux_impl: linux::LinuxDockMenu<R>,
}

impl<R: tauri::Runtime> DockManager<R> {
    #[allow(unused_variables)]
    pub fn new(app_handle: &tauri::AppHandle<R>) -> Result<Self, String> {
        let state = DockState::new();

        #[cfg(target_os = "macos")]
        {
            let macos_impl = macos::MacOSDockMenu::new(state.clone(), app_handle)?;
            Ok(Self {
                state,
                _macos_impl: macos_impl,
            })
        }

        #[cfg(target_os = "windows")]
        {
            let windows_impl = windows::WindowsJumpList::new(state.clone(), app_handle)?;
            Ok(Self {
                state,
                _windows_impl: windows_impl,
            })
        }

        #[cfg(target_os = "linux")]
        {
            let linux_impl = linux::LinuxDockMenu::new(state.clone(), app_handle)?;
            Ok(Self {
                state,
                _linux_impl: linux_impl,
            })
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        {
            Err("Unsupported platform".to_string())
        }
    }

    pub fn update_tabs(&self, tabs: Vec<TabEntry>, active_tab_id: Option<String>) -> Result<(), String> {
        self.state.update_tabs(tabs, active_tab_id)?;

        #[cfg(target_os = "macos")]
        {
            self._macos_impl.refresh_menu()?;
        }

        #[cfg(target_os = "windows")]
        {
            self._windows_impl.refresh_menu()?;
        }

        #[cfg(target_os = "linux")]
        {
            self._linux_impl.refresh_menu()?;
        }

        Ok(())
    }

    pub fn state(&self) -> &DockState {
        &self.state
    }
}
