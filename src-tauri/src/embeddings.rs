use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EmbeddingError {
    #[error("Failed to initialize embedding model: {0}")]
    InitError(String),
    #[error("Failed to generate embeddings: {0}")]
    EmbedError(String),
}

pub struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    pub fn new() -> Result<Self, EmbeddingError> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true)
        ).map_err(|e| EmbeddingError::InitError(e.to_string()))?;
        
        Ok(Self { model })
    }
    
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let embeddings = self.model
            .embed(vec![text], None)
            .map_err(|e| EmbeddingError::EmbedError(e.to_string()))?;
        
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::EmbedError("No embedding generated".to_string()))
    }
    
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        let text_refs: Vec<&str> = texts.iter().map(|s| s.as_str()).collect();
        
        let chunk_size = 100;
        let mut all_embeddings = Vec::with_capacity(texts.len());
        
        for chunk in text_refs.chunks(chunk_size) {
            let embeddings = self.model
                .embed(chunk.to_vec(), None)
                .map_err(|e| EmbeddingError::EmbedError(e.to_string()))?;
            all_embeddings.extend(embeddings);
        }
        
        Ok(all_embeddings)
    }
}
