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
    pub fn learn_from_messages(&mut self, messages: &[String]) {
        let mut cooccurrence: HashMap<(String, String), usize> = HashMap::new();
        let mut entity_freq: HashMap<String, usize> = HashMap::new();
        
        // Common relationship indicators
        let family_words = ["mom", "dad", "mother", "father", "brother", "sister", "family"];
        let partner_words = ["love", "babe", "honey", "wife", "husband", "partner"];
        
        for msg in messages {
            let words: Vec<&str> = msg.split_whitespace().collect();
            let capitalized: Vec<String> = words.iter()
                .filter(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
                .filter(|w| w.len() >= 2 && w.len() <= 20)
                .filter(|w| !["The", "This", "That", "What", "When", "I", "We", "You", "It"].contains(w))
                .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
                .filter(|w| !w.is_empty())
                .collect();
            
            // Count entity frequencies
            for entity in &capitalized {
                *entity_freq.entry(entity.clone()).or_insert(0) += 1;
            }
            
            // Count co-occurrences
            for i in 0..capitalized.len() {
                for j in (i+1)..capitalized.len() {
                    let pair = if capitalized[i] < capitalized[j] {
                        (capitalized[i].clone(), capitalized[j].clone())
                    } else {
                        (capitalized[j].clone(), capitalized[i].clone())
                    };
                    *cooccurrence.entry(pair).or_insert(0) += 1;
                }
            }
            
            // Detect relationship types from context
            let lower = msg.to_lowercase();
            for entity in &capitalized {
                if family_words.iter().any(|w| lower.contains(w)) {
                    let _ = self.atoms.get_mut(entity);
                    // Mark as potential family
                }
                if partner_words.iter().any(|w| lower.contains(w)) {
                    // Mark as potential partner
                }
            }
        }
        
        // Add atoms for frequent entities
        for (entity, freq) in &entity_freq {
            if *freq >= 3 {
                let embedding = self.generate_embedding(entity);
                self.add_atom(entity, embedding, *freq);
            }
        }
        
        // Add edges for strong co-occurrences
        for ((e1, e2), count) in &cooccurrence {
            if *count >= 2 {
                self.add_edge(
                    vec![e1.clone()],
                    e2.clone(),
                    RelationType::Unknown
                );
            }
        }
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

/// Build a semantic lattice from WhatsApp messages
pub fn build_lattice_from_messages() -> SemanticLattice {
    let mut lattice = SemanticLattice::new();
    
    // Load messages from WhatsApp DB
    if let Ok(messages) = load_messages_for_lattice() {
        lattice.learn_from_messages(&messages);
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

fn load_messages_for_lattice() -> Result<Vec<String>, String> {
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
    
    let mut stmt = conn.prepare(
        "SELECT ZTEXT FROM ZWAMESSAGE WHERE ZTEXT IS NOT NULL AND ZTEXT != '' ORDER BY ZMESSAGEDATE DESC LIMIT 5000"
    ).map_err(|e| e.to_string())?;
    
    let messages: Vec<String> = stmt.query_map([], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();
    
    Ok(messages)
}
