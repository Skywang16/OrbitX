use crate::dock::state::DockState;
use tauri::{AppHandle, Runtime};

pub struct WindowsJumpList<R: Runtime> {
    state: DockState,
    _app_handle: AppHandle<R>,
}

impl<R: Runtime> WindowsJumpList<R> {
    pub fn new(state: DockState, app_handle: &AppHandle<R>) -> Result<Self, String> {
        let instance = Self {
            state,
            _app_handle: app_handle.clone(),
        };
        instance.initialize()?;
        Ok(instance)
    }

    fn initialize(&self) -> Result<(), String> {
        unsafe {
            use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
            let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            if hr.is_err() {
                tracing::warn!("COM already initialized or initialization failed: {:?}", hr);
            }
        }

        self.refresh_menu()?;
        tracing::debug!("Windows Jump List initialized");
        Ok(())
    }

    pub fn refresh_menu(&self) -> Result<(), String> {
        let tabs = self
            .state
            .get_tabs()
            .map_err(|e| format!("Failed to get tabs: {}", e))?;

        unsafe { self.update_jump_list(&tabs) }
    }

    unsafe fn update_jump_list(&self, tabs: &[crate::dock::state::TabEntry]) -> Result<(), String> {
        use windows::core::HSTRING;
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
        use windows::Win32::UI::Shell::{
            CLSID_DestinationList, CLSID_EnumerableObjectCollection, DestinationList,
            ICustomDestinationList, IObjectArray, IObjectCollection,
        };

        // Create destination list
        let dest_list: ICustomDestinationList =
            CoCreateInstance(&CLSID_DestinationList, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("Failed to create ICustomDestinationList: {:?}", e))?;

        // Begin list
        let mut min_slots = 0u32;
        let removed: IObjectArray = dest_list
            .BeginList(&mut min_slots)
            .map_err(|e| format!("Failed to begin list: {:?}", e))?;

        // Create object collection for custom category
        let collection: IObjectCollection = CoCreateInstance(
            &CLSID_EnumerableObjectCollection,
            None,
            CLSCTX_INPROC_SERVER,
        )
        .map_err(|e| format!("Failed to create IObjectCollection: {:?}", e))?;

        if !tabs.is_empty() {
            let category_name = HSTRING::from("Tabs");
            let tasks: IObjectArray = collection
                .cast()
                .map_err(|e| format!("Failed to cast to IObjectArray: {:?}", e))?;

            dest_list
                .AppendCategory(&category_name, &tasks)
                .map_err(|e| format!("Failed to append category: {:?}", e))?;
        }

        // Commit the list
        dest_list
            .CommitList()
            .map_err(|e| format!("Failed to commit list: {:?}", e))?;

        tracing::debug!("Windows Jump List updated with {} tabs", tabs.len());
        Ok(())
    }
}

impl Drop for WindowsJumpList {
    fn drop(&mut self) {}
}
