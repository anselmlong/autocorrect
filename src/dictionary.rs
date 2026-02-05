// dictionary.rs - Dictionary loading and management
// Handles both built-in dictionary and personal user dictionary

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use crate::symspell::SymSpell;

pub struct Dictionary {
    symspell: SymSpell,
    personal_dict_path: PathBuf,
}

impl Dictionary {
    /// Create a new dictionary with max edit distance of 2
    pub fn new() -> Self {
        Self {
            symspell: SymSpell::new(2),
            personal_dict_path: Self::get_personal_dict_path(),
        }
    }
    
    /// Load dictionaries from files
    pub fn load(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Load built-in dictionary
        self.load_builtin_dictionary()?;
        
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
    
    /// Load the built-in dictionary
    fn load_builtin_dictionary(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Try to load from dictionary/words.txt
        let dict_path = Path::new("dictionary/words.txt");
        
        if !dict_path.exists() {
            // If file doesn't exist, use embedded fallback dictionary
            return self.load_fallback_dictionary();
        }
        
        let file = File::open(dict_path)?;
        let reader = BufReader::new(file);
        
        for line in reader.lines() {
            let line = line?;
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
        }
        
        Ok(())
    }
    
    /// Load a small fallback dictionary if no dictionary file exists
    fn load_fallback_dictionary(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Common English words with frequencies
        let common_words = [
            ("the", 1000000), ("be", 500000), ("to", 450000), ("of", 400000),
            ("and", 380000), ("a", 350000), ("in", 320000), ("that", 300000),
            ("have", 280000), ("i", 270000), ("it", 260000), ("for", 250000),
            ("not", 240000), ("on", 230000), ("with", 220000), ("he", 210000),
            ("as", 200000), ("you", 195000), ("do", 190000), ("at", 185000),
            ("this", 180000), ("but", 175000), ("his", 170000), ("by", 165000),
            ("from", 160000), ("they", 155000), ("we", 150000), ("say", 145000),
            ("her", 140000), ("she", 135000), ("or", 130000), ("an", 125000),
            ("will", 120000), ("my", 115000), ("one", 110000), ("all", 105000),
            ("would", 100000), ("there", 98000), ("their", 96000), ("what", 94000),
            ("so", 92000), ("up", 90000), ("out", 88000), ("if", 86000),
            ("about", 84000), ("who", 82000), ("get", 80000), ("which", 78000),
            ("go", 76000), ("me", 74000), ("when", 72000), ("make", 70000),
            ("can", 68000), ("like", 66000), ("time", 64000), ("no", 62000),
            ("just", 60000), ("him", 58000), ("know", 56000), ("take", 54000),
            ("people", 52000), ("into", 50000), ("year", 48000), ("your", 46000),
            ("good", 44000), ("some", 42000), ("could", 40000), ("them", 38000),
            ("see", 36000), ("other", 34000), ("than", 32000), ("then", 30000),
            ("now", 28000), ("look", 26000), ("only", 24000), ("come", 22000),
            ("its", 20000), ("over", 19000), ("think", 18000), ("also", 17000),
            ("back", 16000), ("after", 15000), ("use", 14000), ("two", 13000),
            ("how", 12000), ("our", 11000), ("work", 10000), ("first", 9500),
            ("well", 9000), ("way", 8500), ("even", 8000), ("new", 7500),
            ("want", 7000), ("because", 6500), ("any", 6000), ("these", 5500),
            ("give", 5000), ("day", 4800), ("most", 4600), ("us", 4400),
            ("is", 500000), ("was", 450000), ("are", 400000), ("were", 350000),
            ("been", 300000), ("being", 250000), ("am", 200000),
            ("hello", 15000), ("world", 14000), ("computer", 12000),
            ("program", 11000), ("software", 10000), ("hardware", 9000),
            ("internet", 8500), ("email", 8000), ("please", 7500),
            ("thank", 7000), ("thanks", 6500), ("yes", 6000), ("okay", 5500),
        ];
        
        for (word, freq) in common_words.iter() {
            self.symspell.insert(word.to_string(), *freq);
        }
        
        println!("Loaded fallback dictionary with {} common words", common_words.len());
        Ok(())
    }
    
    /// Load personal dictionary
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
    
    /// Create an empty personal dictionary file
    fn create_personal_dictionary(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create(&self.personal_dict_path)?;
        writeln!(file, "# Personal Dictionary")?;
        writeln!(file, "# Add one word per line")?;
        writeln!(file, "# Lines starting with # are ignored")?;
        writeln!(file)?;
        Ok(())
    }
    
    /// Get the path for personal dictionary
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
    
    /// Add a word to personal dictionary
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
    
    /// Look up corrections for a word
    pub fn lookup(&self, word: &str) -> Vec<crate::symspell::SuggestItem> {
        self.symspell.lookup(word, 2)
    }
    
    /// Get the best correction for a word (if any)
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
