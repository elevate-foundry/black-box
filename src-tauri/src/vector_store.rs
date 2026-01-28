use rusqlite::{Connection, params};
use std::path::Path;
use thiserror::Error;
use chrono::Utc;

#[derive(Error, Debug)]
pub enum VectorStoreError {
    #[error("Database error: {0}")]
    DbError(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

pub struct VectorStore {
    conn: Connection,
}

impl VectorStore {
    pub fn new(data_dir: &Path) -> Result<Self, VectorStoreError> {
        let db_path = data_dir.join("vault.db");
        let conn = Connection::open(&db_path)?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content TEXT NOT NULL,
                embedding BLOB NOT NULL,
                source TEXT NOT NULL,
                indexed_at TEXT NOT NULL
            )",
            [],
        )?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_source ON messages(source)",
            [],
        )?;
        
        Ok(Self { conn })
    }
    
    pub fn insert(&mut self, content: &str, embedding: Vec<f32>, source: &str) -> Result<(), String> {
        let embedding_bytes = embedding_to_bytes(&embedding);
        let now = Utc::now().to_rfc3339();
        
        self.conn.execute(
            "INSERT INTO messages (content, embedding, source, indexed_at) VALUES (?1, ?2, ?3, ?4)",
            params![content, embedding_bytes, source, now],
        ).map_err(|e| e.to_string())?;
        
        Ok(())
    }
    
    pub fn search(&self, query_embedding: &[f32], limit: usize) -> Result<Vec<(String, f32)>, String> {
        let mut stmt = self.conn
            .prepare("SELECT content, embedding FROM messages")
            .map_err(|e| e.to_string())?;
        
        let mut results: Vec<(String, f32)> = stmt
            .query_map([], |row| {
                let content: String = row.get(0)?;
                let embedding_bytes: Vec<u8> = row.get(1)?;
                Ok((content, embedding_bytes))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .map(|(content, bytes)| {
                let embedding = bytes_to_embedding(&bytes);
                let similarity = cosine_similarity(query_embedding, &embedding);
                (content, similarity)
            })
            .collect();
        
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        
        Ok(results)
    }
    
    pub fn count(&self) -> usize {
        self.conn
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
            .unwrap_or(0)
    }
    
    pub fn get_sources(&self) -> Vec<String> {
        let mut stmt = self.conn
            .prepare("SELECT DISTINCT source FROM messages")
            .unwrap();
        
        stmt.query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }
    
    pub fn last_indexed(&self) -> Option<String> {
        self.conn
            .query_row(
                "SELECT indexed_at FROM messages ORDER BY indexed_at DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .ok()
    }
}

fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| {
            let arr: [u8; 4] = chunk.try_into().unwrap();
            f32::from_le_bytes(arr)
        })
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
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
