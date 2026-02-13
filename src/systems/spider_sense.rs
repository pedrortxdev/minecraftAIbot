use serde::{Deserialize, Serialize};
use crate::cognitive::memory::SocialMemory;
// use rand::Rng;

// ============================================================
// SPIDER SENSE — Threat Prediction Engine
// Predict the future, act before it happens
// "Aquele cara com trust -50 tá vindo com lava na mão"
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreatLevel {
    None,
    Low,       // Suspicious but not urgent
    Medium,    // Something is probably about to happen
    High,      // Almost certain threat, act NOW
    Critical,  // Imminent death if we don't move
}

#[derive(Debug, Clone)]
pub struct PredictedThreat {
    pub threat_type: PredictionType,
    pub level: ThreatLevel,
    pub description: String,
    pub recommended_action: PredictedAction,
    pub time_to_impact_ms: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PredictionType {
    PlayerGriefing,       // Player with lava/TNT approaching
    FallingBlock,         // Gravel/sand above while mining up
    CreeperExplosion,     // Creeper close and hissing
    LavaFlow,             // Breaking block with lava behind
    FallDamage,           // Walking toward a cliff
    Drowning,             // In water with low bubbles
    SuffocationMining,    // Mining up into gravel
    PlayerAmbush,         // Enemy player sneaking nearby
    MobSwarm,             // Many hostiles spawning
    StarvationDeath,      // No food, hunger depleting
}

#[derive(Debug, Clone, PartialEq)]
pub enum PredictedAction {
    DoNothing,
    PlaceTorch,           // Under feet before mining up (anti-gravel)
    PlaceBlock,           // Block a path
    Sprint,               // Run away
    AttackFirst,          // Preemptive strike
    Tower,                // Tower up
    EatNow,               // Eat before HP too low
    SwimUp,               // Get air
    AvoidDirection,       // Don't go that way
    WarnChat(String),     // Warn in chat
}

#[derive(Debug, Clone, Default)]
pub struct SpiderSense {
    pub active_predictions: Vec<PredictedThreat>,
    pub predictions_made: u32,
    pub predictions_correct: u32,
    pub accuracy: f32,
}

impl SpiderSense {
    /// Analyze: Is a player approaching with dangerous items?
    pub fn predict_player_threat(
        &self,
        player: &str,
        held_item: &str,
        distance: f64,
        approaching: bool,
        trust: i32,
        _social: &SocialMemory,
    ) -> Option<PredictedThreat> {
        let dangerous_items = [
            "lava_bucket", "flint_and_steel", "tnt", "fire_charge",
            "end_crystal", "respawn_anchor",
        ];

        let weapons = [
            "diamond_sword", "netherite_sword", "iron_sword",
            "bow", "crossbow", "trident",
        ];

        let is_dangerous_item = dangerous_items.iter().any(|i| held_item.contains(i));
        let is_weapon = weapons.iter().any(|i| held_item.contains(i));

        // Enemy + dangerous item + approaching = CRITICAL
        if trust < -20 && is_dangerous_item && approaching && distance < 30.0 {
            return Some(PredictedThreat {
                threat_type: PredictionType::PlayerGriefing,
                level: ThreatLevel::Critical,
                description: format!("{} (trust:{}) vindo com {} a {}m", player, trust, held_item, distance as i32),
                recommended_action: PredictedAction::AttackFirst,
                time_to_impact_ms: (distance * 200.0) as u64, // ~200ms per block sprint
            });
        }

        // Low trust + weapon + approaching
        if trust < 10 && is_weapon && approaching && distance < 20.0 {
            return Some(PredictedThreat {
                threat_type: PredictionType::PlayerAmbush,
                level: ThreatLevel::High,
                description: format!("{} armado com {} se aproximando", player, held_item),
                recommended_action: if distance < 8.0 {
                    PredictedAction::AttackFirst
                } else {
                    PredictedAction::Sprint
                },
                time_to_impact_ms: (distance * 200.0) as u64,
            });
        }

        // Unknown player sneaking nearby
        if trust == 20 && distance < 15.0 && is_weapon {
            return Some(PredictedThreat {
                threat_type: PredictionType::PlayerAmbush,
                level: ThreatLevel::Medium,
                description: format!("{} desconhecido com {}", player, held_item),
                recommended_action: PredictedAction::WarnChat(
                    format!("eai {}, o que ce ta fazendo ai com {} na mão?", player, held_item)
                ),
                time_to_impact_ms: 5000,
            });
        }

        None
    }

    /// Analyze: Am I about to mine into falling blocks?
    pub fn predict_mining_danger(
        &self,
        block_above: &str,
        is_mining_up: bool,
    ) -> Option<PredictedThreat> {
        let falling_blocks = ["gravel", "sand", "red_sand", "anvil", "dragon_egg"];

        if is_mining_up && falling_blocks.iter().any(|b| block_above.contains(b)) {
            return Some(PredictedThreat {
                threat_type: PredictionType::SuffocationMining,
                level: ThreatLevel::High,
                description: format!("{} em cima, vai cair se quebrar", block_above),
                recommended_action: PredictedAction::PlaceTorch,
                time_to_impact_ms: 500,
            });
        }

        // Lava behind blocks at low Y
        if block_above == "stone" && is_mining_up {
            // Can't know for sure, but at Y < 11, lava is common
            return Some(PredictedThreat {
                threat_type: PredictionType::LavaFlow,
                level: ThreatLevel::Low,
                description: "Possível lava atrás do bloco (Y baixo)".into(),
                recommended_action: PredictedAction::PlaceBlock,
                time_to_impact_ms: 2000,
            });
        }

        None
    }

    /// Analyze: Am I going to starve?
    pub fn predict_starvation(
        &self,
        food_level: u32,
        hp: f32,
        has_food: bool,
    ) -> Option<PredictedThreat> {
        if food_level <= 6 && hp < 10.0 && !has_food {
            return Some(PredictedThreat {
                threat_type: PredictionType::StarvationDeath,
                level: if hp < 4.0 { ThreatLevel::Critical } else { ThreatLevel::High },
                description: format!("Fome: {} | HP: {:.0} | Sem comida!", food_level, hp),
                recommended_action: PredictedAction::EatNow,
                time_to_impact_ms: if hp < 4.0 { 2000 } else { 10000 },
            });
        }

        if food_level <= 6 && has_food {
            return Some(PredictedThreat {
                threat_type: PredictionType::StarvationDeath,
                level: ThreatLevel::Medium,
                description: "Fome baixa, comer agora".into(),
                recommended_action: PredictedAction::EatNow,
                time_to_impact_ms: 5000,
            });
        }

        None
    }

    /// Analyze: Is a creeper about to explode?
    pub fn predict_creeper_explosion(
        &self,
        creeper_distance: f64,
        creeper_fuse_started: bool,
    ) -> Option<PredictedThreat> {
        if creeper_fuse_started && creeper_distance < 5.0 {
            return Some(PredictedThreat {
                threat_type: PredictionType::CreeperExplosion,
                level: ThreatLevel::Critical,
                description: format!("CREEPER ASISSSSANDO a {}m!", creeper_distance as i32),
                recommended_action: PredictedAction::Sprint,
                time_to_impact_ms: 1500, // Creeper fuse is 1.5s
            });
        }

        if creeper_distance < 3.0 && !creeper_fuse_started {
            return Some(PredictedThreat {
                threat_type: PredictionType::CreeperExplosion,
                level: ThreatLevel::High,
                description: "Creeper muito perto, pode assar a qualquer momento".into(),
                recommended_action: PredictedAction::Sprint,
                time_to_impact_ms: 3000,
            });
        }

        None
    }

    /// Get the most urgent prediction
    pub fn most_urgent(&self) -> Option<&PredictedThreat> {
        self.active_predictions.iter().min_by_key(|p| {
            match p.level {
                ThreatLevel::Critical => 0,
                ThreatLevel::High => 1,
                ThreatLevel::Medium => 2,
                ThreatLevel::Low => 3,
                ThreatLevel::None => 4,
            }
        })
    }

    pub fn record_prediction(&mut self, threat: PredictedThreat) {
        self.predictions_made += 1;
        println!("[SPIDER] 🕷️ {:?}: {} → {:?}", threat.level, threat.description, threat.recommended_action);
        self.active_predictions.push(threat);

        // Keep only recent predictions
        if self.active_predictions.len() > 10 {
            self.active_predictions.drain(0..5);
        }
    }

    pub fn record_correct(&mut self) {
        self.predictions_correct += 1;
        self.accuracy = self.predictions_correct as f32 / self.predictions_made.max(1) as f32;
    }

    pub fn context_summary(&self) -> String {
        let active = self.active_predictions.len();
        format!(
            "Previsões: {} ativas | Total: {} | Precisão: {:.0}%",
            active, self.predictions_made, self.accuracy * 100.0
        )
    }
}
