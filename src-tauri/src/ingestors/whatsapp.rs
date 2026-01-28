use regex::Regex;
use rusqlite::Connection;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

pub fn import() -> Result<Vec<String>, String> {
    // Try to read from WhatsApp Desktop's local SQLite database first
    if let Ok(messages) = import_from_local_db() {
        if !messages.is_empty() {
            return Ok(messages);
        }
    }
    
    // Fallback to searching for export files
    let downloads_dir = dirs::download_dir()
        .ok_or_else(|| "Could not find Downloads directory".to_string())?;
    
    let documents_dir = dirs::document_dir()
        .ok_or_else(|| "Could not find Documents directory".to_string())?;
    
    let mut all_messages = Vec::new();
    
    for search_dir in [downloads_dir, documents_dir] {
        for entry in WalkDir::new(&search_dir)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                let filename = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                if filename.starts_with("WhatsApp Chat") && filename.ends_with(".txt") {
                    match parse_whatsapp_file(path) {
                        Ok(messages) => all_messages.extend(messages),
                        Err(e) => eprintln!("Failed to parse {:?}: {}", path, e),
                    }
                }
            }
        }
    }
    
    if all_messages.is_empty() {
        return Err(
            "No WhatsApp data found. Make sure WhatsApp Desktop is installed, or use 'Import File' to select an export.".to_string()
        );
    }
    
    Ok(all_messages)
}

pub fn import_from_local_db() -> Result<Vec<String>, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Could not find home directory".to_string())?;
    
    let db_path = home
        .join("Library")
        .join("Group Containers")
        .join("group.net.whatsapp.WhatsApp.shared")
        .join("ChatStorage.sqlite");
    
    if !db_path.exists() {
        return Err("WhatsApp Desktop database not found. Is WhatsApp Desktop installed?".to_string());
    }
    
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open WhatsApp database: {}", e))?;
    
    let mut stmt = conn.prepare(
        r#"
        SELECT 
            COALESCE(m.ZTEXT, '') as text,
            datetime(m.ZMESSAGEDATE + 978307200, 'unixepoch', 'localtime') as timestamp,
            CASE 
                WHEN m.ZISFROMME = 1 THEN 'Me' 
                ELSE COALESCE(c.ZPARTNERNAME, 'Someone')
            END as display_sender
        FROM ZWAMESSAGE m
        LEFT JOIN ZWACHATSESSION c ON m.ZCHATSESSION = c.Z_PK
        WHERE m.ZTEXT IS NOT NULL AND m.ZTEXT != ''
        ORDER BY m.ZMESSAGEDATE DESC
        LIMIT 100000
        "#
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let messages: Vec<String> = stmt.query_map([], |row| {
        let text: String = row.get(0)?;
        let timestamp: String = row.get(1)?;
        let display_sender: String = row.get(2)?;
        Ok(format!("[{}] {}: {}", timestamp, display_sender, text))
    })
    .map_err(|e| format!("Failed to query messages: {}", e))?
    .filter_map(|r| r.ok())
    .filter(|msg| !msg.is_empty())
    .collect();
    
    if messages.is_empty() {
        return Err("No messages found in WhatsApp database.".to_string());
    }
    
    println!("Found {} WhatsApp messages from local database", messages.len());
    
    Ok(messages)
}

pub fn get_contact_names() -> Result<Vec<String>, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Could not find home directory".to_string())?;
    
    let db_path = home
        .join("Library")
        .join("Group Containers")
        .join("group.net.whatsapp.WhatsApp.shared")
        .join("ChatStorage.sqlite");
    
    if !db_path.exists() {
        return Ok(vec![]);
    }
    
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open WhatsApp database: {}", e))?;
    
    let mut stmt = conn.prepare(
        r#"
        SELECT DISTINCT ZPUSHNAME 
        FROM ZWAMESSAGE 
        WHERE ZPUSHNAME IS NOT NULL 
          AND ZPUSHNAME != '' 
          AND ZISFROMME = 0
          AND LENGTH(ZPUSHNAME) > 2
          AND ZPUSHNAME NOT LIKE '%@%'
        ORDER BY COUNT(*) DESC
        LIMIT 20
        "#
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let names: Vec<String> = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        Ok(name)
    })
    .map_err(|e| format!("Failed to query contacts: {}", e))?
    .filter_map(|r| r.ok())
    .filter(|name| {
        let n = name.trim();
        n.len() >= 2 
            && !n.contains('@') 
            && !n.chars().all(|c| c.is_uppercase() || c.is_numeric())
            && n.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false)
    })
    .collect();
    
    Ok(names)
}

pub fn get_recent_topics() -> Result<Vec<String>, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Could not find home directory".to_string())?;
    
    let db_path = home
        .join("Library")
        .join("Group Containers")
        .join("group.net.whatsapp.WhatsApp.shared")
        .join("ChatStorage.sqlite");
    
    if !db_path.exists() {
        return Ok(vec![]);
    }
    
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open WhatsApp database: {}", e))?;
    
    let mut stmt = conn.prepare(
        r#"
        SELECT ZTEXT 
        FROM ZWAMESSAGE 
        WHERE ZTEXT IS NOT NULL AND ZTEXT != ''
        ORDER BY ZMESSAGEDATE DESC
        LIMIT 200
        "#
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let messages: Vec<String> = stmt.query_map([], |row| row.get(0))
        .map_err(|e| format!("Failed to query: {}", e))?
        .filter_map(|r| r.ok())
        .collect();
    
    let topic_keywords = [
        ("dinner", "dinner plans"),
        ("lunch", "lunch"),
        ("meeting", "the meeting"),
        ("call", "calling"),
        ("trip", "the trip"),
        ("vacation", "vacation"),
        ("birthday", "birthday"),
        ("wedding", "the wedding"),
        ("party", "the party"),
        ("job", "the job"),
        ("interview", "the interview"),
        ("project", "the project"),
        ("doctor", "the doctor"),
        ("appointment", "the appointment"),
        ("flight", "the flight"),
        ("movie", "movies"),
        ("game", "the game"),
        ("concert", "the concert"),
        ("gym", "the gym"),
        ("school", "school"),
    ];
    
    let mut found_topics = Vec::new();
    for msg in &messages {
        let lower = msg.to_lowercase();
        for (keyword, display) in &topic_keywords {
            if lower.contains(keyword) && !found_topics.contains(&display.to_string()) {
                found_topics.push(display.to_string());
                if found_topics.len() >= 5 {
                    return Ok(found_topics);
                }
            }
        }
    }
    
    Ok(found_topics)
}

pub fn import_from_file(file_path: &str) -> Result<Vec<String>, String> {
    let path = Path::new(file_path);
    
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    
    if !path.is_file() {
        return Err(format!("Not a file: {}", file_path));
    }
    
    parse_whatsapp_file(path)
}

fn parse_whatsapp_file(path: &std::path::Path) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let message_regex = Regex::new(
        r"^\[?(\d{1,2}/\d{1,2}/\d{2,4},?\s*\d{1,2}:\d{2}(?::\d{2})?\s*(?:AM|PM)?)\]?\s*-?\s*([^:]+):\s*(.+)$"
    ).map_err(|e| format!("Regex error: {}", e))?;
    
    let messages: Vec<String> = content
        .lines()
        .filter_map(|line| {
            message_regex.captures(line).map(|caps| {
                let timestamp = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                let sender = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                let text = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                format!("[{}] {}: {}", timestamp, sender.trim(), text)
            })
        })
        .filter(|msg| !msg.contains("<Media omitted>"))
        .collect();
    
    Ok(messages)
}
