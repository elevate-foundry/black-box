mod ingestors;
mod embeddings;
mod vector_store;
mod llm;
mod federation;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub vector_store: Mutex<vector_store::VectorStore>,
    pub embedder: Mutex<Option<embeddings::Embedder>>,
    pub llm: Mutex<Option<llm::LocalLLM>>,
    pub federation: Mutex<federation::FederationClient>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultStats {
    pub total_messages: usize,
    pub sources: Vec<String>,
    pub last_indexed: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct QueryResponse {
    pub answer: String,
    pub sources: Vec<String>,
}

#[tauri::command]
fn check_offline_status() -> bool {
    use std::net::TcpStream;
    use std::time::Duration;
    
    let test_hosts = [
        ("8.8.8.8", 53),
        ("1.1.1.1", 53),
        ("208.67.222.222", 53),
    ];
    
    for (host, port) in test_hosts {
        if TcpStream::connect_timeout(
            &format!("{}:{}", host, port).parse().unwrap(),
            Duration::from_millis(500),
        ).is_ok() {
            return false;
        }
    }
    
    true
}

#[tauri::command]
fn get_vault_stats(state: State<AppState>) -> Result<VaultStats, String> {
    let store = state.vector_store.lock().map_err(|e| e.to_string())?;
    Ok(VaultStats {
        total_messages: store.count(),
        sources: store.get_sources(),
        last_indexed: store.last_indexed(),
    })
}

#[tauri::command]
async fn import_messages(source: String, file_path: Option<String>, state: State<'_, AppState>) -> Result<(), String> {
    let messages = match source.as_str() {
        "imessage" => ingestors::imessage::import()?,
        "whatsapp" => {
            if let Some(path) = file_path {
                ingestors::whatsapp::import_from_file(&path)?
            } else {
                ingestors::whatsapp::import()?
            }
        },
        "slack" => ingestors::slack::import()?,
        _ => return Err(format!("Unknown source: {}", source)),
    };
    
    let mut embedder_guard = state.embedder.lock().map_err(|e| e.to_string())?;
    let embedder = embedder_guard.get_or_insert_with(|| {
        embeddings::Embedder::new().expect("Failed to initialize embedder")
    });
    
    let embeddings = embedder.embed_batch(&messages).map_err(|e| e.to_string())?;
    
    let mut store = state.vector_store.lock().map_err(|e| e.to_string())?;
    for (msg, embedding) in messages.iter().zip(embeddings.iter()) {
        store.insert(msg, embedding.clone(), &source).map_err(|e| e.to_string())?;
    }
    
    Ok(())
}

#[tauri::command]
async fn query_vault(prompt: String, state: State<'_, AppState>) -> Result<QueryResponse, String> {
    if !check_offline_status() {
        return Err("For your security, please disconnect Wi-Fi to use the Vault.".to_string());
    }
    
    let mut embedder_guard = state.embedder.lock().map_err(|e| e.to_string())?;
    let embedder = embedder_guard.get_or_insert_with(|| {
        embeddings::Embedder::new().expect("Failed to initialize embedder")
    });
    
    let query_embedding = embedder.embed(&prompt).map_err(|e| e.to_string())?;
    
    let store = state.vector_store.lock().map_err(|e| e.to_string())?;
    let results = store.search(&query_embedding, 5)?;
    
    let context: String = results
        .iter()
        .map(|(text, _score)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n---\n");
    
    let sources: Vec<String> = results
        .iter()
        .take(3)
        .map(|(text, _)| {
            if text.len() > 50 {
                format!("{}...", &text[..50])
            } else {
                text.clone()
            }
        })
        .collect();
    
    let mut llm_guard = state.llm.lock().map_err(|e| e.to_string())?;
    let llm = llm_guard.get_or_insert_with(|| {
        llm::LocalLLM::new().expect("Failed to initialize LLM")
    });
    
    let system_prompt = format!(
        "You are a helpful assistant that answers questions based on the user's personal message history. \
        Use the following context from their messages to answer their question. \
        Be concise and direct. If the context doesn't contain relevant information, say so.\n\n\
        Context:\n{}", 
        context
    );
    
    let answer = llm.generate(&system_prompt, &prompt).map_err(|e| e.to_string())?;
    
    Ok(QueryResponse { answer, sources })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FederationStatus {
    pub opted_in: bool,
    pub embeddings_contributed: usize,
    pub collective_users: usize,
}

#[tauri::command]
fn get_federation_status(state: State<AppState>) -> Result<FederationStatus, String> {
    let federation = state.federation.lock().map_err(|e| e.to_string())?;
    Ok(FederationStatus {
        opted_in: federation.is_opted_in(),
        embeddings_contributed: 0,
        collective_users: 0,
    })
}

#[tauri::command]
fn opt_in_federation(state: State<AppState>) -> Result<FederationStatus, String> {
    let mut federation = state.federation.lock().map_err(|e| e.to_string())?;
    federation.opt_in();
    Ok(FederationStatus {
        opted_in: true,
        embeddings_contributed: 0,
        collective_users: 0,
    })
}

#[tauri::command]
fn opt_out_federation(state: State<AppState>) -> Result<FederationStatus, String> {
    let mut federation = state.federation.lock().map_err(|e| e.to_string())?;
    federation.opt_out();
    Ok(FederationStatus {
        opted_in: false,
        embeddings_contributed: 0,
        collective_users: 0,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WhatsAppStatus {
    pub available: bool,
    pub message_count: usize,
}

#[tauri::command]
fn check_whatsapp_available() -> Result<WhatsAppStatus, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Could not find home directory".to_string())?;
    
    let db_path = home
        .join("Library")
        .join("Group Containers")
        .join("group.net.whatsapp.WhatsApp.shared")
        .join("ChatStorage.sqlite");
    
    if !db_path.exists() {
        return Ok(WhatsAppStatus {
            available: false,
            message_count: 0,
        });
    }
    
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Failed to open WhatsApp database: {}", e))?;
    
    let count: usize = conn.query_row(
        "SELECT COUNT(*) FROM ZWAMESSAGE WHERE ZTEXT IS NOT NULL AND ZTEXT != ''",
        [],
        |row| row.get(0)
    ).unwrap_or(0);
    
    Ok(WhatsAppStatus {
        available: true,
        message_count: count,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("black-box");
    
    std::fs::create_dir_all(&data_dir).expect("Failed to create data directory");
    
    let vector_store = vector_store::VectorStore::new(&data_dir)
        .expect("Failed to initialize vector store");
    
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState {
            vector_store: Mutex::new(vector_store),
            embedder: Mutex::new(None),
            llm: Mutex::new(None),
            federation: Mutex::new(federation::FederationClient::new(
                federation::FederationConfig::default()
            )),
        })
        .invoke_handler(tauri::generate_handler![
            check_offline_status,
            get_vault_stats,
            import_messages,
            query_vault,
            get_federation_status,
            opt_in_federation,
            opt_out_federation,
            check_whatsapp_available,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
