use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const DATA_DIR: &str = "data";

// ============================================================
// EPISODIC MEMORY — "What happened"
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub timestamp: DateTime<Utc>,
    pub event_type: EpisodeType,
    pub description: String,
    pub location: Option<[i32; 3]>,
    pub players_involved: Vec<String>,
    pub emotional_impact: i8, // -5 (terrible) to +5 (amazing)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EpisodeType {
    Death,
    Kill,
    FoundResource,
    BuiltStructure,
    MetPlayer,
    ReceivedGift,
    GaveGift,
    TradeCompleted,
    AskedForHelp,
    WasAttacked,
    ExploredArea,
    FarmHarvest,
    CraftedItem,
    ChatConversation,
    ServerJoin,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EpisodicMemory {
    pub episodes: Vec<Episode>,
}

impl EpisodicMemory {
    pub fn add(&mut self, episode: Episode) {
        self.episodes.push(episode);
        // Keep last 500 episodes to avoid bloat
        if self.episodes.len() > 500 {
            self.episodes.drain(0..100);
        }
    }

    pub fn recent(&self, count: usize) -> Vec<&Episode> {
        self.episodes.iter().rev().take(count).collect()
    }

    pub fn recent_of_type(&self, etype: &EpisodeType, count: usize) -> Vec<&Episode> {
        self.episodes
            .iter()
            .rev()
            .filter(|e| std::mem::discriminant(&e.event_type) == std::mem::discriminant(etype))
            .take(count)
            .collect()
    }

    /// Get a summary string for the AI context window
    pub fn context_summary(&self, count: usize) -> String {
        let recent = self.recent(count);
        if recent.is_empty() {
            return "Nada de interessante aconteceu ainda.".to_string();
        }
        recent
            .iter()
            .map(|e| {
                format!(
                    "[{}] {} {}",
                    e.timestamp.format("%H:%M"),
                    e.description,
                    if e.emotional_impact > 2 {
                        "🔥"
                    } else if e.emotional_impact < -2 {
                        "💀"
                    } else {
                        ""
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ============================================================
// SPATIAL MEMORY — "Where things are"
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub name: String,
    pub coords: [i32; 3],
    pub location_type: LocationType,
    pub notes: String,
    pub discovered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LocationType {
    Home,
    Mine,
    Farm,
    Village,
    PlayerBase,
    Portal,
    Stronghold,
    SpawnerRoom,
    ResourceDeposit,
    DangerZone,
    DeathPoint,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SpatialMemory {
    pub locations: Vec<Location>,
    pub home_coords: Option<[i32; 3]>,
}

impl SpatialMemory {
    pub fn remember_location(&mut self, loc: Location) {
        // Update if same name exists
        if let Some(existing) = self.locations.iter_mut().find(|l| l.name == loc.name) {
            existing.coords = loc.coords;
            existing.notes = loc.notes;
        } else {
            self.locations.push(loc);
        }
    }

    pub fn set_home(&mut self, coords: [i32; 3]) {
        self.home_coords = Some(coords);
        self.remember_location(Location {
            name: "Base Principal".into(),
            coords,
            location_type: LocationType::Home,
            notes: "Minha base".into(),
            discovered_at: Utc::now(),
        });
    }

    pub fn nearest_of_type(&self, pos: [i32; 3], ltype: &LocationType) -> Option<&Location> {
        self.locations
            .iter()
            .filter(|l| &l.location_type == ltype)
            .min_by_key(|l| {
                let dx = (l.coords[0] - pos[0]) as i64;
                let dy = (l.coords[1] - pos[1]) as i64;
                let dz = (l.coords[2] - pos[2]) as i64;
                dx * dx + dy * dy + dz * dz
            })
    }

    pub fn context_summary(&self) -> String {
        if self.locations.is_empty() {
            return "Não conheço nenhum lugar ainda.".to_string();
        }
        let mut s = String::new();
        if let Some(h) = &self.home_coords {
            s.push_str(&format!("Base: [{}, {}, {}]\n", h[0], h[1], h[2]));
        }
        for loc in self.locations.iter().take(10) {
            s.push_str(&format!(
                "- {} ({:?}) em [{}, {}, {}]\n",
                loc.name, loc.location_type, loc.coords[0], loc.coords[1], loc.coords[2]
            ));
        }
        s
    }
}

// ============================================================
// SOCIAL MEMORY — "Who is who"
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub name: String,
    pub trust_level: i32, // 0 (stranger) to 100 (best friend), can go negative (enemy)
    pub times_met: u32,
    pub last_seen: DateTime<Utc>,
    pub gifts_received: Vec<String>,
    pub gifts_given: Vec<String>,
    pub help_requests_made: u32,
    pub help_requests_fulfilled: u32,
    pub notes: Vec<String>, // things the bot remembers about this player
    pub relationship: Relationship,
    pub last_messages: Vec<String>, // last 5 messages from this player
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Relationship {
    Stranger,
    Acquaintance,
    Friend,
    BestFriend,
    Rival,
    Enemy,
}

impl Default for PlayerProfile {
    fn default() -> Self {
        Self {
            name: String::new(),
            trust_level: 20,
            times_met: 0,
            last_seen: Utc::now(),
            gifts_received: vec![],
            gifts_given: vec![],
            help_requests_made: 0,
            help_requests_fulfilled: 0,
            notes: vec![],
            relationship: Relationship::Stranger,
            last_messages: vec![],
        }
    }
}

impl PlayerProfile {
    pub fn update_relationship(&mut self) {
        self.relationship = match self.trust_level {
            t if t < 0 => Relationship::Enemy,
            t if t < 10 => Relationship::Rival,
            t if t < 30 => Relationship::Stranger,
            t if t < 50 => Relationship::Acquaintance,
            t if t < 80 => Relationship::Friend,
            _ => Relationship::BestFriend,
        };
    }

    pub fn add_message(&mut self, msg: &str) {
        self.last_messages.push(msg.to_string());
        if self.last_messages.len() > 5 {
            self.last_messages.remove(0);
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SocialMemory {
    pub players: HashMap<String, PlayerProfile>,
}

impl SocialMemory {
    pub fn get_or_create(&mut self, name: &str) -> &mut PlayerProfile {
        self.players.entry(name.to_string()).or_insert_with(|| {
            let mut p = PlayerProfile::default();
            p.name = name.to_string();
            p
        })
    }

    pub fn record_interaction(&mut self, name: &str, trust_delta: i32) {
        let player = self.get_or_create(name);
        player.times_met += 1;
        player.last_seen = Utc::now();
        player.trust_level = (player.trust_level + trust_delta).clamp(-100, 100);
        player.update_relationship();
    }

    pub fn context_summary(&self) -> String {
        if self.players.is_empty() {
            return "Não conheço ninguém ainda.".to_string();
        }
        self.players
            .values()
            .take(10)
            .map(|p| {
                format!(
                    "- {} ({:?}, trust:{}, visto:{}x)",
                    p.name, p.relationship, p.trust_level, p.times_met
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ============================================================
// INVENTORY KNOWLEDGE — "What I have and need"
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InventoryKnowledge {
    pub crafting_history: Vec<String>,
    pub items_lost_on_death: Vec<String>,
    pub resource_priorities: Vec<String>, // what we're actively looking for
    pub failed_attempts: HashMap<String, u32>, // task → number of failures
}

impl InventoryKnowledge {
    pub fn record_craft(&mut self, item: &str) {
        if !self.crafting_history.contains(&item.to_string()) {
            self.crafting_history.push(item.to_string());
        }
    }

    pub fn record_failure(&mut self, task: &str) -> u32 {
        let count = self.failed_attempts.entry(task.to_string()).or_insert(0);
        *count += 1;
        *count
    }

    /// Returns true if we've failed enough times to justify asking for help
    pub fn should_ask_for_help(&self, task: &str) -> bool {
        self.failed_attempts.get(task).map_or(false, |&c| c >= 3)
    }

    pub fn context_summary(&self) -> String {
        let mut s = String::new();
        if !self.resource_priorities.is_empty() {
            s.push_str(&format!("Procurando: {}\n", self.resource_priorities.join(", ")));
        }
        for (task, count) in &self.failed_attempts {
            if *count >= 2 {
                s.push_str(&format!("Dificuldade com '{}' ({}x falhou)\n", task, count));
            }
        }
        s
    }
}

// ============================================================
// MASTER MEMORY — Combines everything
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Memory {
    pub episodes: EpisodicMemory,
    pub spatial: SpatialMemory,
    pub social: SocialMemory,
    pub inventory: InventoryKnowledge,
}

impl Memory {
    /// Load from disk or create fresh
    pub fn load() -> Self {
        let path = format!("{}/memory.json", DATA_DIR);
        if Path::new(&path).exists() {
            match fs::read_to_string(&path) {
                Ok(data) => match serde_json::from_str::<Memory>(&data) {
                    Ok(mem) => {
                        println!("[MEMORY] Loaded {} episodes, {} locations, {} players",
                            mem.episodes().episodes.len(),
                            mem.spatial().locations.len(),
                            mem.social().players.len(),
                        );
                        return mem;
                    }
                    Err(e) => {
                        println!("[MEMORY] Failed to parse memory.json: {}. Starting fresh.", e);
                    }
                },
                Err(e) => {
                    println!("[MEMORY] Failed to read memory.json: {}. Starting fresh.", e);
                }
            }
        }
        println!("[MEMORY] No existing memory found. Starting fresh.");
        Self::default()
    }

    fn episodes(&self) -> &EpisodicMemory {
        &self.episodes
    }

    fn spatial(&self) -> &SpatialMemory {
        &self.spatial
    }

    fn social(&self) -> &SocialMemory {
        &self.social
    }

    /// Save to disk
    pub fn save(&self) {
        let _ = fs::create_dir_all(DATA_DIR);
        let path = format!("{}/memory.json", DATA_DIR);
        match serde_json::to_string_pretty(self) {
            Ok(data) => {
                if let Err(e) = fs::write(&path, data) {
                    println!("[MEMORY] Failed to save: {}", e);
                }
            }
            Err(e) => println!("[MEMORY] Failed to serialize: {}", e),
        }
    }

    /// Build a full context string for the AI
    pub fn full_context(&self) -> String {
        format!(
            "=== MEMÓRIA ===\n\
             Eventos recentes:\n{}\n\n\
             Lugares conhecidos:\n{}\n\n\
             Jogadores:\n{}\n\n\
             Inventário:\n{}",
            self.episodes.context_summary(5),
            self.spatial.context_summary(),
            self.social.context_summary(),
            self.inventory.context_summary(),
        )
    }
}
