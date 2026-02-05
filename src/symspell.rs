// symspell.rs - SymSpell algorithm implementation for fast spell correction
// Based on the SymSpell algorithm with edit distance up to 2

use ahash::{AHashMap, AHashSet};
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct SuggestItem {
    pub term: String,
    pub distance: i32,
    pub frequency: u64,
}

impl SuggestItem {
    fn new(term: String, distance: i32, frequency: u64) -> Self {
        Self { term, distance, frequency }
    }
}

pub struct SymSpell {
    // Main dictionary: word -> frequency
    words: AHashMap<String, u64>,
    // Delete dictionary: delete_word -> list of original words
    deletes: AHashMap<String, Vec<String>>,
    max_edit_distance: i32,
}

impl SymSpell {
    pub fn new(max_edit_distance: i32) -> Self {
        Self {
            words: AHashMap::new(),
            deletes: AHashMap::new(),
            max_edit_distance,
        }
    }
    
    /// Add a word to the dictionary with its frequency
    pub fn insert(&mut self, word: String, frequency: u64) {
        // Store the word
        self.words.insert(word.clone(), frequency);
        
        // Generate deletes for this word
        let deletes = Self::generate_deletes(&word, self.max_edit_distance);
        for delete in deletes {
            self.deletes.entry(delete)
                .or_insert_with(Vec::new)
                .push(word.clone());
        }
    }
    
    /// Lookup suggestions for a word
    pub fn lookup(&self, input: &str, max_edit_distance: i32) -> Vec<SuggestItem> {
        let mut suggestions = Vec::new();
        let mut considered = AHashSet::new();
        
        // Check if input is in dictionary
        if let Some(&frequency) = self.words.get(input) {
            suggestions.push(SuggestItem::new(input.to_string(), 0, frequency));
            if max_edit_distance == 0 {
                return suggestions;
            }
        }
        
        considered.insert(input.to_string());
        
        // Generate deletes for input
        let input_deletes = Self::generate_deletes(input, max_edit_distance);
        
        for delete in input_deletes {
            if let Some(originals) = self.deletes.get(&delete) {
                for original in originals {
                    if considered.contains(original) {
                        continue;
                    }
                    considered.insert(original.clone());
                    
                    let distance = Self::damerau_levenshtein_distance(input, original, max_edit_distance);
                    
                    if distance >= 0 && distance <= max_edit_distance {
                        if let Some(&frequency) = self.words.get(original) {
                            suggestions.push(SuggestItem::new(
                                original.clone(),
                                distance,
                                frequency
                            ));
                        }
                    }
                }
            }
        }
        
        // Sort by distance first, then by frequency
        suggestions.sort_by(|a, b| {
            match a.distance.cmp(&b.distance) {
                Ordering::Equal => b.frequency.cmp(&a.frequency),
                other => other,
            }
        });
        
        suggestions
    }
    
    /// Generate all delete strings within max_edit_distance
    fn generate_deletes(word: &str, max_edit_distance: i32) -> Vec<String> {
        let mut deletes = Vec::new();
        let mut queue = vec![(word.to_string(), 0)];
        let mut seen = AHashSet::new();
        seen.insert(word.to_string());
        
        while let Some((current, depth)) = queue.pop() {
            if depth < max_edit_distance {
                let chars: Vec<char> = current.chars().collect();
                for i in 0..chars.len() {
                    let mut new_word = String::with_capacity(current.len() - 1);
                    for (j, &c) in chars.iter().enumerate() {
                        if i != j {
                            new_word.push(c);
                        }
                    }
                    
                    if !seen.contains(&new_word) {
                        seen.insert(new_word.clone());
                        deletes.push(new_word.clone());
                        queue.push((new_word, depth + 1));
                    }
                }
            }
        }
        
        deletes
    }
    
    /// Calculate Damerau-Levenshtein distance with early termination
    fn damerau_levenshtein_distance(source: &str, target: &str, max_distance: i32) -> i32 {
        let source_chars: Vec<char> = source.chars().collect();
        let target_chars: Vec<char> = target.chars().collect();
        let len1 = source_chars.len();
        let len2 = target_chars.len();
        
        // Quick checks
        if len1 == 0 {
            return if len2 as i32 <= max_distance { len2 as i32 } else { -1 };
        }
        if len2 == 0 {
            return if len1 as i32 <= max_distance { len1 as i32 } else { -1 };
        }
        if (len1 as i32 - len2 as i32).abs() > max_distance {
            return -1;
        }
        
        // Initialize matrix
        let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
        for i in 0..=len1 {
            matrix[i][0] = i;
        }
        for j in 0..=len2 {
            matrix[0][j] = j;
        }
        
        // Calculate distances
        for i in 1..=len1 {
            let mut min_in_row = usize::MAX;
            
            for j in 1..=len2 {
                let cost = if source_chars[i - 1] == target_chars[j - 1] { 0 } else { 1 };
                
                let deletion = matrix[i - 1][j] + 1;
                let insertion = matrix[i][j - 1] + 1;
                let substitution = matrix[i - 1][j - 1] + cost;
                
                matrix[i][j] = deletion.min(insertion).min(substitution);
                
                // Damerau: transposition
                if i > 1 && j > 1 
                    && source_chars[i - 1] == target_chars[j - 2] 
                    && source_chars[i - 2] == target_chars[j - 1] {
                    matrix[i][j] = matrix[i][j].min(matrix[i - 2][j - 2] + cost);
                }
                
                min_in_row = min_in_row.min(matrix[i][j]);
            }
            
            // Early termination: if the minimum in this row exceeds max_distance, we can stop
            if min_in_row > max_distance as usize {
                return -1;
            }
        }
        
        let distance = matrix[len1][len2] as i32;
        if distance <= max_distance {
            distance
        } else {
            -1
        }
    }
    
    /// Get word count
    pub fn word_count(&self) -> usize {
        self.words.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_lookup() {
        let mut symspell = SymSpell::new(2);
        symspell.insert("hello".to_string(), 100);
        symspell.insert("world".to_string(), 50);
        
        let suggestions = symspell.lookup("hello", 2);
        assert_eq!(suggestions[0].term, "hello");
        assert_eq!(suggestions[0].distance, 0);
    }
    
    #[test]
    fn test_correction() {
        let mut symspell = SymSpell::new(2);
        symspell.insert("hello".to_string(), 100);
        
        let suggestions = symspell.lookup("helo", 2);
        assert!(!suggestions.is_empty());
        assert_eq!(suggestions[0].term, "hello");
        assert_eq!(suggestions[0].distance, 1);
    }
    
    #[test]
    fn test_distance() {
        let dist = SymSpell::damerau_levenshtein_distance("hello", "helo", 2);
        assert_eq!(dist, 1);
        
        let dist = SymSpell::damerau_levenshtein_distance("hello", "world", 2);
        assert_eq!(dist, -1); // Exceeds max distance
    }
}
