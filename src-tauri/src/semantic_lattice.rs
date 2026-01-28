use std::collections::HashMap;

/// Semantic Compression Lattice (SCL) implementation
/// Based on Barrett & Agents, December 2025
/// 
/// L = (V, E, κ, I, ∇SAL)
/// - V: Meaning atoms (entities in the user's world)
/// - E: Directed hyperedges (relationships)
/// - κ: Curvature functional (semantic energy)
/// - I: Invariant shells (identity-preserving constraints)
/// - ∇SAL: Teleological gradient (SAL's learning direction)

const LAMBDA: f32 = 0.1;
const BETA: f32 = 1.0;
const DELTA: f32 = 0.5;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MeaningAtom {
    pub id: String,
    pub embedding: Vec<f32>,
    pub frequency: usize,
    pub curvature: f32,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct Hyperedge {
    pub sources: Vec<String>,
    pub target: String,
    pub weight: f32,
    pub relationship_type: RelationType,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum RelationType {
    Family,
    Partner,
    Friend,
    Coworker,
    Acquaintance,
    Unknown,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct InvariantShell {
    pub name: String,
    pub members: Vec<String>,
    pub tolerance: f32,
    pub predicate: ShellPredicate,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum ShellPredicate {
    IsFamily,
    IsPartner,
    IsFrequentContact,
    SharesContext(String),
}

pub struct SemanticLattice {
    atoms: HashMap<String, MeaningAtom>,
    edges: Vec<Hyperedge>,
    shells: Vec<InvariantShell>,
}

#[derive(Default)]
#[allow(dead_code)]
struct EntityContext {
    message_count: usize,
    mention_count: usize,
    family_score: usize,
    partner_score: usize,
    is_sender: bool,
}

#[allow(dead_code)]
impl EntityContext {
    fn new() -> Self {
        Self::default()
    }
}

impl SemanticLattice {
    pub fn new() -> Self {
        Self {
            atoms: HashMap::new(),
            edges: Vec::new(),
            shells: Vec::new(),
        }
    }

    /// κ(v) = ||∇L_world(v)||² + λ||v||²
    /// Measures semantic energy - lower = more compressed/fundamental
    fn compute_curvature(&self, embedding: &[f32], frequency: usize) -> f32 {
        let norm_sq: f32 = embedding.iter().map(|x| x * x).sum();
        let freq_factor = 1.0 / (frequency as f32 + 1.0).ln();
        norm_sq * LAMBDA + freq_factor
    }

    /// Add a meaning atom (entity) to the lattice
    pub fn add_atom(&mut self, id: &str, embedding: Vec<f32>, frequency: usize) {
        let curvature = self.compute_curvature(&embedding, frequency);
        self.atoms.insert(id.to_string(), MeaningAtom {
            id: id.to_string(),
            embedding,
            frequency,
            curvature,
        });
    }

    /// Add a hyperedge (relationship) between atoms
    /// e = (T_e → h_e) is admissible if κ(h_e) + δ ≤ min κ(u) for u ∈ T_e
    pub fn add_edge(&mut self, sources: Vec<String>, target: String, rel_type: RelationType) {
        let target_curvature = self.atoms.get(&target)
            .map(|a| a.curvature)
            .unwrap_or(f32::MAX);
        
        let min_source_curvature = sources.iter()
            .filter_map(|s| self.atoms.get(s))
            .map(|a| a.curvature)
            .fold(f32::MAX, f32::min);
        
        // Admissibility check
        let is_admissible = target_curvature + DELTA <= min_source_curvature;
        
        // Weight: w_e = exp(-β(κ(h_e) - min κ(u)))
        let weight = if is_admissible {
            (-BETA * (target_curvature - min_source_curvature)).exp()
        } else {
            0.1 // Low weight for non-admissible edges
        };
        
        self.edges.push(Hyperedge {
            sources,
            target,
            weight,
            relationship_type: rel_type,
        });
    }

    /// Define an invariant shell - identity that must be preserved
    #[allow(dead_code)]
    pub fn add_shell(&mut self, name: &str, members: Vec<String>, predicate: ShellPredicate) {
        self.shells.push(InvariantShell {
            name: name.to_string(),
            members,
            tolerance: 0.0, // Strict invariance
            predicate,
        });
    }

    /// Learn relationships from co-occurrence patterns
    #[allow(dead_code)]
    pub fn learn_from_messages(&mut self, messages: &[String]) {
        // Blocklist: common words that aren't people
        let blocklist = [
            // Games/Apps
            "Wordle", "Puzzle", "Connections", "Game", "App", "Link", "Click",
            // Common sentence starters
            "The", "This", "That", "What", "When", "Where", "Why", "How", "Who",
            "I", "We", "You", "It", "They", "He", "She", "My", "Your", "Our",
            "I'm", "I'll", "I've", "We're", "You're", "It's", "That's", "What's",
            // Common words that get capitalized
            "Yeah", "Yes", "No", "Okay", "Ok", "Thanks", "Thank", "Please", "Sorry",
            "And", "But", "So", "If", "Or", "Just", "Like", "Really", "Very",
            "Good", "Great", "Nice", "Love", "Happy", "Today", "Tomorrow", "Yesterday",
            "Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday",
            "January", "February", "March", "April", "May", "June", "July", 
            "August", "September", "October", "November", "December",
            "Hey", "Hi", "Hello", "Bye", "Lol", "Haha", "Omg", "Wow",
            // URLs and tech
            "Http", "Https", "Www", "Com", "Org", "Net",
        ];
        
        // Relationship context words
        let family_context = ["mom", "dad", "mother", "father", "brother", "sister", 
                              "family", "parent", "son", "daughter", "grandma", "grandpa"];
        let partner_context = ["love you", "babe", "honey", "wife", "husband", "partner",
                               "miss you", "love ya", "❤", "😘", "💕"];
        
        let mut entity_contexts: HashMap<String, EntityContext> = HashMap::new();
        
        for msg in messages {
            // Extract sender from message format: [timestamp] Sender: message
            let sender = self.extract_sender(msg);
            
            // Find potential names (capitalized words not in blocklist)
            let words: Vec<&str> = msg.split_whitespace().collect();
            let potential_names: Vec<String> = words.iter()
                .filter(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
                .filter(|w| w.len() >= 2 && w.len() <= 15)
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                .filter(|w| !w.is_empty() && w.len() >= 2)
                .filter(|w| !blocklist.iter().any(|b| b.eq_ignore_ascii_case(w)))
                .filter(|w| !w.chars().all(|c| c.is_numeric()))
                .collect();
            
            let lower = msg.to_lowercase();
            let is_family_context = family_context.iter().any(|w| lower.contains(w));
            let is_partner_context = partner_context.iter().any(|w| lower.contains(w));
            
            // Add sender as entity if valid
            if let Some(ref s) = sender {
                if !blocklist.iter().any(|b| b.eq_ignore_ascii_case(s)) && s.len() >= 2 {
                    let ctx = entity_contexts.entry(s.clone()).or_insert(EntityContext::new());
                    ctx.message_count += 1;
                    ctx.is_sender = true;
                    if is_family_context { ctx.family_score += 1; }
                    if is_partner_context { ctx.partner_score += 1; }
                }
            }
            
            // Add mentioned names
            for name in &potential_names {
                let ctx = entity_contexts.entry(name.clone()).or_insert(EntityContext::new());
                ctx.mention_count += 1;
                if is_family_context { ctx.family_score += 1; }
                if is_partner_context { ctx.partner_score += 1; }
            }
        }
        
        // Calculate invariant score: entities that appear consistently as senders
        // or are mentioned in relationship contexts are true invariants
        for (name, ctx) in &entity_contexts {
            let invariant_score = 
                (ctx.message_count as f32 * 2.0) +  // Senders are strong signals
                (ctx.mention_count as f32 * 0.5) +  // Mentions are weaker
                (ctx.family_score as f32 * 5.0) +   // Family context is very strong
                (ctx.partner_score as f32 * 10.0);  // Partner context is strongest
            
            if invariant_score >= 10.0 {
                let embedding = self.generate_embedding(name);
                let total_freq = ctx.message_count + ctx.mention_count;
                self.add_atom(name, embedding, total_freq);
                
                // Determine relationship type
                let rel_type = if ctx.partner_score > 5 {
                    RelationType::Partner
                } else if ctx.family_score > 3 {
                    RelationType::Family
                } else if ctx.message_count > 50 {
                    RelationType::Friend
                } else {
                    RelationType::Acquaintance
                };
                
                // Store relationship type in atom (via edge to self for now)
                if rel_type != RelationType::Acquaintance {
                    self.add_edge(vec![name.clone()], name.clone(), rel_type);
                }
            }
        }
    }
    
    /// Learn from actual WhatsApp contacts - the scalable approach
    /// Works for ANY WhatsApp user by querying ZWACHATSESSION
    fn learn_from_contacts(&mut self, contacts: &[ContactStats]) {
        for contact in contacts {
            // Skip empty names or status broadcasts
            if contact.name.is_empty() || contact.name == "status@broadcast" {
                continue;
            }
            
            // Generate Braille embedding for the contact name
            let embedding = self.generate_embedding(&contact.name);
            
            // Add as meaning atom with message count as frequency
            self.add_atom(&contact.name, embedding, contact.message_count);
            
            // Determine relationship type based on message frequency
            // This heuristic works for any user:
            // - Very high frequency (500+) = likely partner/family
            // - High frequency (100+) = close friend
            // - Medium frequency (20+) = friend
            // - Lower = acquaintance
            let rel_type = if contact.message_count >= 500 {
                RelationType::Partner // or close family
            } else if contact.message_count >= 100 {
                RelationType::Family // close relationship
            } else if contact.message_count >= 20 {
                RelationType::Friend
            } else {
                RelationType::Acquaintance
            };
            
            // Add self-edge to mark relationship type (for non-acquaintances)
            if rel_type != RelationType::Acquaintance {
                self.add_edge(vec![contact.name.clone()], contact.name.clone(), rel_type);
            }
        }
    }

    #[allow(dead_code)]
    fn extract_sender(&self, msg: &str) -> Option<String> {
        // Format: [timestamp] Sender: message
        if let Some(bracket_end) = msg.find(']') {
            let after_bracket = &msg[bracket_end + 1..];
            if let Some(colon_pos) = after_bracket.find(':') {
                let sender = after_bracket[..colon_pos].trim();
                if !sender.is_empty() && sender != "Me" {
                    return Some(sender.to_string());
                }
            }
        }
        None
    }

    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        let mut embed = vec![0.0f32; 64];
        for (i, byte) in text.bytes().enumerate() {
            for bit in 0..8 {
                let idx = (i * 8 + bit) % 64;
                embed[idx] += ((byte >> bit) & 1) as f32 / (i as f32 + 1.0);
            }
        }
        // Normalize
        let norm: f32 = embed.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut embed {
                *v /= norm;
            }
        }
        embed
    }

    /// Generate a knowledge summary for SAL's system prompt
    pub fn generate_knowledge_prompt(&self) -> String {
        let mut prompt = String::new();
        
        // Sort atoms by frequency (most important first)
        let mut sorted_atoms: Vec<_> = self.atoms.values().collect();
        sorted_atoms.sort_by(|a, b| b.frequency.cmp(&a.frequency));
        
        if !sorted_atoms.is_empty() {
            prompt.push_str("Key people in the user's life (by importance):\n");
            for atom in sorted_atoms.iter().take(10) {
                prompt.push_str(&format!("- {} (mentioned {} times)\n", atom.id, atom.frequency));
            }
        }
        
        // Add relationship insights
        let strong_edges: Vec<_> = self.edges.iter()
            .filter(|e| e.weight > 0.5)
            .take(5)
            .collect();
        
        if !strong_edges.is_empty() {
            prompt.push_str("\nStrong relationships detected:\n");
            for edge in strong_edges {
                prompt.push_str(&format!("- {} ↔ {}\n", 
                    edge.sources.join(", "), 
                    edge.target
                ));
            }
        }
        
        // Add invariant shells
        for shell in &self.shells {
            prompt.push_str(&format!("\nInvariant: {} includes: {}\n",
                shell.name,
                shell.members.join(", ")
            ));
        }
        
        prompt
    }
}

/// Build a semantic lattice from WhatsApp contacts
/// This is the scalable approach - works for ANY WhatsApp user
pub fn build_lattice_from_messages() -> SemanticLattice {
    let mut lattice = SemanticLattice::new();
    
    // Load actual contacts from WhatsApp DB (not message text)
    if let Ok(contacts) = load_contacts_for_lattice() {
        lattice.learn_from_contacts(&contacts);
    }
    
    lattice
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct PersonInfo {
    pub name: String,
    pub mentions: usize,
    pub curvature: f32,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct RelationshipInfo {
    pub person1: String,
    pub person2: String,
    pub strength: f32,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct LatticeSnapshot {
    pub key_people: Vec<PersonInfo>,
    pub relationships: Vec<RelationshipInfo>,
    pub total_atoms: usize,
    pub total_edges: usize,
}

pub fn get_lattice_snapshot() -> LatticeSnapshot {
    let lattice = build_lattice_from_messages();
    
    let mut sorted_atoms: Vec<_> = lattice.atoms.values().collect();
    sorted_atoms.sort_by(|a, b| b.frequency.cmp(&a.frequency));
    
    let key_people: Vec<PersonInfo> = sorted_atoms.iter()
        .take(15)
        .map(|a| PersonInfo {
            name: a.id.clone(),
            mentions: a.frequency,
            curvature: a.curvature,
        })
        .collect();
    
    let relationships: Vec<RelationshipInfo> = lattice.edges.iter()
        .filter(|e| e.weight > 0.3)
        .take(10)
        .map(|e| RelationshipInfo {
            person1: e.sources.first().cloned().unwrap_or_default(),
            person2: e.target.clone(),
            strength: e.weight,
        })
        .collect();
    
    LatticeSnapshot {
        key_people,
        relationships,
        total_atoms: lattice.atoms.len(),
        total_edges: lattice.edges.len(),
    }
}

#[allow(dead_code)]
fn load_messages_for_lattice() -> Result<Vec<String>, String> {
    // This is now unused - we use load_contacts_for_lattice instead
    Ok(vec![])
}

/// Contact with message count - the TRUE invariant shells
#[derive(Debug)]
struct ContactStats {
    name: String,
    message_count: usize,
    #[allow(dead_code)]
    is_group: bool,
}

/// Load actual contacts from WhatsApp database
/// This scales to ANY WhatsApp user - we query the chat sessions table
/// which contains real contact names (ZPARTNERNAME) and message counts
fn load_contacts_for_lattice() -> Result<Vec<ContactStats>, String> {
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
    
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Failed to open database: {}", e))?;
    
    // Query chat sessions to get contact names and message counts
    // ZPARTNERNAME = actual contact name (works for any WhatsApp user)
    // ZMESSAGECOUNTER = total messages in that chat
    // ZCONTACTJID ending in @g.us = group chat, @s.whatsapp.net = individual
    let mut stmt = conn.prepare(
        r#"
        SELECT 
            ZPARTNERNAME,
            ZMESSAGECOUNTER,
            CASE WHEN ZCONTACTJID LIKE '%@g.us' THEN 1 ELSE 0 END as is_group
        FROM ZWACHATSESSION 
        WHERE ZPARTNERNAME IS NOT NULL 
          AND ZPARTNERNAME != ''
          AND ZMESSAGECOUNTER > 0
        ORDER BY ZMESSAGECOUNTER DESC
        "#
    ).map_err(|e| e.to_string())?;
    
    let contacts: Vec<ContactStats> = stmt.query_map([], |row| {
        Ok(ContactStats {
            name: row.get(0)?,
            message_count: row.get::<_, i64>(1)? as usize,
            is_group: row.get::<_, i64>(2)? == 1,
        })
    })
    .map_err(|e| e.to_string())?
    .filter_map(|r| r.ok())
    .collect();
    
    Ok(contacts)
}
