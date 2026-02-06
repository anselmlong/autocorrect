//! Word tracking, correction logic, and undo buffer management.
//!
//! This module is the core of the autocorrection system. It:
//! - Tracks words as the user types
//! - Detects word boundaries (space, punctuation, enter)
//! - Queries the SymSpell dictionary for corrections
//! - Replaces misspelled words with corrections
//! - Provides an undo mechanism (Ctrl+Z within 5 seconds)
//!
//! # Word Lifecycle
//!
//! ```text
//! User Types:    t  e  h  [space]
//!                ↓  ↓  ↓     ↓
//! Current Word: "t" → "te" → "teh" → TRIGGER CORRECTION
//!                                      ↓
//!                                  Check Dictionary
//!                                      ↓
//!                                  Replace with "the"
//! ```
//!
//! # Undo Mechanism
//!
//! After a correction, the original word is stored in `undo_buffer`.
//! If the user presses Ctrl+Z within 5 seconds, the correction is undone
//! and the original word is restored.

use crate::dictionary::Dictionary;
use std::time::Instant;
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
///
/// The undo buffer retains corrections for 5 seconds, allowing users
/// to press Ctrl+Z immediately after an unwanted correction.
#[derive(Debug, Clone)]
struct UndoState {
    /// The original (misspelled) word before correction.
    original_word: String,
    /// The word that replaced the original.
    corrected_word: String,
    /// When the correction occurred (for timeout calculation).
    timestamp: Instant,
}

/// The main autocorrection engine.
///
/// Tracks keystrokes to build words, detects word boundaries, and
/// triggers corrections using the SymSpell algorithm. Manages the
/// undo buffer for reverting unwanted corrections.
pub struct Corrector {
    /// Dictionary containing word frequencies and SymSpell instance.
    dictionary: Dictionary,
    /// The word currently being typed (since last word boundary).
    current_word: String,
    /// Whether autocorrection is enabled.
    enabled: bool,
    /// Maximum edit distance for correction lookup.
    max_edit_distance: i32,
    /// Timeout window for undo after a correction.
    undo_timeout_seconds: u64,
    /// Stores the last correction for potential undo.
    undo_buffer: Option<UndoState>,
    /// Tracks if Ctrl is currently held (for undo detection).
    ctrl_pressed: bool,
    /// When the last correction occurred (for undo timeout).
    last_correction_time: Option<Instant>,
}

impl Corrector {
    /// Create a new `Corrector` with default settings.
    ///
    /// Initializes the SymSpell algorithm with a max edit distance of 2
    /// and an empty trigram model for context-based scoring.
    ///
    /// # Example
    /// ```rust
    /// let corrector = Corrector::new();
    /// ```
    pub fn new() -> Self {
        Self::new_with_settings(2, true, 5)
    }

    /// Create a new `Corrector` using runtime configuration.
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
    
    /// Initialize the corrector by loading dictionaries.
    ///
    /// Loads both the built-in dictionary and the user's personal dictionary.
    /// Must be called before the corrector can make suggestions.
    ///
    /// # Errors
    /// Returns an error if dictionary files cannot be read or parsed.
    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.initialize_with_dictionary(None)
    }

    /// Initialize the corrector with an optional custom built-in dictionary path.
    pub fn initialize_with_dictionary(
        &mut self,
        dictionary_path: Option<&Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.dictionary.load_from_path(dictionary_path)?;
        Ok(())
    }

    /// Explicitly set whether autocorrection is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if autocorrection is currently enabled.
    ///
    /// When disabled, keystrokes are passed through without processing.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Toggle autocorrection on/off.
    ///
    /// Changes the enabled state and updates the system tray tooltip.
    pub fn toggle_enabled(&mut self) {
        self.enabled = !self.enabled;
    }

    /// Process a keystroke from the keyboard hook.
    ///
    /// This is the main entry point for keyboard input. It:
    /// - Tracks Ctrl key state for undo detection
    /// - Builds words from letter keystrokes
    /// - Detects word boundaries (space, punctuation, enter)
    /// - Triggers corrections at word boundaries
    ///
    /// # Arguments
    /// * `vk_code` - The virtual key code from Windows
    ///
    /// # Returns
    /// * `true` - The key was consumed (correction made, suppress the key)
    /// * `false` - The key should be passed through to the application
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
    
    /// Handle a letter key being pressed.
    ///
    /// Adds the character to the current word being built. Handles
    /// uppercase/lowercase based on Shift and Caps Lock state.
    ///
    /// # Arguments
    /// * `vk_code` - Virtual key code (A-Z range expected)
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
    
    /// Handle the Backspace key.
    ///
    /// Removes the last character from the current word being built.
    fn handle_backspace(&mut self) {
        if !self.current_word.is_empty() {
            self.current_word.pop();
        }
    }

    /// Handle word boundary (space, punctuation, or enter).
    ///
    /// Called when the user has finished typing a word. This method:
    /// 1. Checks if the word exists in the dictionary
    /// 2. If not, finds the best correction using SymSpell
    /// 3. Replaces the misspelled word with the correction
    /// 4. Stores the original word for potential undo
    fn handle_word_end(&mut self) {
        if self.current_word.is_empty() {
            return;
        }

        // Check if word needs correction
        let word_lower = self.current_word.to_lowercase();

        // Use dictionary to get correction (it uses SymSpell internally)
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
    
    /// Replace the current word with a corrected version.
    ///
    /// Simulates backspace keystrokes to delete the misspelled word,
    /// then types the corrected word character by character.
    ///
    /// # Arguments
    /// * `correction` - The corrected word to type
    ///
    /// # Safety
    /// This function uses `unsafe` to call Windows API for simulating keystrokes.
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
    
    /// Undo the last correction.
    ///
    /// Called when Ctrl+Z is pressed within 5 seconds of a correction.
    /// Restores the original word by deleting the correction and retyping
    /// the original misspelled word.
    ///
    /// # Returns
    /// * `true` - The undo was performed (suppress the Ctrl+Z keystroke)
    /// * `false` - No undo available or timeout exceeded
    ///
    /// # Safety
    /// This function uses `unsafe` to call Windows API for simulating keystrokes.
    fn handle_undo(&mut self) -> bool {
        if let Some(undo) = &self.undo_buffer {
            // Only undo if it was recent (within 5 seconds)
            if undo.timestamp.elapsed().as_secs() < self.undo_timeout_seconds {
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
    
    /// Send a virtual key press or release event.
    ///
    /// Uses Windows `SendInput` API to simulate keyboard input.
    /// Includes a 1ms delay to ensure the key is processed.
    ///
    /// # Arguments
    /// * `vk` - Virtual key code
    /// * `key_down` - `true` for key press, `false` for key release
    ///
    /// # Safety
    /// This is an `unsafe` function because it calls Windows API.
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
    
    /// Send a character using simulated keystrokes.
    ///
    /// For ASCII letters, simulates the appropriate key with optional Shift.
    /// For other characters, uses Unicode input via `KEYEVENTF_UNICODE`.
    ///
    /// # Arguments
    /// * `ch` - The character to type
    ///
    /// # Safety
    /// This is an `unsafe` function because it calls Windows API.
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
    
    /// Check if a virtual key code represents a letter (A-Z).
    fn is_letter(vk_code: u32) -> bool {
        (0x41..=0x5A).contains(&vk_code) // A-Z
    }

    /// Check if a virtual key code represents word-ending punctuation.
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

    /// Convert a virtual key code to a character.
    ///
    /// Only handles A-Z keys. Returns `None` for other keys.
    ///
    /// # Arguments
    /// * `vk_code` - Virtual key code (expected to be in A-Z range)
    /// * `uppercase` - Whether to return uppercase or lowercase
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
