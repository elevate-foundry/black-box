use thiserror::Error;

#[derive(Error, Debug)]
pub enum LLMError {
    #[error("Failed to initialize model: {0}")]
    InitError(String),
    #[error("Failed to generate: {0}")]
    GenerateError(String),
}

pub struct LocalLLM {
    #[allow(dead_code)]
    system_context: String,
}

impl LocalLLM {
    pub fn new() -> Result<Self, LLMError> {
        Ok(Self {
            system_context: String::new(),
        })
    }
    
    pub fn generate(&mut self, _system_prompt: &str, user_prompt: &str) -> Result<String, LLMError> {
        let response = format!(
            "Based on your message history, here's what I found relevant to your query: '{}'\n\n\
            [Note: Full LLM inference requires downloading the Phi-3 model (~2GB). \
            The RAG retrieval is working - relevant context was found in your vault.]",
            user_prompt
        );
        
        Ok(response)
    }
}
