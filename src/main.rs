// main.rs - Entry point, tray icon, and keyboard hook management
#![windows_subsystem = "windows"]

use parking_lot::Mutex;
use std::sync::Arc;
use std::ptr::null_mut;
use winapi::um::winuser::*;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::shared::windef::HHOOK;
use winapi::shared::minwindef::{LPARAM, WPARAM, LRESULT};
use tray_icon::{TrayIconBuilder, menu::Menu, menu::MenuItem};

mod symspell;
mod dictionary;
mod corrector;

use corrector::Corrector;

// Global state for the keyboard hook
static mut HOOK_HANDLE: HHOOK = null_mut();
static CORRECTOR: once_cell::sync::Lazy<Arc<Mutex<Corrector>>> = once_cell::sync::Lazy::new(|| {
    Arc::new(Mutex::new(Corrector::new()))
});

// Keyboard hook callback - called on every key press
unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb_struct = *(lparam as *const KBDLLHOOKSTRUCT);
        let vk_code = kb_struct.vkCode;
        let is_key_down = wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize;
        
        if is_key_down {
            let mut corrector = CORRECTOR.lock();
            
            // Check if autocorrect is enabled
            if !corrector.is_enabled() {
                return CallNextHookEx(HOOK_HANDLE, code, wparam, lparam);
            }
            
            // Handle the key press
            if corrector.handle_key(vk_code) {
                // Key was handled (correction was made), suppress it
                return 1;
            }
        }
    }
    
    CallNextHookEx(HOOK_HANDLE, code, wparam, lparam)
}

// Install the low-level keyboard hook
unsafe fn install_hook() -> Result<(), String> {
    let h_instance = GetModuleHandleW(null_mut());
    
    HOOK_HANDLE = SetWindowsHookExW(
        WH_KEYBOARD_LL,
        Some(keyboard_proc),
        h_instance,
        0
    );
    
    if HOOK_HANDLE.is_null() {
        return Err("Failed to install keyboard hook".to_string());
    }
    
    Ok(())
}

// Remove the keyboard hook
unsafe fn uninstall_hook() {
    if !HOOK_HANDLE.is_null() {
        UnhookWindowsHookEx(HOOK_HANDLE);
        HOOK_HANDLE = null_mut();
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the corrector (loads dictionaries)
    {
        let mut corrector = CORRECTOR.lock();
        corrector.initialize()?;
    }
    
    // Install keyboard hook
    unsafe {
        install_hook()?;
    }
    
    // Create tray icon menu
    let menu = Menu::new();
    let toggle_item = MenuItem::new("Disable Autocorrect", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    
    menu.append(&toggle_item)?;
    menu.append(&quit_item)?;
    
    // Create tray icon
    let icon = load_icon();
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Autocorrect - Enabled")
        .with_icon(icon)
        .build()?;
    
    println!("Autocorrect started. Running in system tray.");
    println!("Press Ctrl+C to quit.");
    
    // Menu event handling
    let menu_channel = tray_icon::menu::MenuEvent::receiver();
    
    // Message loop
    let mut msg = std::mem::MaybeUninit::uninit();
    unsafe {
        loop {
            // Check for menu events
            if let Ok(event) = menu_channel.try_recv() {
                if event.id == toggle_item.id() {
                    let mut corrector = CORRECTOR.lock();
                    corrector.toggle_enabled();
                    
                    let new_label = if corrector.is_enabled() {
                        "Disable Autocorrect"
                    } else {
                        "Enable Autocorrect"
                    };
                    toggle_item.set_text(new_label);
                    
                    let tooltip = if corrector.is_enabled() {
                        "Autocorrect - Enabled"
                    } else {
                        "Autocorrect - Disabled"
                    };
                    _tray_icon.set_tooltip(Some(tooltip))?;
                    
                    println!("Autocorrect {}", if corrector.is_enabled() { "enabled" } else { "disabled" });
                } else if event.id == quit_item.id() {
                    break;
                }
            }
            
            // Process Windows messages
            let ret = GetMessageW(msg.as_mut_ptr(), null_mut(), 0, 0);
            if ret <= 0 {
                break;
            }
            
            TranslateMessage(msg.as_ptr());
            DispatchMessageW(msg.as_ptr());
        }
        
        // Cleanup
        uninstall_hook();
    }
    
    println!("Autocorrect stopped.");
    Ok(())
}

// Load a simple icon for the tray (creates a basic icon)
fn load_icon() -> tray_icon::Icon {
    // Create a simple 16x16 RGBA icon (green checkmark-ish)
    let width = 16;
    let height = 16;
    let mut rgba = vec![0u8; (width * height * 4) as usize];
    
    // Simple green circle pattern
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) * 4) as usize;
            let dx = x as i32 - 8;
            let dy = y as i32 - 8;
            let dist_sq = dx * dx + dy * dy;
            
            if dist_sq < 36 {
                rgba[idx] = 50;      // R
                rgba[idx + 1] = 200; // G
                rgba[idx + 2] = 50;  // B
                rgba[idx + 3] = 255; // A
            } else {
                rgba[idx + 3] = 0; // Transparent
            }
        }
    }
    
    tray_icon::Icon::from_rgba(rgba, width, height).expect("Failed to create icon")
}

// Dependency for lazy initialization
mod once_cell {
    pub mod sync {
        pub struct Lazy<T> {
            cell: std::sync::OnceLock<T>,
            init: fn() -> T,
        }
        
        impl<T> Lazy<T> {
            pub const fn new(init: fn() -> T) -> Self {
                Self {
                    cell: std::sync::OnceLock::new(),
                    init,
                }
            }
        }
        
        impl<T> std::ops::Deref for Lazy<T> {
            type Target = T;
            
            fn deref(&self) -> &T {
                self.cell.get_or_init(self.init)
            }
        }
    }
}
