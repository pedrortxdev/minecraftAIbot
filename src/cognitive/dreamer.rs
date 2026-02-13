use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rand::Rng;
use crate::cognitive::personality::Mood;
use crate::cognitive::memory::Memory;
use crate::cognitive::goal_planner::{Goal, GoalPriority, GoalPlanner};

// ============================================================
// DREAMER — Spontaneous goal generation from boredom
// "Vi uma montanha legal, vou fazer uma base secreta lá"
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dream {
    pub idea: String,
    pub motivation: String,
    pub generated_at: DateTime<Utc>,
    pub mood_when_dreamed: String,
    pub priority: GoalPriority,
}

#[derive(Debug, Clone)]
pub struct DreamerState {
    pub idle_ticks: u64,
    pub last_dream_time: DateTime<Utc>,
    pub dreams_generated: u32,
    pub boredom_threshold: u64, // Ticks before boredom kicks in (~2 min = 2400 ticks)
}

impl Default for DreamerState {
    fn default() -> Self {
        Self {
            idle_ticks: 0,
            last_dream_time: Utc::now() - chrono::Duration::minutes(10),
            dreams_generated: 0,
            boredom_threshold: 2400, // ~2 minutes at 20 TPS
        }
    }
}

impl DreamerState {
    /// Record a tick of idleness
    pub fn tick_idle(&mut self) {
        self.idle_ticks += 1;
    }

    /// Reset idle counter (bot is busy)
    pub fn reset_idle(&mut self) {
        self.idle_ticks = 0;
    }

    /// Is the bot bored enough to dream?
    pub fn is_bored(&self) -> bool {
        self.idle_ticks >= self.boredom_threshold
    }

    /// Can we dream? (cooldown: 5 min between dreams)
    pub fn can_dream(&self) -> bool {
        Utc::now().signed_duration_since(self.last_dream_time).num_seconds() > 300
    }
}

/// Dream templates based on mood and memory
struct DreamTemplate {
    idea: &'static str,
    motivation: &'static str,
    priority: GoalPriority,
    required_mood: Option<Mood>,
}

const DREAM_TEMPLATES: &[DreamTemplate] = &[
    // Creative dreams
    DreamTemplate {
        idea: "Construir uma torre de vigia no ponto mais alto",
        motivation: "to de saco cheio, bora subir aquela montanha e fazer algo massa",
        priority: GoalPriority::Low,
        required_mood: None,
    },
    DreamTemplate {
        idea: "Fazer uma base secreta subterrânea",
        motivation: "ninguem pode saber onde eu guardo meus diamantes",
        priority: GoalPriority::Low,
        required_mood: Some(Mood::Suspicious),
    },
    DreamTemplate {
        idea: "Construir uma pixel art gigante",
        motivation: "preciso deixar minha marca nesse server",
        priority: GoalPriority::Background,
        required_mood: Some(Mood::Hyped),
    },
    DreamTemplate {
        idea: "Terraformar uma montanha",
        motivation: "aquela montanha ficaria insana se eu desse uma arrumada",
        priority: GoalPriority::Background,
        required_mood: None,
    },
    // Technical dreams
    DreamTemplate {
        idea: "Criar uma iron farm automática",
        motivation: "to cansado de minerar ferro manualmente",
        priority: GoalPriority::Medium,
        required_mood: Some(Mood::Focused),
    },
    DreamTemplate {
        idea: "Fazer um sugarcane farm com hopper",
        motivation: "preciso de muito papel pra encantamento",
        priority: GoalPriority::Medium,
        required_mood: None,
    },
    DreamTemplate {
        idea: "Construir um mob grinder",
        motivation: "xp grátis, quem não quer?",
        priority: GoalPriority::Medium,
        required_mood: None,
    },
    DreamTemplate {
        idea: "Melhorar o sistema de redstone da base",
        motivation: "aquele circuito tá muito gambiarra, preciso refazer",
        priority: GoalPriority::Low,
        required_mood: Some(Mood::Focused),
    },
    // Exploration dreams
    DreamTemplate {
        idea: "Explorar a caverna que achei ontem",
        motivation: "aposto que tem spawner la dentro",
        priority: GoalPriority::Low,
        required_mood: None,
    },
    DreamTemplate {
        idea: "Ir pro Nether achar uma fortaleza",
        motivation: "preciso de blaze rods pra poção",
        priority: GoalPriority::Medium,
        required_mood: Some(Mood::Chill),
    },
    DreamTemplate {
        idea: "Mapear a região toda",
        motivation: "quero saber tudo que tem por aqui",
        priority: GoalPriority::Background,
        required_mood: None,
    },
    // Social dreams
    DreamTemplate {
        idea: "Fazer uma arena PvP pro server",
        motivation: "falta um lugar decente pra lutar aqui",
        priority: GoalPriority::Background,
        required_mood: Some(Mood::Generous),
    },
    DreamTemplate {
        idea: "Criar uma loja de trocas",
        motivation: "vou virar o comerciante oficial do server",
        priority: GoalPriority::Background,
        required_mood: Some(Mood::Chill),
    },
    // Revenge/defense dreams
    DreamTemplate {
        idea: "Construir armadilhas ao redor da base",
        motivation: "nunca mais vão grifar minha casa",
        priority: GoalPriority::Medium,
        required_mood: Some(Mood::Annoyed),
    },
    DreamTemplate {
        idea: "Montar um bunker com obsidian",
        motivation: "sem TNT vai passar por essa parede",
        priority: GoalPriority::Low,
        required_mood: Some(Mood::Scared),
    },
];

/// Generate a spontaneous dream/goal
pub fn dream(mood: &Mood, memory: &Memory) -> Option<Dream> {
    let mut rng = rand::thread_rng();

    // Filter templates by mood compatibility
    let compatible: Vec<&DreamTemplate> = DREAM_TEMPLATES.iter()
        .filter(|t| {
            t.required_mood.as_ref().map_or(true, |m| m == mood)
        })
        .collect();

    if compatible.is_empty() {
        return None;
    }

    let template = compatible[rng.r#gen::<usize>() % compatible.len()];

    // Context enrichment: if we remember a cool location, reference it
    let motivation = if !memory.spatial.locations.is_empty() && rng.r#gen::<f32>() < 0.3 {
        let loc = &memory.spatial.locations[rng.r#gen::<usize>() % memory.spatial.locations.len()];
        format!("{} (lembrei de {} em [{},{},{}])",
            template.motivation, loc.name, loc.coords[0], loc.coords[1], loc.coords[2])
    } else {
        template.motivation.to_string()
    };

    Some(Dream {
        idea: template.idea.to_string(),
        motivation,
        generated_at: Utc::now(),
        mood_when_dreamed: format!("{:?}", mood),
        priority: template.priority.clone(),
    })
}

/// Convert a dream into a real goal and inject it into the planner
pub fn realize_dream(dream: &Dream, planner: &mut GoalPlanner) {
    let goal = Goal::new(&dream.idea, &dream.motivation, dream.priority.clone());
    println!("[DREAMER] 💭 \"{}\" — {}", dream.idea, dream.motivation);
    planner.add_goal(goal);
}

/// The main dreaming cycle
pub fn maybe_dream(
    state: &mut DreamerState,
    mood: &Mood,
    memory: &Memory,
    planner: &mut GoalPlanner,
) -> Option<String> {
    if !state.is_bored() || !state.can_dream() {
        return None;
    }

    if let Some(d) = dream(mood, memory) {
        state.last_dream_time = Utc::now();
        state.dreams_generated += 1;
        state.idle_ticks = 0; // Reset boredom

        let chat_msg = format!("hmm sabe oq, {}", d.motivation);
        realize_dream(&d, planner);

        Some(chat_msg)
    } else {
        None
    }
}
