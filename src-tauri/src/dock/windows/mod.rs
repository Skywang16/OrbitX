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
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;

        use windows::core::{HSTRING, PCWSTR, Interface, IUnknown};
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
        use windows::Win32::UI::Shell::{DestinationList, ICustomDestinationList, IShellLinkW, ShellLink};
        use windows::Win32::UI::Shell::Common::{EnumerableObjectCollection, IObjectArray, IObjectCollection};

        // Create destination list
        let dest_list: ICustomDestinationList =
            CoCreateInstance(&DestinationList, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("Failed to create ICustomDestinationList: {:?}", e))?;

        // Begin list
        let mut _min_slots = 0u32;
        let _removed: IObjectArray = dest_list
            .BeginList(&mut _min_slots)
            .map_err(|e| format!("Failed to begin list: {:?}", e))?;

        // Build custom category from current tabs (limit to avoid oversized lists)
        let collection: IObjectCollection = CoCreateInstance(&EnumerableObjectCollection, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| format!("Failed to create EnumerableObjectCollection: {:?}", e))?;

        // Resolve current exe path
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("Failed to get current exe path: {e}"))?;
        let exe_w: Vec<u16> = exe_path
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();

        for tab in tabs.iter().take(10) {
            // Create a shell link to the current executable with arguments to identify the tab
            let link: IShellLinkW =
                CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| format!("Failed to create IShellLink: {:?}", e))?;

            link
                .SetPath(PCWSTR(exe_w.as_ptr()))
                .map_err(|e| format!("Failed to SetPath on IShellLink: {:?}", e))?;

            let args = format!("--activate-tab {}", tab.id);
            let args_w: Vec<u16> = OsStr::new(&args).encode_wide().chain(Some(0)).collect();
            link
                .SetArguments(PCWSTR(args_w.as_ptr()))
                .map_err(|e| format!("Failed to SetArguments on IShellLink: {:?}", e))?;

            // Optionally set description (title) via IPropertyStore if needed in future
            // let prop_store: IPropertyStore = link.cast()?;
            // prop_store.SetValue(&PKEY_Title, &PropVariantFromString(...))?;

            // Add to collection
            collection
                .AddObject(
                    link
                        .cast::<IUnknown>()
                        .map_err(|e| format!("Failed to cast IShellLink to IUnknown: {:?}", e))?,
                )
                .map_err(|e| format!("Failed to add link to collection: {:?}", e))?;
        }

        let tasks: IObjectArray = collection
            .cast::<IObjectArray>()
            .map_err(|e| format!("Failed to cast collection to IObjectArray: {:?}", e))?;

        let category_name = HSTRING::from("Tabs");
        dest_list
            .AppendCategory(&category_name, &tasks)
            .map_err(|e| format!("Failed to append custom category: {:?}", e))?;

        // Commit the list
        dest_list
            .CommitList()
            .map_err(|e| format!("Failed to commit list: {:?}", e))?;

        tracing::debug!("Windows Jump List updated with {} tabs", tabs.len());
        Ok(())
    }
}

impl<R: Runtime> Drop for WindowsJumpList<R> {
    fn drop(&mut self) {}
}
