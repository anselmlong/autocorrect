//! Word tracking, correction logic, and undo buffer management.

use crate::dictionary::Dictionary;
use std::path::Path;
use std::time::Instant;

#[cfg(windows)]
use winapi::um::winuser::*;

/// Virtual key code for Backspace.
const VK_BACK: u32 = 0x08;
/// Virtual key code for Enter/Return.
const VK_RETURN: u32 = 0x0D;
/// Virtual key code for Space.
const VK_SPACE: u32 = 0x20;
/// Virtual key code for Control.
const VK_CONTROL: u32 = 0x11;

/// Stores information about a correction for potential undo.
#[derive(Debug, Clone)]
struct UndoState {
    original_word: String,
    corrected_word: String,
    timestamp: Instant,
}

/// The main autocorrection engine.
pub struct Corrector {
    dictionary: Dictionary,
    current_word: String,
    enabled: bool,
    max_edit_distance: i32,
    undo_timeout_seconds: u64,
    undo_buffer: Option<UndoState>,
    ctrl_pressed: bool,
    last_correction_time: Option<Instant>,
}

impl Corrector {
    pub fn new() -> Self {
        Self::new_with_settings(2, true, 5)
    }

    pub fn new_with_config(config: &crate::config::Config) -> Self {
        Self::new_with_settings(
            config.max_edit_distance,
            config.enabled_by_default,
            config.undo_timeout_seconds,
        )
    }

    fn new_with_settings(max_edit_distance: i32, enabled: bool, undo_timeout_seconds: u64) -> Self {
        let max_edit_distance = max_edit_distance.max(0);

        Self {
            dictionary: Dictionary::new(),
            current_word: String::new(),
            enabled,
            max_edit_distance,
            undo_timeout_seconds,
            undo_buffer: None,
            ctrl_pressed: false,
            last_correction_time: None,
        }
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.initialize_with_dictionary(None)
    }

    pub fn initialize_with_dictionary(
        &mut self,
        dictionary_path: Option<&std::path::Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.dictionary.load_from_path(dictionary_path)?;
        Ok(())
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn handle_key(&mut self, vk_code: u32) -> bool {
        #[cfg(not(windows))]
        {
            let _ = vk_code;
            return false;
        }

        #[cfg(windows)]
        {
            if vk_code == VK_CONTROL {
                self.ctrl_pressed = true;
                return false;
            }

            if self.ctrl_pressed && vk_code == 0x5A {
                // Z key
                return self.handle_undo();
            }

            match vk_code {
                VK_BACK => {
                    self.handle_backspace();
                    false
                }
                VK_SPACE | VK_RETURN => {
                    self.handle_word_end();
                    false
                }
                _ if Self::is_punctuation(vk_code) => {
                    self.handle_word_end();
                    false
                }
                _ if Self::is_letter(vk_code) => {
                    self.handle_letter(vk_code);
                    false
                }
                _ => {
                    self.current_word.clear();
                    false
                }
            }
        }
    }

    fn handle_letter(&mut self, vk_code: u32) {
        if self.undo_buffer.is_some() {
            if let Some(correction_time) = self.last_correction_time {
                if correction_time.elapsed().as_secs() > 2 {
                    self.undo_buffer = None;
                }
            }
        }

        #[cfg(windows)]
        let uppercase = {
            let shift_pressed = unsafe { GetAsyncKeyState(VK_SHIFT as i32) < 0 };
            let caps_lock = unsafe { GetKeyState(VK_CAPITAL as i32) & 1 != 0 };
            shift_pressed ^ caps_lock
        };
        #[cfg(not(windows))]
        let uppercase = false;

        if let Some(ch) = Self::vk_to_char(vk_code, uppercase) {
            self.current_word.push(ch);
        }
    }

    fn handle_backspace(&mut self) {
        if !self.current_word.is_empty() {
            self.current_word.pop();
        }
    }

    fn handle_word_end(&mut self) {
        if self.current_word.is_empty() {
            return;
        }

        let word_lower = self.current_word.to_lowercase();

        if let Some(correction) = self.dictionary.get_correction(&word_lower) {
            self.undo_buffer = Some(UndoState {
                original_word: self.current_word.clone(),
                corrected_word: correction.clone(),
                timestamp: Instant::now(),
            });
            self.last_correction_time = Some(Instant::now());

            self.replace_word(&correction);

            println!("Corrected: '{}' -> '{}'", self.current_word, correction);
        }

        self.current_word.clear();
    }

    fn replace_word(&self, correction: &str) {
        #[cfg(windows)]
        unsafe {
            let backspace_count = self.current_word.chars().count();

            for _ in 0..backspace_count {
                Self::send_key(VK_BACK as u16, true);
                Self::send_key(VK_BACK as u16, false);
            }

            for ch in correction.chars() {
                Self::send_char(ch);
            }
        }
        #[cfg(not(windows))]
        {
            let _ = correction;
        }
    }

    fn handle_undo(&mut self) -> bool {
        if let Some(undo) = &self.undo_buffer {
            if undo.timestamp.elapsed().as_secs() < self.undo_timeout_seconds {
                #[cfg(windows)]
                {
                    let correction_len = undo.corrected_word.chars().count();
                    unsafe {
                        for _ in 0..correction_len {
                            Self::send_key(VK_BACK as u16, true);
                            Self::send_key(VK_BACK as u16, false);
                        }

                        for ch in undo.original_word.chars() {
                            Self::send_char(ch);
                        }
                    }
                }

                println!(
                    "Undo: '{}' -> '{}'",
                    undo.corrected_word, undo.original_word
                );

                self.undo_buffer = None;
                return true;
            }
        }

        false
    }

    #[cfg(windows)]
    unsafe fn send_key(vk: u16, key_down: bool) {
        let mut input = INPUT {
            type_: INPUT_KEYBOARD,
            u: std::mem::zeroed(),
        };

        *input.u.ki_mut() = KEYBDINPUT {
            wVk: vk,
            wScan: 0,
            dwFlags: if key_down { 0 } else { KEYEVENTF_KEYUP },
            time: 0,
            dwExtraInfo: 0,
        };

        SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);

        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    #[cfg(windows)]
    unsafe fn send_char(ch: char) {
        if ch.is_ascii_alphabetic() {
            let vk = ch.to_ascii_uppercase() as u16;
            let shift = ch.is_uppercase();

            if shift {
                Self::send_key(VK_SHIFT as u16, true);
            }

            Self::send_key(vk, true);
            Self::send_key(vk, false);

            if shift {
                Self::send_key(VK_SHIFT as u16, false);
            }
        } else {
            let mut input = INPUT {
                type_: INPUT_KEYBOARD,
                u: std::mem::zeroed(),
            };

            *input.u.ki_mut() = KEYBDINPUT {
                wVk: 0,
                wScan: ch as u16,
                dwFlags: KEYEVENTF_UNICODE,
                time: 0,
                dwExtraInfo: 0,
            };

            SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);

            input.u.ki_mut().dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
            SendInput(1, &mut input, std::mem::size_of::<INPUT>() as i32);
        }

        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    fn is_letter(vk_code: u32) -> bool {
        (0x41..=0x5A).contains(&vk_code)
    }

    fn is_punctuation(vk_code: u32) -> bool {
        matches!(
            vk_code,
            0xBC | 0xBE | 0xBF | 0xBA | 0xDE | 0xDB | 0xDD | 0xC0 | 0xBD | 0xBB
        )
    }

    fn vk_to_char(vk_code: u32, uppercase: bool) -> Option<char> {
        if (0x41..=0x5A).contains(&vk_code) {
            let ch = (vk_code - 0x41 + b'a' as u32) as u8 as char;
            Some(if uppercase {
                ch.to_ascii_uppercase()
            } else {
                ch
            })
        } else {
            None
        }
    }
}

impl Drop for Corrector {
    fn drop(&mut self) {
        self.ctrl_pressed = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_letter() {
        assert!(Corrector::is_letter(0x41));
        assert!(Corrector::is_letter(0x5A));
        assert!(!Corrector::is_letter(0x20));
    }

    #[test]
    fn test_vk_to_char() {
        assert_eq!(Corrector::vk_to_char(0x41, false), Some('a'));
        assert_eq!(Corrector::vk_to_char(0x41, true), Some('A'));
        assert_eq!(Corrector::vk_to_char(0x5A, false), Some('z'));
    }
}
