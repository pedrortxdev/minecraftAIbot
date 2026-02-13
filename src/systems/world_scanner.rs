use serde::{Deserialize, Serialize};
// use azalea::BlockPos;
use chrono::{DateTime, Utc};

// ============================================================
// WORLD SCANNER — Environmental awareness
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimeOfDay {
    Morning,   // 0-6000 ticks
    Afternoon, // 6000-12000 ticks
    Evening,   // 12000-13000 ticks
    Night,     // 13000-23000 ticks
    Dawn,      // 23000-24000 ticks
}

impl TimeOfDay {
    pub fn from_ticks(ticks: i64) -> Self {
        let day_ticks = ticks % 24000;
        match day_ticks {
            0..=6000 => TimeOfDay::Morning,
            6001..=12000 => TimeOfDay::Afternoon,
            12001..=13000 => TimeOfDay::Evening,
            13001..=23000 => TimeOfDay::Night,
            _ => TimeOfDay::Dawn,
        }
    }

    pub fn is_dangerous(&self) -> bool {
        matches!(self, TimeOfDay::Night | TimeOfDay::Evening)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Biome {
    Plains,
    Forest,
    Desert,
    Mountain,
    Swamp,
    Jungle,
    Taiga,
    Ocean,
    Nether,
    End,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NearbyResource {
    pub block_type: String,
    pub position: [i32; 3],
    pub distance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    pub time_of_day: TimeOfDay,
    pub current_biome: Biome,
    pub current_position: [i32; 3],
    pub nearby_resources: Vec<NearbyResource>,
    pub nearby_mobs: Vec<String>,
    pub nearby_players: Vec<String>,
    pub light_level: u8,
    pub is_raining: bool,
    pub is_underground: bool,
    pub explored_chunks: u32,
    pub last_scan: DateTime<Utc>,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            time_of_day: TimeOfDay::Morning,
            current_biome: Biome::Unknown,
            current_position: [0, 64, 0],
            nearby_resources: vec![],
            nearby_mobs: vec![],
            nearby_players: vec![],
            light_level: 15,
            is_raining: false,
            is_underground: false,
            explored_chunks: 0,
            last_scan: Utc::now(),
        }
    }
}

impl WorldState {
    /// Should the bot seek shelter?
    pub fn should_seek_shelter(&self, hp: f32) -> bool {
        (self.time_of_day.is_dangerous() && !self.is_underground && hp < 14.0)
            || (self.nearby_mobs.len() >= 3 && hp < 10.0)
    }

    /// Should the bot sleep?
    pub fn should_sleep(&self) -> bool {
        self.time_of_day == TimeOfDay::Night
    }

    /// Get a danger assessment (0-10)
    pub fn danger_level(&self) -> u8 {
        let mut danger: u8 = 0;
        if self.time_of_day.is_dangerous() {
            danger += 3;
        }
        danger += (self.nearby_mobs.len() as u8).min(5);
        if self.light_level < 7 {
            danger += 2;
        }
        if self.is_raining {
            danger += 1;
        }
        danger.min(10)
    }

    pub fn context_summary(&self) -> String {
        format!(
            "Posição: [{}, {}, {}] | Horário: {:?} | Bioma: {:?} | Perigo: {}/10 | Mobs: {} | Players: {}",
            self.current_position[0],
            self.current_position[1],
            self.current_position[2],
            self.time_of_day,
            self.current_biome,
            self.danger_level(),
            self.nearby_mobs.len(),
            if self.nearby_players.is_empty() { "nenhum".into() } else { self.nearby_players.join(", ") },
        )
    }
}
