use serde::{Deserialize, Serialize};

// ============================================================
// COMBAT — Intelligent fighting
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CombatState {
    Peaceful,
    Alert,       // Mob detected nearby
    Engaging,    // Actively fighting
    Retreating,  // Running away
    Towering,    // Building up to escape
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreatType {
    Zombie,
    Skeleton,
    Creeper,
    Spider,
    Enderman,
    Witch,
    Player(String),
    Unknown,
}

impl ThreatType {
    /// Priority: higher = more dangerous
    pub fn danger_level(&self) -> u8 {
        match self {
            ThreatType::Creeper => 9,
            ThreatType::Skeleton => 7,
            ThreatType::Witch => 8,
            ThreatType::Player(_) => 10,
            ThreatType::Enderman => 6,
            ThreatType::Zombie => 4,
            ThreatType::Spider => 5,
            ThreatType::Unknown => 3,
        }
    }

    /// Recommended tactic
    pub fn tactic(&self) -> CombatTactic {
        match self {
            ThreatType::Creeper => CombatTactic::SprintHitRetreat, // Hit and run
            ThreatType::Skeleton => CombatTactic::ShieldAndClose,  // Block arrows, close gap
            ThreatType::Zombie => CombatTactic::CriticalHit,       // Easy kill
            ThreatType::Spider => CombatTactic::CriticalHit,
            ThreatType::Enderman => CombatTactic::AvoidEyes,       // Don't look
            ThreatType::Witch => CombatTactic::SprintHitRetreat,   // Dodge potions
            ThreatType::Player(_) => CombatTactic::PvP,            // Full PvP mode
            ThreatType::Unknown => CombatTactic::CriticalHit,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CombatTactic {
    CriticalHit,       // Jump + hit for 1.5x damage
    SprintHitRetreat,  // Sprint attack then back off
    ShieldAndClose,    // Hold shield, approach, then attack
    AvoidEyes,         // Don't look at it, back away
    PvP,               // Shield, strafe, w-tap, crit
    Flee,              // Just run
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatSystem {
    pub state: CombatState,
    pub current_threats: Vec<ThreatInfo>,
    pub kills: u32,
    pub deaths: u32,
    pub kd_ratio: f32,
    pub flee_hp_threshold: f32,     // HP below which we run
    pub engage_hp_threshold: f32,   // HP above which we fight
    pub has_shield: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatInfo {
    pub threat_type: ThreatType,
    pub distance: f64,
    pub entity_id: u32,
}

impl Default for CombatSystem {
    fn default() -> Self {
        Self {
            state: CombatState::Peaceful,
            current_threats: vec![],
            kills: 0,
            deaths: 0,
            kd_ratio: 0.0,
            flee_hp_threshold: 6.0,
            engage_hp_threshold: 10.0,
            has_shield: false,
        }
    }
}

impl CombatSystem {
    /// Evaluate threats and decide what to do
    pub fn evaluate(&mut self, hp: f32, food: u32) -> CombatDecision {
        if self.current_threats.is_empty() {
            self.state = CombatState::Peaceful;
            return CombatDecision::DoNothing;
        }

        // Sort by danger
        self.current_threats.sort_by(|a, b| {
            b.threat_type.danger_level().cmp(&a.threat_type.danger_level())
        });

        let top_threat = &self.current_threats[0];

        // Should we flee?
        if hp < self.flee_hp_threshold || (food < 6 && hp < 14.0) {
            self.state = CombatState::Retreating;
            return CombatDecision::Flee;
        }

        // Multiple threats?
        if self.current_threats.len() >= 3 && hp < 14.0 {
            self.state = CombatState::Retreating;
            return CombatDecision::Tower; // Tower up
        }

        // Single creeper close?
        if top_threat.threat_type == ThreatType::Creeper && top_threat.distance < 4.0 {
            self.state = CombatState::Retreating;
            return CombatDecision::Flee;
        }

        // Engage
        self.state = CombatState::Engaging;
        let tactic = top_threat.threat_type.tactic();
        CombatDecision::Fight(tactic, top_threat.entity_id)
    }

    pub fn record_kill(&mut self) {
        self.kills += 1;
        self.update_kd();
    }

    pub fn record_death(&mut self) {
        self.deaths += 1;
        self.update_kd();
    }

    fn update_kd(&mut self) {
        self.kd_ratio = if self.deaths > 0 {
            self.kills as f32 / self.deaths as f32
        } else {
            self.kills as f32
        };
    }

    pub fn context_summary(&self) -> String {
        format!(
            "Combate: {:?} | K/D: {}/{} ({:.1}) | Ameaças: {}",
            self.state, self.kills, self.deaths, self.kd_ratio, self.current_threats.len()
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CombatDecision {
    DoNothing,
    Fight(CombatTactic, u32), // tactic + target entity_id
    Flee,
    Tower,
}
