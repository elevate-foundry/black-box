mod ingestors;
mod embeddings;
mod vector_store;
mod llm;
mod federation;
mod braille_embed;
mod persona;
mod semantic_lattice;
mod braille_contractions;

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{State, Emitter};

fn truncate_safe(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

fn disable_wifi() {
    // On macOS, use networksetup to disable WiFi
    // This requires the app to have appropriate permissions
    if cfg!(target_os = "macos") {
        let _ = std::process::Command::new("networksetup")
            .args(["-setairportpower", "en0", "off"])
            .output();
        // Give the network stack time to fully disconnect
        std::thread::sleep(std::time::Duration::from_millis(500));
        println!("SAL: WiFi disabled for your protection");
    }
}

pub struct AppState {
    pub vector_store: Mutex<vector_store::VectorStore>,
    pub embedder: Mutex<Option<embeddings::Embedder>>,
    pub braille_embedder: Mutex<braille_embed::BrailleEmbedder>,
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
async fn import_messages(source: String, file_path: Option<String>, state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), String> {
    let _ = app.emit("status", "Finding your messages...");
    
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
    
    let _ = app.emit("status", format!("Found {} messages! Creating memory index...", messages.len()));
    
    let start = std::time::Instant::now();
    
    let mut braille = state.braille_embedder.lock().map_err(|e| e.to_string())?;
    let embeddings = braille.embed_batch(&messages);
    
    let elapsed = start.elapsed();
    let _ = app.emit("status", format!("Indexed {} messages in {:.1}s. Saving to vault...", messages.len(), elapsed.as_secs_f32()));
    
    let mut store = state.vector_store.lock().map_err(|e| e.to_string())?;
    for (msg, embedding) in messages.iter().zip(embeddings.iter()) {
        store.insert(msg, embedding.clone(), &source).map_err(|e| e.to_string())?;
    }
    
    let _ = app.emit("status", format!("Your vault is ready with {} memories!", messages.len()));
    
    Ok(())
}

#[tauri::command]
async fn query_vault(prompt: String, state: State<'_, AppState>) -> Result<QueryResponse, String> {
    if !check_offline_status() {
        return Err("For your security, please disconnect Wi-Fi to use the Vault.".to_string());
    }
    
    let mut braille = state.braille_embedder.lock().map_err(|e| e.to_string())?;
    let query_embedding = braille.embed(&prompt);
    
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
            truncate_safe(text, 50)
        })
        .collect();
    
    let mut llm_guard = state.llm.lock().map_err(|e| e.to_string())?;
    let llm = llm_guard.get_or_insert_with(|| {
        llm::LocalLLM::new().expect("Failed to initialize LLM")
    });
    
    let sal_identity = persona::get_sal_identity();
    let personalized_context = persona::generate_system_prompt();
    
    let lattice = semantic_lattice::build_lattice_from_messages();
    let relationship_knowledge = lattice.generate_knowledge_prompt();
    
    let system_prompt = format!(
        "{}\n\n{}\n\n{}\n\n\
        IMPORTANT RULES:\n\
        1. ONLY reference messages that are explicitly shown below. Do NOT invent or hallucinate messages.\n\
        2. If you don't have relevant messages, say so honestly.\n\
        3. Quote the actual message text when referencing it.\n\
        4. The messages below are the ONLY context you have - do not make up dates, names, or content.\n\n\
        Retrieved messages from their history:\n{}", 
        sal_identity,
        personalized_context,
        relationship_knowledge,
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
fn get_suggested_queries(_state: State<AppState>) -> Result<Vec<String>, String> {
    let names = ingestors::whatsapp::get_contact_names().unwrap_or_default();
    let topics = ingestors::whatsapp::get_recent_topics().unwrap_or_default();
    
    let mut suggestions = Vec::new();
    
    if let Some(name) = names.first() {
        suggestions.push(format!("What did {} and I talk about recently?", name));
    }
    if let Some(topic) = topics.first() {
        suggestions.push(format!("Find messages about {}", topic));
    }
    if names.len() > 1 {
        suggestions.push(format!("When did {} last message me?", names[1]));
    }
    
    if suggestions.is_empty() {
        suggestions = vec![
            "What did we talk about last week?".to_string(),
            "Find messages about plans".to_string(),
            "Who messaged me recently?".to_string(),
        ];
    }
    
    Ok(suggestions)
}

#[tauri::command]
fn get_lattice_snapshot() -> Result<semantic_lattice::LatticeSnapshot, String> {
    Ok(semantic_lattice::get_lattice_snapshot())
}

#[tauri::command]
fn generate_braille_file() -> Result<String, String> {
    let corpus = braille_contractions::generate_braille_contractions()?;
    Ok(format!(
        "Generated {} Braille contractions at {} tokens/sec ({}ms). Compression: {:.2}x. Saved to ~/Desktop/sal_braille_contractions.txt",
        corpus.braille_messages.len(),
        corpus.tokens_per_sec,
        corpus.elapsed_ms,
        corpus.compression_ratio
    ))
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
    // SAL auto-disables WiFi on startup for maximum security
    disable_wifi();
    
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
            braille_embedder: Mutex::new(braille_embed::BrailleEmbedder::new()),
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
            get_suggested_queries,
            get_lattice_snapshot,
            generate_braille_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
