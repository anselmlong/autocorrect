// trigram.rs - Trigram Language Model for context-based word scoring
// Provides conditional probabilities P(word | prev_word, two_words_ago)

use ahash::AHashMap;

pub struct TrigramModel {
    trigram_counts: AHashMap<(String, String, String), u64>,
    bigram_counts: AHashMap<(String, String), u64>,
    unigram_counts: AHashMap<String, u64>,
    total_words: u64,
}

impl TrigramModel {
    pub fn new() -> Self {
        Self {
            trigram_counts: AHashMap::new(),
            bigram_counts: AHashMap::new(),
            unigram_counts: AHashMap::new(),
            total_words: 0,
        }
    }

    /// Train the trigram model on a corpus of sentences
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

    /// Returns P(word | prev, prev_prev)
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