use regex::Regex;
use std::fs;
use walkdir::WalkDir;

pub fn import() -> Result<Vec<String>, String> {
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
            "No WhatsApp exports found. Please export a chat from WhatsApp and place the .txt file in your Downloads or Documents folder.".to_string()
        );
    }
    
    Ok(all_messages)
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
