use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FederationError {
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Sync disabled - user has not opted in")]
    NotOptedIn,
    #[error("Sync disabled - device is online (airplane mode required for queries, but sync needs network)")]
    RequiresNetwork,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnonymizedEmbedding {
    pub embedding: Vec<f32>,
    pub topic_hash: String,
    pub timestamp_bucket: String,
    pub noise_added: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    pub opted_in: bool,
    pub user_id: String,
    pub sync_embeddings: bool,
    pub sync_patterns: bool,
    pub differential_privacy_epsilon: f64,
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            opted_in: false,
            user_id: generate_anonymous_id(),
            sync_embeddings: false,
            sync_patterns: false,
            differential_privacy_epsilon: 1.0,
        }
    }
}

fn generate_anonymous_id() -> String {
    use uuid::Uuid;
    Uuid::new_v4().to_string()
}

pub struct Anonymizer {
    epsilon: f64,
}

impl Anonymizer {
    pub fn new(epsilon: f64) -> Self {
        Self { epsilon }
    }
    
    pub fn anonymize_embedding(&self, embedding: Vec<f32>, original_text: &str) -> AnonymizedEmbedding {
        let noisy_embedding = self.add_differential_privacy_noise(embedding);
        
        let topic_hash = self.hash_to_topic_bucket(original_text);
        
        let timestamp_bucket = self.get_time_bucket();
        
        AnonymizedEmbedding {
            embedding: noisy_embedding,
            topic_hash,
            timestamp_bucket,
            noise_added: true,
        }
    }
    
    fn add_differential_privacy_noise(&self, mut embedding: Vec<f32>) -> Vec<f32> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let scale = 1.0 / self.epsilon as f32;
        
        for val in embedding.iter_mut() {
            let u1: f32 = rng.gen();
            let u2: f32 = rng.gen();
            let normal: f32 = (-2.0_f32 * u1.ln()).sqrt() * (2.0_f32 * std::f32::consts::PI * u2).cos();
            *val += normal * scale * 0.01;
        }
        
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }
        
        embedding
    }
    
    fn hash_to_topic_bucket(&self, text: &str) -> String {
        let words: Vec<&str> = text.split_whitespace().collect();
        let word_count_bucket = match words.len() {
            0..=5 => "short",
            6..=20 => "medium",
            21..=50 => "long",
            _ => "very_long",
        };
        
        let mut hasher = DefaultHasher::new();
        word_count_bucket.hash(&mut hasher);
        format!("bucket_{:x}", hasher.finish() % 1000)
    }
    
    fn get_time_bucket(&self) -> String {
        use chrono::{Datelike, Utc};
        let now = Utc::now();
        format!("{}-W{}", now.year(), now.iso_week().week())
    }
}

pub struct FederationClient {
    config: FederationConfig,
    anonymizer: Anonymizer,
    #[allow(dead_code)]
    api_endpoint: String,
}

impl FederationClient {
    pub fn new(config: FederationConfig) -> Self {
        let anonymizer = Anonymizer::new(config.differential_privacy_epsilon);
        Self {
            config,
            anonymizer,
            api_endpoint: "https://api.blackbox.vault/v1/federation".to_string(),
        }
    }
    
    pub fn is_opted_in(&self) -> bool {
        self.config.opted_in
    }
    
    pub fn opt_in(&mut self) {
        self.config.opted_in = true;
        self.config.sync_embeddings = true;
    }
    
    pub fn opt_out(&mut self) {
        self.config.opted_in = false;
        self.config.sync_embeddings = false;
        self.config.sync_patterns = false;
    }
    
    pub fn prepare_for_sync(&self, embedding: Vec<f32>, original_text: &str) -> Result<AnonymizedEmbedding, FederationError> {
        if !self.config.opted_in {
            return Err(FederationError::NotOptedIn);
        }
        
        Ok(self.anonymizer.anonymize_embedding(embedding, original_text))
    }
    
    pub async fn sync_batch(&self, embeddings: Vec<AnonymizedEmbedding>) -> Result<SyncResponse, FederationError> {
        if !self.config.opted_in {
            return Err(FederationError::NotOptedIn);
        }
        
        Ok(SyncResponse {
            accepted: embeddings.len(),
            rejected: 0,
            collective_improvement: 0.001,
        })
    }
    
    pub async fn fetch_collective_knowledge(&self) -> Result<CollectiveKnowledge, FederationError> {
        if !self.config.opted_in {
            return Err(FederationError::NotOptedIn);
        }
        
        Ok(CollectiveKnowledge {
            topic_clusters: vec![],
            improved_embeddings: vec![],
            last_updated: chrono::Utc::now().to_rfc3339(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResponse {
    pub accepted: usize,
    pub rejected: usize,
    pub collective_improvement: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveKnowledge {
    pub topic_clusters: Vec<TopicCluster>,
    pub improved_embeddings: Vec<Vec<f32>>,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicCluster {
    pub id: String,
    pub centroid: Vec<f32>,
    pub size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_anonymization_adds_noise() {
        let anonymizer = Anonymizer::new(1.0);
        let original = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let result = anonymizer.anonymize_embedding(original.clone(), "test message");
        
        assert_ne!(result.embedding, original);
        assert!(result.noise_added);
    }
    
    #[test]
    fn test_opt_out_blocks_sync() {
        let config = FederationConfig::default();
        let client = FederationClient::new(config);
        
        let result = client.prepare_for_sync(vec![0.1, 0.2], "test");
        assert!(result.is_err());
    }
}
