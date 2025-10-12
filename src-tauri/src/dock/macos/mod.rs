use crate::dock::state::DockState;
use cocoa::appkit::{NSApp, NSMenu, NSMenuItem};
use cocoa::base::{id, nil, NO};
use cocoa::foundation::{NSAutoreleasePool, NSString};
use objc::{class, msg_send, sel, sel_impl};
use objc::runtime::{Class, Object, Sel, BOOL};
use serde_json::json;
use std::ffi::CStr;
use tauri::{AppHandle, Emitter, Runtime};

extern "C" {
    fn objc_setAssociatedObject(
        object: id,
        key: *const std::ffi::c_void,
        value: id,
        policy: usize,
    );
    fn objc_getAssociatedObject(object: id, key: *const std::ffi::c_void) -> id;
    fn class_addMethod(
        cls: *mut Class,
        name: Sel,
        imp: *const std::ffi::c_void,
        types: *const i8,
    ) -> BOOL;
}

const OBJC_ASSOCIATION_RETAIN: usize = 0x301;

pub struct MacOSDockMenu<R: Runtime> {
    state: DockState,
    app_handle: AppHandle<R>,
}

extern "C" fn switch_to_tab_action(this: &Object, _cmd: Sel, sender: id) {
    unsafe {
        let tab_id_obj: id = msg_send![sender, representedObject];
        if tab_id_obj == nil {
            tracing::warn!("No tab ID found in menu item");
            return;
        }
        
        let tab_id_cstr: *const i8 = msg_send![tab_id_obj, UTF8String];
        if tab_id_cstr.is_null() {
            tracing::warn!("Failed to get tab ID string");
            return;
        }
        
        let tab_id_str = std::ffi::CStr::from_ptr(tab_id_cstr)
            .to_string_lossy()
            .to_string();
        
        let pane_id: u32 = match tab_id_str.parse() {
            Ok(id) => id,
            Err(_) => {
                tracing::error!("Invalid tab ID: {}", tab_id_str);
                return;
            }
        };
        
        tracing::info!("Dock menu: switching to tab {}", pane_id);
        
        static APP_HANDLE_KEY: &[u8] = b"orbitx_dock_app_handle\0";
        let key_ptr = APP_HANDLE_KEY.as_ptr() as *const std::ffi::c_void;
        let number: id = objc_getAssociatedObject(this as *const _ as id, key_ptr);
        
        if number != nil {
            let app_handle_ptr: usize = msg_send![number, unsignedLongLongValue];
            if app_handle_ptr != 0 {
                let app_handle_ref = &*(app_handle_ptr as *const tauri::AppHandle);
                let payload = json!({ "tabId": pane_id });
                if let Err(e) = app_handle_ref.emit("dock_switch_tab", payload) {
                    tracing::error!("Failed to emit dock_switch_tab event: {}", e);
                } else {
                    tracing::debug!("Emitted dock_switch_tab event for tab {}", pane_id);
                }
            } else {
                tracing::warn!("AppHandle pointer is null");
            }
        } else {
            tracing::warn!("No AppHandle found in associated object");
        }
        
        let app = NSApp();
        let _: () = msg_send![app, activateIgnoringOtherApps: cocoa::base::YES];
    }
}


impl<R: Runtime> MacOSDockMenu<R> {
    pub fn new(state: DockState, app_handle: &AppHandle<R>) -> Result<Self, String> {
        let instance = Self {
            state: state.clone(),
            app_handle: app_handle.clone(),
        };
        instance.setup_delegate()?;
        Ok(instance)
    }

    fn setup_delegate(&self) -> Result<(), String> {
        unsafe {
            let app = NSApp();
            if app == nil {
                return Err("Failed to get NSApplication instance".to_string());
            }

            // Get the existing delegate
            let delegate: id = msg_send![app, delegate];
            if delegate == nil {
                return Err("NSApplication delegate is nil".to_string());
            }

            let delegate_class: *const Class = msg_send![delegate, class];
            
            let state_clone = self.state.clone();
            let state_ptr = Box::into_raw(Box::new(state_clone));
            let state_ptr_value = state_ptr as usize;

            static KEY: &[u8] = b"orbitx_dock_state\0";
            let key_ptr = KEY.as_ptr() as *const std::ffi::c_void;

            objc::rc::autoreleasepool(|| {
                let number: id = msg_send![class!(NSNumber), numberWithUnsignedLongLong: state_ptr_value];
                objc_setAssociatedObject(delegate, key_ptr, number, OBJC_ASSOCIATION_RETAIN);
            });
            
            let app_handle_clone = self.app_handle.clone();
            let app_handle_ptr = Box::into_raw(Box::new(app_handle_clone));
            let app_handle_ptr_value = app_handle_ptr as usize;
            
            static APP_HANDLE_KEY: &[u8] = b"orbitx_dock_app_handle\0";
            let app_handle_key_ptr = APP_HANDLE_KEY.as_ptr() as *const std::ffi::c_void;
            
            objc::rc::autoreleasepool(|| {
                let number: id = msg_send![class!(NSNumber), numberWithUnsignedLongLong: app_handle_ptr_value];
                objc_setAssociatedObject(delegate, app_handle_key_ptr, number, OBJC_ASSOCIATION_RETAIN);
            });

            let dock_menu_sel = sel!(applicationDockMenu:);
            let dock_menu_types = CStr::from_bytes_with_nul(b"@@:@\0").unwrap().as_ptr();
            let method_added = class_addMethod(
                delegate_class as *mut Class,
                dock_menu_sel,
                application_dock_menu_impl as *const std::ffi::c_void,
                dock_menu_types,
            );

            if method_added == NO {
                tracing::warn!("applicationDockMenu: method already exists or failed to add");
            } else {
                tracing::info!("Successfully added applicationDockMenu: method to delegate");
            }

            let switch_tab_sel = sel!(switchToTab:);
            let switch_tab_types = CStr::from_bytes_with_nul(b"v@:@\0").unwrap().as_ptr();
            let action_added = class_addMethod(
                delegate_class as *mut Class,
                switch_tab_sel,
                switch_to_tab_action as *const std::ffi::c_void,
                switch_tab_types,
            );

            if action_added == NO {
                tracing::warn!("switchToTab: method already exists or failed to add");
            } else {
                tracing::info!("Successfully added switchToTab: action method to delegate");
            }

            Ok(())
        }
    }

    pub fn refresh_menu(&self) -> Result<(), String> {
        tracing::debug!("Dock menu state updated, will refresh on next dock menu open");
        Ok(())
    }
}

unsafe fn build_dock_menu(state: &DockState) -> id {
        let _pool = NSAutoreleasePool::new(nil);

        let menu = NSMenu::alloc(nil);
        let menu: id = msg_send![menu, init];

        let tabs = match state.get_tabs() {
            Ok(tabs) => tabs,
            Err(e) => {
                tracing::error!("Failed to get tabs for dock menu: {}", e);
                return menu;
            }
        };
        
        let active_tab_id = state.get_active_tab_id().ok().flatten();

        if tabs.is_empty() {
            let title = NSString::alloc(nil);
            let title: id = msg_send![title, initWithUTF8String: "No Active Tabs\0".as_ptr()];
            let item = NSMenuItem::alloc(nil);
            let empty_key = NSString::alloc(nil);
            let empty_key: id = msg_send![empty_key, initWithUTF8String: "\0".as_ptr()];
            let item = item.initWithTitle_action_keyEquivalent_(title, sel!(noAction:), empty_key);
            let _: () = msg_send![item, setEnabled: NO];
            menu.addItem_(item);
        } else {
            let app = NSApp();
            let delegate: id = msg_send![app, delegate];
            
            for tab in tabs {
                let title = NSString::alloc(nil);
                let title_cstr = format!("{}\0", tab.title);
                let title: id = msg_send![title, initWithUTF8String: title_cstr.as_ptr()];
                let item = NSMenuItem::alloc(nil);
                let empty_key = NSString::alloc(nil);
                let empty_key: id = msg_send![empty_key, initWithUTF8String: "\0".as_ptr()];
                let item = item.initWithTitle_action_keyEquivalent_(title, sel!(switchToTab:), empty_key);
                let _: () = msg_send![item, setTarget: delegate];
                
                let tab_id_cstr = format!("{}\0", tab.id);
                let tab_id_str = NSString::alloc(nil);
                let tab_id_str: id = msg_send![tab_id_str, initWithUTF8String: tab_id_cstr.as_ptr()];
                let _: () = msg_send![item, setRepresentedObject: tab_id_str];
                
                if let Some(ref active_id) = active_tab_id {
                    if *active_id == tab.id {
                        let _: () = msg_send![item, setState: 1];
                    }
                }

                menu.addItem_(item);
            }
        }

    menu
}

extern "C" fn application_dock_menu_impl(this: &Object, _cmd: Sel, _sender: id) -> id {
    unsafe {
        static KEY: &[u8] = b"orbitx_dock_state\0";
        let key_ptr = KEY.as_ptr() as *const std::ffi::c_void;
        let number: id = objc_getAssociatedObject(this as *const _ as id, key_ptr);

        if number == nil {
            tracing::error!("No dock state found in delegate");
            return nil;
        }

        let state_ptr_value: usize = msg_send![number, unsignedLongLongValue];
        let state_ptr = state_ptr_value as *const DockState;

        if state_ptr.is_null() {
            tracing::error!("Invalid dock state pointer");
            return nil;
        }

        let state = &*state_ptr;
        build_dock_menu(state)
    }
}

impl<R: Runtime> Drop for MacOSDockMenu<R> {
    fn drop(&mut self) {
        unsafe {
            let app = NSApp();
            if app == nil {
                return;
            }

            let delegate: id = msg_send![app, delegate];
            if delegate == nil {
                return;
            }

            static KEY: &[u8] = b"orbitx_dock_state\0";
            let key_ptr = KEY.as_ptr() as *const std::ffi::c_void;
            
            static APP_HANDLE_KEY: &[u8] = b"orbitx_dock_app_handle\0";
            let app_handle_key_ptr = APP_HANDLE_KEY.as_ptr() as *const std::ffi::c_void;

            objc::rc::autoreleasepool(|| {
                let number: id = objc_getAssociatedObject(delegate, key_ptr);
                if number != nil {
                    let state_ptr_value: usize = msg_send![number, unsignedLongLongValue];
                    let state_ptr = state_ptr_value as *mut DockState;
                    if !state_ptr.is_null() {
                        let _boxed = Box::from_raw(state_ptr);
                    }
                }
                
                let app_handle_number: id = objc_getAssociatedObject(delegate, app_handle_key_ptr);
                if app_handle_number != nil {
                    let app_handle_ptr_value: usize = msg_send![app_handle_number, unsignedLongLongValue];
                    let app_handle_ptr = app_handle_ptr_value as *mut AppHandle<R>;
                    if !app_handle_ptr.is_null() {
                        let _boxed = Box::from_raw(app_handle_ptr);
                    }
                }

                objc_setAssociatedObject(delegate, key_ptr, nil, OBJC_ASSOCIATION_RETAIN);
                objc_setAssociatedObject(delegate, app_handle_key_ptr, nil, OBJC_ASSOCIATION_RETAIN);
            });
        }
    }
}

