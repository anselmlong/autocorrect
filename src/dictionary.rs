//! Dictionary loading and management for the autocorrection system.
//!
//! This module handles loading and managing two types of dictionaries:
//! - **Built-in dictionary**: A comprehensive list of common English words with frequencies
//! - **Personal dictionary**: User-defined words that should never be corrected
//!
//! # Dictionary Format
//!
//! Dictionary files use a simple text format:
//! ```text
//! # Comments start with #
//! word frequency
//! the 1000000
//! be 500000
//! hello 15000
//! ```
//!
//! If frequency is omitted, it defaults to 1.
//!
//! # Fallback Dictionary
//!
//! If no dictionary file is found at `dictionary/words.txt`, a built-in
//! fallback dictionary of common English words is used. This ensures the
//! application works even without external dictionary files.
//!
//! # Personal Dictionary
//!
//! Personal words are stored in `%APPDATA%/Autocorrect/personal_dictionary.txt`.
//! These words are given very high frequency (1,000,000) to ensure they are
//! always preferred over similar dictionary words.

use crate::symspell::SymSpell;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

// Embed the dictionary file at compile time
// If the file doesn't exist, this will fail at compile time with a clear error
const EMBEDDED_DICTIONARY: &str = include_str!("../dictionary/words.txt");

/// Manages dictionary loading and word storage.
///
/// The dictionary system consists of:
/// - A SymSpell instance containing all words and their frequencies
/// - A path to the personal dictionary file
pub struct Dictionary {
    /// The SymSpell instance containing all loaded words.
    symspell: SymSpell,
    /// Path to the user's personal dictionary file.
    personal_dict_path: PathBuf,
}

impl Dictionary {
    /// Create a new dictionary instance.
    ///
    /// Initializes a SymSpell with max edit distance of 2 and determines
    /// the path for the personal dictionary file.
    ///
    /// # Example
    /// ```rust
    /// let dict = Dictionary::new();
    /// ```
    pub fn new() -> Self {
        Self {
            symspell: SymSpell::new(2),
            personal_dict_path: Self::get_personal_dict_path(),
        }
    }

    /// Load both built-in and personal dictionaries.
    ///
    /// This method:
    /// 1. Loads the built-in dictionary from `dictionary/words.txt` (or uses fallback)
    /// 2. Loads the personal dictionary from the user's AppData folder
    /// 3. Creates an empty personal dictionary if it doesn't exist
    ///
    /// # Errors
    /// Returns an error if dictionary files cannot be read.
    ///
    /// # Example
    /// ```rust
    /// let mut dict = Dictionary::new();
    /// dict.load()?;
    /// ```
    pub fn load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.load_from_path(None)
    }

    /// Load both built-in and personal dictionaries using an optional custom path.
    ///
    /// If `dictionary_path` is provided, that file is used as the built-in dictionary.
    pub fn load_from_path(
        &mut self,
        dictionary_path: Option<&Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Load built-in dictionary
        self.load_builtin_dictionary(dictionary_path)?;

        // Load personal dictionary if it exists
        if self.personal_dict_path.exists() {
            self.load_personal_dictionary()?;
        } else {
            // Create empty personal dictionary file
            self.create_personal_dictionary()?;
        }

        println!("Dictionary loaded: {} words", self.symspell.word_count());
        Ok(())
    }

    /// Load the built-in dictionary from file or use fallback.
    ///
    /// Attempts to load the compile-time embedded dictionary. If it is unavailable
    /// or contains no valid words, falls back to a hardcoded list of common English
    /// words.
    ///
    /// # Errors
    /// Returns an error if fallback dictionary loading fails.
    fn load_builtin_dictionary(
        &mut self,
        dictionary_path: Option<&Path>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(dict_path) = dictionary_path {
            return self.load_dictionary_file(dict_path);
        }

        if EMBEDDED_DICTIONARY.trim().is_empty() {
            println!("Embedded dictionary unavailable; using fallback dictionary");
            return self.load_fallback_dictionary();
        }

        let mut loaded_words = 0usize;
        for line in EMBEDDED_DICTIONARY.lines() {
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Format: word frequency
            // or just: word (default frequency = 1)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let word = parts[0].to_lowercase();
            let frequency = if parts.len() > 1 {
                parts[1].parse::<u64>().unwrap_or(1)
            } else {
                1
            };

            self.symspell.insert(word, frequency);
            loaded_words += 1;
        }

        if loaded_words == 0 {
            println!("Embedded dictionary empty or invalid; using fallback dictionary");
            return self.load_fallback_dictionary();
        }

        println!("Loaded embedded dictionary with {} words", loaded_words);
        Ok(())
    }

    /// Load a dictionary from a file path.
    fn load_dictionary_file(&mut self, dict_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(dict_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();

            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let word = parts[0].to_lowercase();
            let frequency = if parts.len() > 1 {
                parts[1].parse::<u64>().unwrap_or(1)
            } else {
                1
            };

            self.symspell.insert(word, frequency);
        }

        println!("Loaded custom dictionary from {}", dict_path.display());
        Ok(())
    }

    /// Load a built-in fallback dictionary of common English words.
    ///
    /// Used when no external dictionary file is available. Contains a curated
    /// list of the most common English words with realistic frequency data.
    fn load_fallback_dictionary(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Common English words with frequencies
        let common_words = [
            ("the", 1000000),
            ("be", 500000),
            ("to", 450000),
            ("of", 400000),
            ("and", 380000),
            ("a", 350000),
            ("in", 320000),
            ("that", 300000),
            ("have", 280000),
            ("i", 270000),
            ("it", 260000),
            ("for", 250000),
            ("not", 240000),
            ("on", 230000),
            ("with", 220000),
            ("he", 210000),
            ("as", 200000),
            ("you", 195000),
            ("do", 190000),
            ("at", 185000),
            ("this", 180000),
            ("but", 175000),
            ("his", 170000),
            ("by", 165000),
            ("from", 160000),
            ("they", 155000),
            ("we", 150000),
            ("say", 145000),
            ("her", 140000),
            ("she", 135000),
            ("or", 130000),
            ("an", 125000),
            ("will", 120000),
            ("my", 115000),
            ("one", 110000),
            ("all", 105000),
            ("would", 100000),
            ("there", 98000),
            ("their", 96000),
            ("what", 94000),
            ("so", 92000),
            ("up", 90000),
            ("out", 88000),
            ("if", 86000),
            ("about", 84000),
            ("who", 82000),
            ("get", 80000),
            ("which", 78000),
            ("go", 76000),
            ("me", 74000),
            ("when", 72000),
            ("make", 70000),
            ("can", 68000),
            ("like", 66000),
            ("time", 64000),
            ("no", 62000),
            ("just", 60000),
            ("him", 58000),
            ("know", 56000),
            ("take", 54000),
            ("people", 52000),
            ("into", 50000),
            ("year", 48000),
            ("your", 46000),
            ("good", 44000),
            ("some", 42000),
            ("could", 40000),
            ("them", 38000),
            ("see", 36000),
            ("other", 34000),
            ("than", 32000),
            ("then", 30000),
            ("now", 28000),
            ("look", 26000),
            ("only", 24000),
            ("come", 22000),
            ("its", 20000),
            ("over", 19000),
            ("think", 18000),
            ("also", 17000),
            ("back", 16000),
            ("after", 15000),
            ("use", 14000),
            ("two", 13000),
            ("how", 12000),
            ("our", 11000),
            ("work", 10000),
            ("first", 9500),
            ("well", 9000),
            ("way", 8500),
            ("even", 8000),
            ("new", 7500),
            ("want", 7000),
            ("because", 6500),
            ("any", 6000),
            ("these", 5500),
            ("give", 5000),
            ("day", 4800),
            ("most", 4600),
            ("us", 4400),
            ("is", 500000),
            ("was", 450000),
            ("are", 400000),
            ("were", 350000),
            ("been", 300000),
            ("being", 250000),
            ("am", 200000),
            ("hello", 15000),
            ("world", 14000),
            ("computer", 12000),
            ("program", 11000),
            ("software", 10000),
            ("hardware", 9000),
            ("internet", 8500),
            ("email", 8000),
            ("please", 7500),
            ("thank", 7000),
            ("thanks", 6500),
            ("yes", 6000),
            ("okay", 5500),
        ];

        for (word, freq) in common_words.iter() {
            self.symspell.insert(word.to_string(), *freq);
        }

        println!(
            "Loaded fallback dictionary with {} common words",
            common_words.len()
        );
        Ok(())
    }

    /// Load the user's personal dictionary.
    ///
    /// Reads words from the personal dictionary file and adds them to
    /// the SymSpell with high frequency (1,000,000) to ensure they are
    /// preferred over similar dictionary words.
    ///
    /// # Errors
    /// Returns an error if the personal dictionary file cannot be read.
    fn load_personal_dictionary(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let file = File::open(&self.personal_dict_path)?;
        let reader = BufReader::new(file);

        let mut count = 0;
        for line in reader.lines() {
            let line = line?;
            let word = line.trim().to_lowercase();

            if !word.is_empty() && !word.starts_with('#') {
                // Personal words get high frequency to prioritize them
                self.symspell.insert(word, 1000000);
                count += 1;
            }
        }

        println!("Loaded {} personal words", count);
        Ok(())
    }

    /// Create an empty personal dictionary file with a template.
    ///
    /// Creates the file at the personal dictionary path with instructions
    /// for the user on how to add words.
    ///
    /// # Errors
    /// Returns an error if the file cannot be created.
    fn create_personal_dictionary(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(&self.personal_dict_path)?;
        writeln!(file, "# Personal Dictionary")?;
        writeln!(file, "# Add one word per line")?;
        writeln!(file, "# Lines starting with # are ignored")?;
        writeln!(file)?;
        Ok(())
    }

    /// Get the path for the personal dictionary file.
    ///
    /// Returns `%APPDATA%/Autocorrect/personal_dictionary.txt` on Windows,
    /// or falls back to `personal_dictionary.txt` in the current directory
    /// if the APPDATA environment variable is not set.
    fn get_personal_dict_path() -> PathBuf {
        // Try to use user's AppData folder
        if let Ok(appdata) = std::env::var("APPDATA") {
            let mut path = PathBuf::from(appdata);
            path.push("Autocorrect");

            // Create directory if it doesn't exist
            if !path.exists() {
                let _ = std::fs::create_dir_all(&path);
            }

            path.push("personal_dictionary.txt");
            path
        } else {
            // Fallback to current directory
            PathBuf::from("personal_dictionary.txt")
        }
    }

    /// Add a word to the personal dictionary.
    ///
    /// Adds the word to both the SymSpell (with high frequency) and
    /// appends it to the personal dictionary file for persistence.
    ///
    /// # Arguments
    /// * `word` - The word to add
    ///
    /// # Errors
    /// Returns an error if the personal dictionary file cannot be written.
    pub fn add_personal_word(&mut self, word: &str) -> Result<(), Box<dyn std::error::Error>> {
        let word = word.trim().to_lowercase();

        // Add to SymSpell
        self.symspell.insert(word.clone(), 1000000);

        // Append to file
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.personal_dict_path)?;

        writeln!(file, "{}", word)?;

        Ok(())
    }

    /// Look up spelling corrections for a word.
    ///
    /// Returns a list of suggestions sorted by edit distance (ascending)
    /// then frequency (descending).
    ///
    /// # Arguments
    /// * `word` - The potentially misspelled word
    ///
    /// # Returns
    /// A vector of `SuggestItem` containing suggestions.
    pub fn lookup(&self, word: &str) -> Vec<crate::symspell::SuggestItem> {
        self.symspell.lookup(word, 2, None)
    }

    /// Get the best correction for a word, if one exists.
    ///
    /// Returns `Some(correction)` only if:
    /// - There are suggestions
    /// - The top suggestion is different from the input
    /// - The edit distance is <= 2
    ///
    /// # Arguments
    /// * `word` - The word to check
    ///
    /// # Returns
    /// `Some(corrected_word)` if a correction is available, `None` otherwise.
    pub fn get_correction(&self, word: &str) -> Option<String> {
        let suggestions = self.lookup(word);

        // Return correction only if:
        // 1. There are suggestions
        // 2. The top suggestion is different from input
        // 3. The distance is <= 2
        if let Some(suggestion) = suggestions.first() {
            if suggestion.term.to_lowercase() != word.to_lowercase() && suggestion.distance <= 2 {
                return Some(suggestion.term.clone());
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_dictionary() {
        let mut dict = Dictionary::new();
        dict.load_fallback_dictionary().unwrap();
        assert!(dict.symspell.word_count() > 0);
    }

    #[test]
    fn test_correction() {
        let mut dict = Dictionary::new();
        dict.load_fallback_dictionary().unwrap();

        let correction = dict.get_correction("teh");
        assert_eq!(correction, Some("the".to_string()));
    }
}
