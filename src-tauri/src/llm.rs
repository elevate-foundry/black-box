use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("Failed to initialize model: {0}")]
    InitError(String),
    #[error("Failed to generate: {0}")]
    GenerateError(String),
    #[error("Ollama not installed or not running")]
    OllamaNotAvailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelTier {
    Fast,      // llama3.2:1b - 1.3GB, instant responses
    Balanced,  // llama3.2:latest - 2GB, good quality
    Quality,   // llama3.1:latest - 4.9GB, best quality
}

impl ModelTier {
    fn model_name(&self) -> &'static str {
        match self {
            ModelTier::Fast => "llama3.2:1b",
            ModelTier::Balanced => "llama3.2:latest",
            ModelTier::Quality => "llama3.1:latest",
        }
    }
    
    fn next_tier(&self) -> Option<ModelTier> {
        match self {
            ModelTier::Fast => Some(ModelTier::Balanced),
            ModelTier::Balanced => Some(ModelTier::Quality),
            ModelTier::Quality => None,
        }
    }
}

pub struct LocalLLM {
    current_tier: ModelTier,
    available_tiers: Vec<ModelTier>,
    downloading: Arc<AtomicBool>,
}

impl LocalLLM {
    pub fn new() -> Result<Self, LLMError> {
        let _ = Command::new("ollama")
            .args(["list"])
            .output()
            .map_err(|_| LLMError::OllamaNotAvailable)?;
        
        let available_tiers = Self::check_available_models();
        
        let current_tier = if available_tiers.is_empty() {
            println!("No models found, pulling llama3.2:1b...");
            let _ = Command::new("ollama")
                .args(["pull", "llama3.2:1b"])
                .output();
            ModelTier::Fast
        } else {
            *available_tiers.last().unwrap()
        };
        
        let mut llm = Self {
            current_tier,
            available_tiers,
            downloading: Arc::new(AtomicBool::new(false)),
        };
        
        llm.start_background_download();
        
        Ok(llm)
    }
    
    fn check_available_models() -> Vec<ModelTier> {
        let output = Command::new("ollama")
            .args(["list"])
            .output()
            .ok();
        
        let list_output = output
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        
        let mut available = Vec::new();
        
        if list_output.contains("llama3.2:1b") {
            available.push(ModelTier::Fast);
        }
        if list_output.contains("llama3.2:latest") || list_output.contains("llama3.2 ") {
            available.push(ModelTier::Balanced);
        }
        if list_output.contains("llama3.1:latest") || list_output.contains("llama3.1 ") {
            available.push(ModelTier::Quality);
        }
        
        available
    }
    
    fn start_background_download(&mut self) {
        if let Some(next_tier) = self.current_tier.next_tier() {
            if !self.available_tiers.contains(&next_tier) && !self.downloading.load(Ordering::SeqCst) {
                let downloading = Arc::clone(&self.downloading);
                let model_name = next_tier.model_name().to_string();
                
                thread::spawn(move || {
                    downloading.store(true, Ordering::SeqCst);
                    println!("Background downloading {}...", model_name);
                    
                    let _ = Command::new("ollama")
                        .args(["pull", &model_name])
                        .output();
                    
                    println!("Finished downloading {}", model_name);
                    downloading.store(false, Ordering::SeqCst);
                });
            }
        }
    }
    
    pub fn get_status(&self) -> ModelStatus {
        let available = Self::check_available_models();
        let best_available = available.last().copied().unwrap_or(ModelTier::Fast);
        
        ModelStatus {
            current_tier: self.current_tier,
            best_available,
            is_downloading: self.downloading.load(Ordering::SeqCst),
            tiers: vec![
                TierInfo { tier: ModelTier::Fast, available: available.contains(&ModelTier::Fast), size_gb: 1.3 },
                TierInfo { tier: ModelTier::Balanced, available: available.contains(&ModelTier::Balanced), size_gb: 2.0 },
                TierInfo { tier: ModelTier::Quality, available: available.contains(&ModelTier::Quality), size_gb: 4.9 },
            ],
        }
    }
    
    pub fn upgrade_if_available(&mut self) {
        let available = Self::check_available_models();
        if let Some(best) = available.last() {
            if *best as u8 > self.current_tier as u8 {
                println!("Upgrading from {:?} to {:?}", self.current_tier, best);
                self.current_tier = *best;
            }
        }
        self.available_tiers = available;
        self.start_background_download();
    }
    
    pub fn generate(&mut self, system_prompt: &str, user_prompt: &str) -> Result<String, LLMError> {
        self.upgrade_if_available();
        
        let full_prompt = format!(
            "{}\n\nUser question: {}\n\nAnswer concisely based on the context above:",
            system_prompt,
            user_prompt
        );
        
        let model = self.current_tier.model_name();
        println!("Using model: {}", model);
        
        let output = Command::new("ollama")
            .args(["run", model, &full_prompt])
            .output()
            .map_err(|e| LLMError::GenerateError(format!("Failed to run ollama: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LLMError::GenerateError(format!("Ollama error: {}", stderr)));
        }
        
        let response = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        if response.is_empty() {
            return Err(LLMError::GenerateError("Empty response from Ollama".to_string()));
        }
        
        Ok(response)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelStatus {
    pub current_tier: ModelTier,
    pub best_available: ModelTier,
    pub is_downloading: bool,
    pub tiers: Vec<TierInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierInfo {
    pub tier: ModelTier,
    pub available: bool,
    pub size_gb: f32,
}
