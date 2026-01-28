use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;

/// 8-dot Braille space: 2^8 = 256 dimensions
/// Each dimension corresponds to one possible 8-bit pattern
/// This allows encoding of ANY modality: text, sound, math, music, images
const EMBED_DIM: usize = 256;

pub struct BrailleEmbedder {
    contractions: HashMap<u64, Vec<f32>>,
    word_cache: HashMap<String, Vec<f32>>,
}

impl BrailleEmbedder {
    pub fn new() -> Self {
        Self {
            contractions: HashMap::new(),
            word_cache: HashMap::new(),
        }
    }

    pub fn embed(&mut self, text: &str) -> Vec<f32> {
        let words: Vec<&str> = text.split_whitespace().collect();
        if words.is_empty() {
            return vec![0.0; EMBED_DIM];
        }

        let mut result = vec![0.0; EMBED_DIM];
        let mut count = 0.0;

        for word in words {
            let word_lower = word.to_lowercase();
            let word_embed = self.embed_word(&word_lower);
            for (i, v) in word_embed.iter().enumerate() {
                result[i] += v;
            }
            count += 1.0;
        }

        if count > 0.0 {
            for v in result.iter_mut() {
                *v /= count;
            }
        }

        self.normalize(&mut result);
        result
    }

    fn embed_word(&mut self, word: &str) -> Vec<f32> {
        if let Some(cached) = self.word_cache.get(word) {
            return cached.clone();
        }

        let hash = self.hash_string(word);
        
        if let Some(contraction) = self.contractions.get(&hash) {
            return contraction.clone();
        }

        let embed = self.geometric_embed(word);
        
        self.word_cache.insert(word.to_string(), embed.clone());
        
        embed
    }

    /// True 8-dot Braille encoding: each byte directly activates its dimension
    /// This creates a 256-dimensional space where:
    /// - Text: each ASCII/UTF-8 byte activates dimension [0-255]
    /// - Sound: audio samples (8-bit) map directly to dimensions
    /// - Math: Unicode math symbols map to their byte patterns
    /// - Music: MIDI notes (0-127) + velocity map to dimensions
    /// - Images: pixel values (0-255) activate corresponding dimensions
    fn geometric_embed(&self, word: &str) -> Vec<f32> {
        let mut embed = vec![0.0; EMBED_DIM];
        let bytes = word.as_bytes();
        
        for (pos, &byte) in bytes.iter().enumerate() {
            // Direct 8-dot mapping: byte value IS the dimension
            // This is the key insight - each possible 8-bit pattern
            // has its own dimension in Braille space
            embed[byte as usize] += 1.0 / (pos as f32 + 1.0);
            
            // Position-weighted bit decomposition for finer structure
            for bit in 0..8 {
                let bit_val = ((byte >> bit) & 1) as f32;
                // Map each bit to a dimension based on position
                let idx = (pos * 8 + bit) % EMBED_DIM;
                embed[idx] += bit_val * 0.1 / (pos as f32 + 1.0);
            }
            
            // Bigram features for sequential patterns
            if pos > 0 {
                let bigram = ((bytes[pos - 1] as u16) << 8) | (byte as u16);
                let bigram_idx = (bigram as usize) % EMBED_DIM;
                embed[bigram_idx] += 0.5 / (pos as f32 + 1.0);
            }
        }

        embed
    }

    fn normalize(&self, vec: &mut Vec<f32>) {
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in vec.iter_mut() {
                *v /= norm;
            }
        }
    }

    fn hash_string(&self, s: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }

    pub fn learn_contraction(&mut self, pattern: &str, embedding: Vec<f32>) {
        let hash = self.hash_string(pattern);
        self.contractions.insert(hash, embedding);
    }

    pub fn embed_batch(&mut self, texts: &[String]) -> Vec<Vec<f32>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    pub fn stats(&self) -> (usize, usize) {
        (self.contractions.len(), self.word_cache.len())
    }
}

#[allow(dead_code)]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embed_speed() {
        let mut embedder = BrailleEmbedder::new();
        let start = std::time::Instant::now();
        
        for i in 0..10000 {
            embedder.embed(&format!("This is test message number {}", i));
        }
        
        let elapsed = start.elapsed();
        println!("10K embeddings in {:?}", elapsed);
        assert!(elapsed.as_millis() < 1000);
    }

    #[test]
    fn test_similarity() {
        let mut embedder = BrailleEmbedder::new();
        
        let a = embedder.embed("hello world");
        let b = embedder.embed("hello there");
        let c = embedder.embed("completely different text");
        
        let sim_ab = cosine_similarity(&a, &b);
        let sim_ac = cosine_similarity(&a, &c);
        
        assert!(sim_ab > sim_ac);
    }
}
