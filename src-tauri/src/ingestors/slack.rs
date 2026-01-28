use serde::Deserialize;
use std::fs;
use walkdir::WalkDir;

#[derive(Debug, Deserialize)]
struct SlackMessage {
    #[serde(default)]
    user: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    ts: Option<String>,
    #[serde(default)]
    user_profile: Option<SlackUserProfile>,
}

#[derive(Debug, Deserialize)]
struct SlackUserProfile {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    real_name: Option<String>,
}

pub fn import() -> Result<Vec<String>, String> {
    let downloads_dir = dirs::download_dir()
        .ok_or_else(|| "Could not find Downloads directory".to_string())?;
    
    let documents_dir = dirs::document_dir()
        .ok_or_else(|| "Could not find Documents directory".to_string())?;
    
    let mut all_messages = Vec::new();
    
    for search_dir in [downloads_dir, documents_dir] {
        for entry in WalkDir::new(&search_dir)
            .max_depth(5)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            if path.is_file() && path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Some(parent) = path.parent() {
                    let parent_name = parent.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    
                    if !parent_name.starts_with('.') && 
                       path.file_name()
                           .and_then(|n| n.to_str())
                           .map(|n| n.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false))
                           .unwrap_or(false)
                    {
                        match parse_slack_file(path, parent_name) {
                            Ok(messages) => all_messages.extend(messages),
                            Err(_) => {}
                        }
                    }
                }
            }
        }
    }
    
    if all_messages.is_empty() {
        return Err(
            "No Slack exports found. Please export your Slack workspace and place the folder in your Downloads or Documents folder.".to_string()
        );
    }
    
    Ok(all_messages)
}

fn parse_slack_file(path: &std::path::Path, channel: &str) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let messages: Vec<SlackMessage> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;
    
    let formatted: Vec<String> = messages
        .into_iter()
        .filter_map(|msg| {
            let text = msg.text?;
            if text.is_empty() {
                return None;
            }
            
            let sender = msg.user_profile
                .and_then(|p| p.display_name.or(p.real_name))
                .or(msg.user)
                .unwrap_or_else(|| "unknown".to_string());
            
            let timestamp = msg.ts
                .and_then(|ts| {
                    ts.split('.').next()
                        .and_then(|s| s.parse::<i64>().ok())
                        .map(|epoch| {
                            chrono::DateTime::from_timestamp(epoch, 0)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                                .unwrap_or_else(|| "unknown".to_string())
                        })
                })
                .unwrap_or_else(|| "unknown".to_string());
            
            Some(format!("[{}] #{} - {}: {}", timestamp, channel, sender, text))
        })
        .collect();
    
    Ok(formatted)
}
