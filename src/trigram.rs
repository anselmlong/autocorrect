//! Trigram Language Model for context-based word scoring.
//!
//! This module provides a simple n-gram language model that estimates
//! the probability of a word given the previous two words in context.
//!
//! # N-gram Backoff
//!
//! The model uses backoff smoothing:
//! - First tries P(word | w-2, w-1) [trigram]
//! - Falls back to P(word | w-1) [bigram]
//! - Falls back to P(word) [unigram]
//! - Uses a small epsilon (1e-9) for unseen words
//!
//! # Usage
//!
//! The trigram model is used by `SymSpell` to rerank suggestions based
//! on context. For example, given "the quick ___", "brown" would score
//! higher than "hello" because "the quick brown" is a common trigram.
//!
//! # Training
//!
//! The model is trained on a corpus of sentences using the `train()` method:
//! ```rust
//! let mut model = TrigramModel::new();
//! model.train(&["the quick brown fox", "the lazy dog"]);
//! ```

use ahash::AHashMap;

/// A trigram language model with backoff smoothing.
///
/// Stores counts for unigrams (single words), bigrams (word pairs),
/// and trigrams (word triples) to compute conditional probabilities.
pub struct TrigramModel {
    /// Counts of word triples: (w-2, w-1, w) → count.
    trigram_counts: AHashMap<(String, String, String), u64>,
    /// Counts of word pairs: (w-1, w) → count.
    bigram_counts: AHashMap<(String, String), u64>,
    /// Counts of single words: w → count.
    unigram_counts: AHashMap<String, u64>,
    /// Total number of word tokens in the training corpus.
    total_words: u64,
}

impl TrigramModel {
    /// Create a new empty trigram model.
    ///
    /// All counts are initialized to zero. Use `train()` to populate
    /// the model with data from a text corpus.
    pub fn new() -> Self {
        Self {
            trigram_counts: AHashMap::new(),
            bigram_counts: AHashMap::new(),
            unigram_counts: AHashMap::new(),
            total_words: 0,
        }
    }

    /// Train the model on a corpus of sentences.
    ///
    /// Extracts unigrams, bigrams, and trigrams from each sentence
    /// and updates the count tables. Words are lowercased before counting.
    ///
    /// # Arguments
    /// * `corpus` - A slice of sentences to train on
    ///
    /// # Example
    /// ```rust
    /// let mut model = TrigramModel::new();
    /// model.train(&["the quick brown fox", "the lazy dog"]);
    /// ```
    pub fn train(&mut self, corpus: &[&str]) {
        for sentence in corpus {
            let words: Vec<String> = sentence
                .split_whitespace()
                .map(|s| s.to_lowercase())
                .collect();

            for i in 0..words.len() {
                *self.unigram_counts.entry(words[i].clone()).or_insert(0) += 1;
                self.total_words += 1;

                if i > 0 {
                    *self.bigram_counts
                        .entry((words[i - 1].clone(), words[i].clone()))
                        .or_insert(0) += 1;
                }

                if i > 1 {
                    *self.trigram_counts
                        .entry((
                            words[i - 2].clone(),
                            words[i - 1].clone(),
                            words[i].clone(),
                        ))
                        .or_insert(0) += 1;
                }
            }
        }
    }

    /// Calculate the conditional probability P(word | prev, prev_prev).
    ///
    /// Uses backoff smoothing: tries trigram, then bigram, then unigram.
    /// Returns a small epsilon (1e-9) for unseen word combinations.
    ///
    /// # Arguments
    /// * `word` - The word to calculate probability for
    /// * `prev` - The previous word (w-1)
    /// * `prev_prev` - The word before the previous word (w-2)
    ///
    /// # Returns
    /// The conditional probability as a float between 0 and 1.
    ///
    /// # Example
    /// ```rust
    /// let p = model.trigram_probability("fox", "quick", "the");
    /// // Returns P("fox" | "quick", "the")
    /// ```
    pub fn trigram_probability(
        &self,
        word: &str,
        prev: &str,
        prev_prev: &str,
    ) -> f64 {
        if let Some(&trigram_count) = self.trigram_counts.get(&(prev_prev.to_string(), prev.to_string(), word.to_string())) {
            if let Some(&bigram_count) = self.bigram_counts.get(&(prev_prev.to_string(), prev.to_string())) {
                return trigram_count as f64 / bigram_count as f64;
            }
        }

        if let Some(&bigram_count) = self.bigram_counts.get(&(prev.to_string(), word.to_string())) {
            if let Some(&unigram_count) = self.unigram_counts.get(prev) {
                return bigram_count as f64 / unigram_count as f64;
            }
        }

        if let Some(&unigram_count) = self.unigram_counts.get(word) {
            return unigram_count as f64 / self.total_words as f64;
        }

        1e-9 // Smoothing for unseen words
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trigram_model() {
        let mut model = TrigramModel::new();
        let corpus = [
            "the quick brown fox",
            "the quick dog",
            "the lazy dog",
            "the fox jumps",
        ];
        model.train(&corpus);

        let p = model.trigram_probability("fox", "quick", "the");
        assert!(p > 0.0);
    }
}