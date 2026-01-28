use rusqlite::Connection;
use std::path::PathBuf;

pub fn import() -> Result<Vec<String>, String> {
    let db_path = get_imessage_db_path()?;
    
    let conn = Connection::open(&db_path)
        .map_err(|e| format!("Failed to open iMessage database: {}. Make sure you have Full Disk Access enabled.", e))?;
    
    let mut stmt = conn.prepare(
        r#"
        SELECT 
            COALESCE(m.text, '') as text,
            COALESCE(h.id, 'Me') as sender,
            datetime(m.date/1000000000 + 978307200, 'unixepoch', 'localtime') as timestamp
        FROM message m
        LEFT JOIN handle h ON m.handle_id = h.ROWID
        WHERE m.text IS NOT NULL AND m.text != ''
        ORDER BY m.date DESC
        LIMIT 50000
        "#
    ).map_err(|e| format!("Failed to prepare query: {}", e))?;
    
    let messages: Vec<String> = stmt.query_map([], |row| {
        let text: String = row.get(0)?;
        let sender: String = row.get(1)?;
        let timestamp: String = row.get(2)?;
        Ok(format!("[{}] {}: {}", timestamp, sender, text))
    })
    .map_err(|e| format!("Failed to query messages: {}", e))?
    .filter_map(|r| r.ok())
    .filter(|msg| !msg.is_empty())
    .collect();
    
    if messages.is_empty() {
        return Err("No messages found in iMessage database. Make sure you have Full Disk Access enabled in System Preferences > Security & Privacy > Privacy.".to_string());
    }
    
    Ok(messages)
}

fn get_imessage_db_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir()
        .ok_or_else(|| "Could not find home directory".to_string())?;
    
    let db_path = home
        .join("Library")
        .join("Messages")
        .join("chat.db");
    
    if !db_path.exists() {
        return Err(format!(
            "iMessage database not found at {:?}. Make sure Messages app has been used on this Mac.",
            db_path
        ));
    }
    
    Ok(db_path)
}
