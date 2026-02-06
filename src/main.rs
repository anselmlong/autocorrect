//! Autocorrect - A Windows system tray application for real-time spell correction.
//!
//! # Overview
//!
//! This application provides system-wide autocorrection by:
//! - Installing a low-level keyboard hook to intercept keystrokes
//! - Tracking words as they are typed
//! - Using the SymSpell algorithm to suggest corrections
//! - Replacing misspelled words automatically
//! - Providing an undo mechanism (Ctrl+Z within 5 seconds)
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐
//! │  User Types     │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │ Keyboard Hook   │ ← Intercepts all keystrokes (main.rs)
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  Corrector      │ ← Builds words, triggers corrections (corrector.rs)
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │   SymSpell      │ ← Finds suggestions using edit distance (symspell.rs)
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  Dictionary     │ ← Loads word frequencies (dictionary.rs)
//! └─────────────────┘
//! ```
//!
//! # Modules
//!
//! - `main.rs`: Entry point, Windows message loop, and system tray
//! - `corrector.rs`: Word tracking, correction logic, and undo buffer
//! - `symspell.rs`: Fast spell correction using the SymSpell algorithm
//! - `dictionary.rs`: Dictionary loading (built-in + personal)
//! - `trigram.rs`: Context-based language model (optional enhancement)
//!
//! # System Tray
//!
//! The application runs minimized in the system tray with:
//! - Toggle to enable/disable autocorrection
//! - Visual indicator (green icon = enabled)
//!
//! # Keyboard Hook
//!
//! Uses Windows `SetWindowsHookExW` with `WH_KEYBOARD_LL` to capture all
//! keystrokes system-wide. The hook runs in the main thread and must
//! be uninstalled on shutdown to avoid leaving the keyboard unresponsive.

#![windows_subsystem = "windows"]

use clap::Parser;
use parking_lot::Mutex;
use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::Arc;
use std::ptr::null_mut;
use winapi::um::winuser::{MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK};
use winapi::um::winuser::*;
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::shared::windef::HHOOK;
use winapi::shared::minwindef::{LPARAM, WPARAM, LRESULT};
use tray_icon::{TrayIconBuilder, menu::Menu, menu::MenuItem};

mod symspell;
mod dictionary;
mod corrector;
mod trigram;
mod config;
mod updater;

use config::Config;
use corrector::Corrector;
use updater::Updater;

#[derive(Parser, Debug)]
#[command(name = "autocorrect")]
#[command(about = "Windows system-wide autocorrect tool")]
#[command(version)]
struct Args {
    /// Start with autocorrect disabled
    #[arg(long)]
    disabled: bool,

    /// Custom dictionary file path
    #[arg(short, long)]
    dictionary: Option<std::path::PathBuf>,

    /// Run in console mode (don't hide console)
    #[arg(long)]
    console: bool,

    /// Check for updates
    #[arg(long)]
    check_update: bool,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetConsoleWindow() -> *mut std::ffi::c_void;
}

/// Global handle to the low-level keyboard hook.
///
/// # Safety
/// This is a raw pointer that must only be accessed from the main thread.
/// It's set during `install_hook()` and cleared in `uninstall_hook()`.
static mut HOOK_HANDLE: HHOOK = null_mut();

/// Global autocorrector instance, lazily initialized.
///
/// Uses `parking_lot::Mutex` for fast, compact locking without poisoning.
/// The `once_cell::sync::Lazy` ensures thread-safe one-time initialization.
static CORRECTOR: std::sync::OnceLock<Arc<Mutex<Corrector>>> = std::sync::OnceLock::new();

fn corrector() -> &'static Arc<Mutex<Corrector>> {
    CORRECTOR
        .get()
        .expect("corrector must be initialized in main before hook installation")
}

/// Convert UTF-8 Rust strings to UTF-16 and display a modal Windows message box.
fn show_dialog(title: &str, message: &str, icon_flag: u32) {
    let title_wide: Vec<u16> = OsStr::new(title).encode_wide().chain(Some(0)).collect();
    let message_wide: Vec<u16> = OsStr::new(message).encode_wide().chain(Some(0)).collect();

    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message_wide.as_ptr(),
            title_wide.as_ptr(),
            MB_OK | icon_flag,
        );
    }
}

fn show_error_dialog(title: &str, message: &str) {
    show_dialog(title, message, MB_ICONERROR);
}

fn show_warning_dialog(title: &str, message: &str) {
    show_dialog(title, message, MB_ICONWARNING);
}

#[allow(dead_code)]
fn show_info_dialog(title: &str, message: &str) {
    show_dialog(title, message, MB_ICONINFORMATION);
}

fn hide_console_window() {
    unsafe {
        let hwnd = GetConsoleWindow();
        if !hwnd.is_null() {
            ShowWindow(hwnd as _, SW_HIDE);
        }
    }
}

/// Low-level keyboard hook callback - called by Windows on every key event.
///
/// This function intercepts all keyboard input system-wide. It:
/// 1. Checks if autocorrection is enabled
/// 2. Passes key events to the `Corrector` for word building
/// 3. Suppresses keys that trigger corrections (returns 1)
/// 4. Passes through all other keys (calls `CallNextHookEx`)
///
/// # Safety
/// Called by Windows with raw pointers. The `lparam` is cast to `KBDLLHOOKSTRUCT`.
/// Must not panic or allocate excessively as it runs on the hook thread.
///
/// # Arguments
/// * `code` - Hook code; if >= 0, process the message
/// * `wparam` - Message identifier (WM_KEYDOWN, WM_KEYUP, etc.)
/// * `lparam` - Pointer to `KBDLLHOOKSTRUCT` with key details
///
/// # Returns
/// * `1` - Suppress the key (correction was made)
/// * Other - Result from `CallNextHookEx` (pass through)
unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        let kb_struct = *(lparam as *const KBDLLHOOKSTRUCT);
        let vk_code = kb_struct.vkCode;
        let is_key_down = wparam == WM_KEYDOWN as usize || wparam == WM_SYSKEYDOWN as usize;
        
        if is_key_down {
            let mut corrector = corrector().lock();
            
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

/// Install the low-level keyboard hook.
///
/// Uses `SetWindowsHookExW` with `WH_KEYBOARD_LL` to capture all keyboard
/// input system-wide. The hook procedure runs in the context of the
/// installing thread (this application's main thread).
///
/// # Safety
/// Unsafe because it calls Windows API with raw pointers. The hook handle
/// is stored in `HOOK_HANDLE` and must be uninstalled before exit.
///
/// # Errors
/// Returns an error if `SetWindowsHookExW` fails (returns null).
/// This typically happens if the application lacks sufficient privileges.
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

/// Remove the keyboard hook and restore normal keyboard input.
///
/// # Safety
/// Unsafe because it accesses `HOOK_HANDLE`. Safe to call multiple times.
/// Must be called before application exit to avoid leaving the keyboard
/// in an inconsistent state.
unsafe fn uninstall_hook() {
    if !HOOK_HANDLE.is_null() {
        UnhookWindowsHookEx(HOOK_HANDLE);
        HOOK_HANDLE = null_mut();
    }
}

/// Application entry point.
///
/// # Initialization Sequence
/// 1. Initialize the corrector (load dictionaries)
/// 2. Install the low-level keyboard hook
/// 3. Create the system tray icon and menu
/// 4. Enter the Windows message loop
///
/// # Shutdown
/// - Menu "Quit" selection breaks the message loop
/// - Ctrl+C (console) breaks the message loop
/// - Hook is uninstalled, resources cleaned up
///
/// # Errors
/// Returns an error if dictionary loading fails or the hook cannot be installed.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.check_update {
        match Updater::check_and_update() {
            Ok(updated) => {
                if updated {
                    println!("Update successful! Please restart the application.");
                }
            }
            Err(e) => {
                eprintln!("Update failed: {}", e);
            }
        }

        return Ok(());
    }

    if !args.console {
        hide_console_window();
    }

    let mut config = Config::load()?;

    if args.disabled {
        config.enabled_by_default = false;
    }

    // Persist defaults so users get a concrete config.toml on first run.
    if let Err(err) = config.save() {
        eprintln!("Failed to persist config defaults: {err}");
    }

    let configured_corrector = Arc::new(Mutex::new(Corrector::new_with_config(&config)));
    CORRECTOR
        .set(configured_corrector)
        .map_err(|_| "Corrector was already initialized")?;

    // Initialize the corrector (loads dictionaries)
    {
        let mut corrector = corrector().lock();
        if let Err(e) = corrector.initialize_with_dictionary(args.dictionary.as_deref()) {
            println!("Failed to initialize corrector: {}", e);
            show_error_dialog(
                "Autocorrect Error",
                &format!("Failed to load dictionary: {}", e),
            );
            return Err(e.into());
        }
    }
    
    // Install keyboard hook
    unsafe {
        if let Err(e) = install_hook() {
            println!("Failed to install keyboard hook: {}", e);
            show_error_dialog(
                "Autocorrect Error",
                &format!("Failed to install keyboard hook: {}", e),
            );
            return Err(e.into());
        }
    }
    
    // Create tray icon menu
    let menu = Menu::new();
    let toggle_item = MenuItem::new(
        if config.enabled_by_default {
            "Disable Autocorrect"
        } else {
            "Enable Autocorrect"
        },
        true,
        None,
    );
    let quit_item = MenuItem::new("Quit", true, None);
    
    if let Err(e) = menu.append(&toggle_item) {
        println!("Failed to append toggle menu item: {}", e);
        show_error_dialog(
            "Autocorrect Error",
            &format!("Failed to create tray menu: {}", e),
        );
        unsafe {
            uninstall_hook();
        }
        return Err(e.into());
    }
    if let Err(e) = menu.append(&quit_item) {
        println!("Failed to append quit menu item: {}", e);
        show_error_dialog(
            "Autocorrect Error",
            &format!("Failed to create tray menu: {}", e),
        );
        unsafe {
            uninstall_hook();
        }
        return Err(e.into());
    }
    
    // Create tray icon
    let icon = load_icon();
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip(if config.enabled_by_default {
            "Autocorrect - Enabled"
        } else {
            "Autocorrect - Disabled"
        })
        .with_icon(icon)
        .build()
        .map_err(|e| {
            println!("Failed to create tray icon: {}", e);
            show_error_dialog(
                "Autocorrect Error",
                &format!("Failed to create system tray icon: {}", e),
            );
            unsafe {
                uninstall_hook();
            }
            e
        })?;
    
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
                    let mut corrector = corrector().lock();
                    corrector.toggle_enabled();
                    config.enabled_by_default = corrector.is_enabled();

                    if let Err(err) = config.save() {
                        eprintln!("Failed to save config: {err}");
                    }
                    
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
                    if let Err(e) = _tray_icon.set_tooltip(Some(tooltip)) {
                        println!("Failed to update tray tooltip: {}", e);
                        show_warning_dialog(
                            "Autocorrect Warning",
                            &format!(
                                "Autocorrect state changed, but tray tooltip could not be updated: {}",
                                e
                            ),
                        );
                    }
                    
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

/// Create the application icon for the system tray.
///
/// Generates a simple 16x16 RGBA icon with a green circle pattern.
/// This is used as the visual indicator in the Windows system tray.
///
/// # Returns
/// A `tray_icon::Icon` suitable for use with `TrayIconBuilder`.
///
/// # Panics
/// Panics if the icon data is invalid (should never happen with valid RGBA).
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
