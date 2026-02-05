// corrector.rs - Word tracking, correction logic, and undo buffer
// Handles keyboard input, builds words, triggers corrections, and manages undo

use crate::dictionary::Dictionary;
use std::time::Instant;
use winapi::um::winuser::*;

const VK_BACK: u32 = 0x08;
const VK_RETURN: u32 = 0x0D;
const VK_SPACE: u32 = 0x20;
const VK_CONTROL: u32 = 0x11;

#[derive(Debug, Clone)]
struct UndoState {
    original_word: String,
    corrected_word: String,
    timestamp: Instant,
}

pub struct Corrector {
    dictionary: Dictionary,
    current_word: String,
    enabled: bool,
    undo_buffer: Option<UndoState>,
    ctrl_pressed: bool,
    last_correction_time: Option<Instant>,
}

impl Corrector {
    pub fn new() -> Self {
        Self {
            dictionary: Dictionary::new(),
            current_word: String::new(),
            enabled: true,
            undo_buffer: None,
            ctrl_pressed: false,
            last_correction_time: None,
        }
    }
    
    /// Initialize the corrector (load dictionaries)
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.dictionary.load()?;
        Ok(())
    }
    
    /// Check if autocorrect is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    /// Toggle enabled state
    pub fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
    }
    
    /// Handle a key press - returns true if the key should be suppressed
    pub fn handle_key(&mut self, vk_code: u32) -> bool {
        // Track Ctrl key state
        if vk_code == VK_CONTROL {
            self.ctrl_pressed = true;
            return false;
        }
        
        // Check for Ctrl+Z (undo)
        if self.ctrl_pressed && vk_code == 0x5A { // Z key
            return self.handle_undo();
        }
        
        // Handle different key types
        match vk_code {
            VK_BACK => {
                self.handle_backspace();
                false
            },
            VK_SPACE | VK_RETURN => {
                self.handle_word_end();
                false
            },
            _ if Self::is_punctuation(vk_code) => {
                self.handle_word_end();
                false
            },
            _ if Self::is_letter(vk_code) => {
                self.handle_letter(vk_code);
                false
            },
            _ => {
                // Other keys (arrows, function keys, etc.) end the word
                self.current_word.clear();
                false
            }
        }
    }
    
    /// Handle a letter being typed
    fn handle_letter(&mut self, vk_code: u32) {
        // Clear undo buffer if we've moved past the correction
        if self.undo_buffer.is_some() {
            if let Some(correction_time) = self.last_correction_time {
                if correction_time.elapsed().as_secs() > 2 {
                    self.undo_buffer = None;
                }
            }
        }
        
        // Check if Shift is pressed for uppercase
        let shift_pressed = unsafe { GetAsyncKeyState(VK_SHIFT as i32) < 0 };
        let caps_lock = unsafe { GetKeyState(VK_CAPITAL as i32) & 1 != 0 };
        let uppercase = shift_pressed ^ caps_lock;
        
        // Convert virtual key code to character
        if let Some(ch) = Self::vk_to_char(vk_code, uppercase) {
            self.current_word.push(ch);
        }
    }
    
    /// Handle backspace key
    fn handle_backspace(&mut self) {
        if !self.current_word.is_empty() {
            self.current_word.pop();
        }
    }
    
    /// Handle end of word (space, punctuation, enter)
    fn handle_word_end(&mut self) {
        if self.current_word.is_empty() {
            return;
        }
        
        // Check if word needs correction
        let word_lower = self.current_word.to_lowercase();
        
        if let Some(correction) = self.dictionary.get_correction(&word_lower) {
            // Store undo state
            self.undo_buffer = Some(UndoState {
                original_word: self.current_word.clone(),
                corrected_word: correction.clone(),
                timestamp: Instant::now(),
            });
            self.last_correction_time = Some(Instant::now());
            
            // Perform the correction
            self.replace_word(&correction);
            
            println!("Corrected: '{}' -> '{}'", self.current_word, correction);
        }
        
        // Clear current word
        self.current_word.clear();
    }
    
    /// Replace the current word with corrected version
    fn replace_word(&self, correction: &str) {
        unsafe {
            // Simulate backspaces to delete the misspelled word
            let backspace_count = self.current_word.chars().count();
            
            for _ in 0..backspace_count {
                Self::send_key(VK_BACK as u16, true);
                Self::send_key(VK_BACK as u16, false);
            }
            
            // Type the corrected word
            for ch in correction.chars() {
                Self::send_char(ch);
            }
        }
    }
    
    /// Handle undo (Ctrl+Z after correction)
    fn handle_undo(&mut self) -> bool {
        if let Some(undo) = &self.undo_buffer {
            // Only undo if it was recent (within 5 seconds)
            if undo.timestamp.elapsed().as_secs() < 5 {
                // Delete the corrected word
                let correction_len = undo.corrected_word.chars().count();
                unsafe {
                    for _ in 0..correction_len {
                        Self::send_key(VK_BACK as u16, true);
                        Self::send_key(VK_BACK as u16, false);
                    }
                    
                    // Type the original word
                    for ch in undo.original_word.chars() {
                        Self::send_char(ch);
                    }
                }
                
                println!("Undo: '{}' -> '{}'", undo.corrected_word, undo.original_word);
                
                self.undo_buffer = None;
                return true; // Suppress the Ctrl+Z
            }
        }
        
        false
    }
    
    /// Send a key press/release
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
        
        // Small delay to ensure key is processed
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    
    /// Send a character (handles Unicode)
    unsafe fn send_char(ch: char) {
        // For ASCII letters, use VK codes
        if ch.is_ascii_alphabetic() {
            let vk = if ch.is_ascii_uppercase() {
                ch.to_ascii_uppercase() as u16
            } else {
                ch.to_ascii_uppercase() as u16
            };
            
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
            // For other characters, use Unicode input
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
    
    /// Check if a virtual key code is a letter
    fn is_letter(vk_code: u32) -> bool {
        (0x41..=0x5A).contains(&vk_code) // A-Z
    }
    
    /// Check if a virtual key code is punctuation that ends a word
    fn is_punctuation(vk_code: u32) -> bool {
        matches!(vk_code,
            0xBC | // Comma
            0xBE | // Period
            0xBF | // Slash
            0xBA | // Semicolon
            0xDE | // Quote
            0xDB | // Open bracket
            0xDD | // Close bracket
            0xC0 | // Backtick
            0xBD | // Minus
            0xBB   // Equals
        )
    }
    
    /// Convert virtual key code to character
    fn vk_to_char(vk_code: u32, uppercase: bool) -> Option<char> {
        if (0x41..=0x5A).contains(&vk_code) {
            let ch = (vk_code - 0x41 + b'a' as u32) as u8 as char;
            Some(if uppercase { ch.to_ascii_uppercase() } else { ch })
        } else {
            None
        }
    }
}

// Implement Drop to reset Ctrl state
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
        assert!(Corrector::is_letter(0x41)); // A
        assert!(Corrector::is_letter(0x5A)); // Z
        assert!(!Corrector::is_letter(0x20)); // Space
    }
    
    #[test]
    fn test_vk_to_char() {
        assert_eq!(Corrector::vk_to_char(0x41, false), Some('a'));
        assert_eq!(Corrector::vk_to_char(0x41, true), Some('A'));
        assert_eq!(Corrector::vk_to_char(0x5A, false), Some('z'));
    }
}
