pub mod imessage;
pub mod whatsapp;
pub mod slack;

use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: Option<String>,
    pub source: String,
}

impl std::fmt::Display for ChatMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", 
            self.timestamp.as_deref().unwrap_or("unknown"),
            self.sender,
            self.content
        )
    }
}
