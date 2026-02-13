use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
// use std::collections::VecDeque;

// ============================================================
// GOAL PLANNER — Hierarchical goals with priorities
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GoalPriority {
    Critical = 0,  // Survive — eat, heal, escape
    High = 1,      // Establish — shelter, basic tools
    Medium = 2,    // Resource — mine, gather, farm
    Low = 3,       // Build — structures, farms, storage
    Background = 4, // Optimize — enchant, auto-farms, trade
    Social = 5,    // Socialize — chat, help, collaborate
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalStatus {
    Pending,
    Active,
    Paused,      // Interrupted by higher priority
    Completed,
    Failed,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub name: String,
    pub description: String,
    pub priority: GoalPriority,
    pub status: GoalStatus,
    pub created_at: DateTime<Utc>,
    pub deadline: Option<DateTime<Utc>>,
    pub parent_goal: Option<String>,      // For sub-goals
    pub sub_goals: Vec<String>,           // IDs of children
    pub preconditions: Vec<String>,       // Human-readable preconditions
    pub attempts: u32,
    pub max_attempts: u32,
}

impl Goal {
    pub fn new(name: &str, description: &str, priority: GoalPriority) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: description.to_string(),
            priority,
            status: GoalStatus::Pending,
            created_at: Utc::now(),
            deadline: None,
            parent_goal: None,
            sub_goals: vec![],
            preconditions: vec![],
            attempts: 0,
            max_attempts: 5,
        }
    }

    pub fn is_actionable(&self) -> bool {
        self.status == GoalStatus::Pending || self.status == GoalStatus::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalPlanner {
    pub goals: Vec<Goal>,
    pub active_goal: Option<String>, // ID of current goal
    pub completed_count: u32,
    pub failed_count: u32,
}

impl Default for GoalPlanner {
    fn default() -> Self {
        let mut planner = Self {
            goals: vec![],
            active_goal: None,
            completed_count: 0,
            failed_count: 0,
        };
        // Seed with initial survival goals
        planner.seed_initial_goals();
        planner
    }
}

impl GoalPlanner {
    fn seed_initial_goals(&mut self) {
        let goals = vec![
            Goal::new("Sobreviver a Primeira Noite", "Conseguir madeira, craftar ferramentas basicas, fazer abrigo", GoalPriority::Critical),
            Goal::new("Craftar Ferramentas de Pedra", "Picareta, machado, espada de pedra", GoalPriority::High),
            Goal::new("Encontrar Comida", "Matar animais ou achar sementes pra farm", GoalPriority::Critical),
            Goal::new("Estabelecer Base", "Construir uma casa basica com cama, bau, furnace", GoalPriority::High),
            Goal::new("Minerar Ferro", "Descer pra caverna ou strip mine e pegar ferro", GoalPriority::Medium),
            Goal::new("Criar Farm de Trigo", "Plantar pelo menos 9x9 de trigo com agua", GoalPriority::Medium),
            Goal::new("Conseguir Diamante", "Strip mine no Y11 até achar diamante", GoalPriority::Low),
            Goal::new("Encantamento", "Mesa de encantamento + estantes", GoalPriority::Background),
        ];
        self.goals = goals;
    }

    /// Get the highest priority actionable goal
    pub fn current_goal(&self) -> Option<&Goal> {
        if let Some(ref id) = self.active_goal {
            return self.goals.iter().find(|g| &g.id == id && g.is_actionable());
        }
        // Find highest priority pending goal
        self.goals
            .iter()
            .filter(|g| g.is_actionable())
            .min_by_key(|g| g.priority.clone())
    }

    /// Pick the next goal to work on
    pub fn pick_next(&mut self) -> Option<&Goal> {
        // Pause current if any
        if let Some(ref id) = self.active_goal {
            if let Some(g) = self.goals.iter_mut().find(|g| &g.id == id) {
                if g.status == GoalStatus::Active {
                    g.status = GoalStatus::Paused;
                }
            }
        }
        // Find highest priority
        let next_id = self
            .goals
            .iter()
            .filter(|g| g.status == GoalStatus::Pending || g.status == GoalStatus::Paused)
            .min_by_key(|g| g.priority.clone())
            .map(|g| g.id.clone());

        if let Some(ref id) = next_id {
            if let Some(g) = self.goals.iter_mut().find(|g| &g.id == id) {
                g.status = GoalStatus::Active;
                g.attempts += 1;
            }
            self.active_goal = next_id;
        }
        self.current_goal()
    }

    /// Mark current goal as completed
    pub fn complete_current(&mut self) {
        if let Some(ref id) = self.active_goal.take() {
            if let Some(g) = self.goals.iter_mut().find(|g| &g.id == id) {
                g.status = GoalStatus::Completed;
                self.completed_count += 1;
                println!("[GOALS] ✅ Completed: {}", g.name);
            }
        }
    }

    /// Mark current goal as failed
    pub fn fail_current(&mut self) {
        if let Some(ref id) = self.active_goal.clone() {
            if let Some(g) = self.goals.iter_mut().find(|g| &g.id == id) {
                if g.attempts >= g.max_attempts {
                    g.status = GoalStatus::Failed;
                    self.failed_count += 1;
                    println!("[GOALS] ❌ Failed permanently: {}", g.name);
                } else {
                    g.status = GoalStatus::Paused;
                    println!("[GOALS] ⏸ Paused (attempt {}/{}): {}", g.attempts, g.max_attempts, g.name);
                }
            }
        }
        self.active_goal = None;
    }

    /// Add a new dynamic goal (e.g., from AI decision)
    pub fn add_goal(&mut self, goal: Goal) {
        println!("[GOALS] 🆕 New goal: {} ({:?})", goal.name, goal.priority);
        self.goals.push(goal);
    }

    /// Emergency: insert a critical goal that takes over immediately
    pub fn emergency(&mut self, name: &str, description: &str) {
        let mut goal = Goal::new(name, description, GoalPriority::Critical);
        goal.status = GoalStatus::Active;
        goal.max_attempts = 1;
        let id = goal.id.clone();
        self.goals.push(goal);
        // Pause current
        if let Some(ref active_id) = self.active_goal {
            if let Some(g) = self.goals.iter_mut().find(|g| &g.id == active_id) {
                if g.status == GoalStatus::Active {
                    g.status = GoalStatus::Paused;
                }
            }
        }
        self.active_goal = Some(id);
    }

    pub fn context_summary(&self) -> String {
        let mut s = String::new();
        if let Some(g) = self.current_goal() {
            s.push_str(&format!("Objetivo atual: {} — {}\n", g.name, g.description));
        }
        let pending: Vec<_> = self.goals.iter().filter(|g| g.is_actionable()).take(5).collect();
        if !pending.is_empty() {
            s.push_str("Próximos objetivos:\n");
            for g in pending {
                s.push_str(&format!("  - {} ({:?})\n", g.name, g.priority));
            }
        }
        s.push_str(&format!("Completos: {} | Falhados: {}", self.completed_count, self.failed_count));
        s
    }
}
