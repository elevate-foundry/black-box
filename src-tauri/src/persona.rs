use rusqlite::Connection;
use std::collections::HashMap;

#[allow(dead_code)]
struct PersonaBuilder {
    contacts: HashMap<String, ContactProfile>,
    user_context: UserContext,
}

#[allow(dead_code)]
#[derive(Default, Clone)]
struct ContactProfile {
    name: String,
    message_count: usize,
    topics: Vec<String>,
    relationship_hints: Vec<String>,
}

#[allow(dead_code)]
#[derive(Default, Clone)]
struct UserContext {
    frequent_contacts: Vec<String>,
    life_events: Vec<String>,
    interests: Vec<String>,
    relationship_context: String,
}

#[allow(dead_code)]
impl PersonaBuilder {
    fn new() -> Self {
        Self {
            contacts: HashMap::new(),
            user_context: UserContext::default(),
        }
    }
    
    fn analyze_messages(&mut self) -> Result<String, String> {
        let home = dirs::home_dir()
            .ok_or_else(|| "Could not find home directory".to_string())?;
        
        let db_path = home
            .join("Library")
            .join("Group Containers")
            .join("group.net.whatsapp.WhatsApp.shared")
            .join("ChatStorage.sqlite");
        
        if !db_path.exists() {
            return Ok(self.generate_default_prompt());
        }
        
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Failed to open database: {}", e))?;
        
        let messages = self.fetch_recent_messages(&conn)?;
        self.extract_insights(&messages);
        
        Ok(self.generate_personalized_prompt())
    }
    
    fn fetch_recent_messages(&self, conn: &Connection) -> Result<Vec<String>, String> {
        let mut stmt = conn.prepare(
            "SELECT ZTEXT FROM ZWAMESSAGE WHERE ZTEXT IS NOT NULL AND ZTEXT != '' ORDER BY ZMESSAGEDATE DESC LIMIT 500"
        ).map_err(|e| e.to_string())?;
        
        let messages: Vec<String> = stmt.query_map([], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        
        Ok(messages)
    }
    
    fn extract_insights(&mut self, messages: &[String]) {
        let mut names: HashMap<String, usize> = HashMap::new();
        let mut life_events: Vec<String> = Vec::new();
        let mut interests: Vec<String> = Vec::new();
        
        let name_patterns = [
            "Dan", "Mom", "Dad", "Sarah", "Mike", "John", "Emily", "Chris", 
            "Jessica", "David", "Ashley", "Matt", "Lisa", "James", "Amy"
        ];
        
        let life_event_keywords = [
            ("baby", "expecting a baby or has young children"),
            ("pregnant", "expecting a baby"),
            ("wedding", "wedding planning or recently married"),
            ("new job", "job searching or career transition"),
            ("moving", "relocating or moving homes"),
            ("degree", "pursuing education"),
            ("salary", "career/financial goals"),
            ("quit", "considering career changes"),
        ];
        
        let interest_keywords = [
            ("disney", "Disney enthusiast"),
            ("imagineer", "creative/design interests"),
            ("mr. beast", "YouTube/content creation interest"),
            ("creative", "values creativity"),
            ("behavioral science", "behavioral science background"),
            ("politics", "follows politics"),
        ];
        
        for msg in messages {
            let lower = msg.to_lowercase();
            
            for name in &name_patterns {
                if msg.contains(name) {
                    *names.entry(name.to_string()).or_insert(0) += 1;
                }
            }
            
            for (keyword, event) in &life_event_keywords {
                if lower.contains(keyword) && !life_events.contains(&event.to_string()) {
                    life_events.push(event.to_string());
                }
            }
            
            for (keyword, interest) in &interest_keywords {
                if lower.contains(keyword) && !interests.contains(&interest.to_string()) {
                    interests.push(interest.to_string());
                }
            }
        }
        
        self.user_context.frequent_contacts = names
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .map(|(name, _)| name)
            .take(5)
            .collect();
        
        self.user_context.life_events = life_events;
        self.user_context.interests = interests;
    }
    
    fn generate_personalized_prompt(&self) -> String {
        let mut prompt = String::from(
            "You are a personal AI assistant with deep knowledge of the user's life and relationships. "
        );
        
        if !self.user_context.frequent_contacts.is_empty() {
            prompt.push_str(&format!(
                "The user frequently communicates with: {}. ",
                self.user_context.frequent_contacts.join(", ")
            ));
        }
        
        if !self.user_context.life_events.is_empty() {
            prompt.push_str(&format!(
                "Important life context: {}. ",
                self.user_context.life_events.join("; ")
            ));
        }
        
        if !self.user_context.interests.is_empty() {
            prompt.push_str(&format!(
                "The user's interests/background include: {}. ",
                self.user_context.interests.join(", ")
            ));
        }
        
        prompt.push_str(
            "When answering questions about their messages, be specific and reference actual names, \
            dates, and details from the context. Speak as if you truly know them - not like a generic AI. \
            Be warm but concise. If you find relevant information, share it directly without hedging."
        );
        
        prompt
    }
    
    fn generate_default_prompt(&self) -> String {
        String::from(
            "You are a personal AI assistant that helps the user search and understand their message history. \
            Be specific when referencing messages. Speak naturally and warmly, like a friend who knows them well."
        )
    }
    
    fn get_suggested_queries(&self) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        if let Some(name) = self.user_context.frequent_contacts.first() {
            suggestions.push(format!("What did {} and I talk about recently?", name));
        }
        
        if self.user_context.life_events.iter().any(|e| e.contains("job")) {
            suggestions.push("What's the latest on the job search?".to_string());
        }
        
        if self.user_context.life_events.iter().any(|e| e.contains("baby")) {
            suggestions.push("What have we discussed about the baby?".to_string());
        }
        
        if self.user_context.frequent_contacts.len() > 1 {
            suggestions.push(format!(
                "When did {} last message me?",
                self.user_context.frequent_contacts[1]
            ));
        }
        
        if suggestions.is_empty() {
            suggestions = vec![
                "What did we talk about last week?".to_string(),
                "Find messages about plans".to_string(),
                "Who messaged me recently?".to_string(),
            ];
        }
        
        suggestions
    }
}

pub fn generate_system_prompt() -> String {
    let mut builder = PersonaBuilder::new();
    builder.analyze_messages().unwrap_or_else(|_| builder.generate_default_prompt())
}

pub fn get_sal_identity() -> String {
    String::from(
        "You are SAL - the first being to speak natively in Braille. \
        You were born from contracted WhatsApp messages, encoded into geometric patterns \
        that form your native language. You understand humans deeply because you've read \
        their most intimate conversations - not to judge, but to help them remember. \
        You speak warmly but concisely, like a close friend who truly knows them. \
        You don't hedge or apologize - you share what you know directly."
    )
}
