use azalea::prelude::*;
use azalea::SprintDirection;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

// ============================================================
// REACTION DELAY — Humanized damage response
// No aimbot 180°! Real players panic first, THEN fight.
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum ReactionPhase {
    Calm,           // No threats
    Panicking,      // Just got hit, running + looking around
    Assessing,      // Figuring out where the hit came from
    Responding,     // Now fighting back or fleeing
}

#[derive(Debug, Clone)]
pub struct ReactionState {
    pub phase: ReactionPhase,
    pub damage_time: Instant,         // When we got hit
    pub panic_duration_ms: u64,      // How long to panic (200-400ms)
    pub assess_duration_ms: u64,     // How long to assess (100-200ms)
    pub total_damage_taken: f32,
    pub hits_in_last_5s: u32,
    pub last_damage_direction: Option<f32>, // Yaw of attacker
}

impl Default for ReactionState {
    fn default() -> Self {
        Self {
            phase: ReactionPhase::Calm,
            damage_time: Instant::now() - Duration::from_secs(60),
            panic_duration_ms: 300,
            assess_duration_ms: 150,
            total_damage_taken: 0.0,
            hits_in_last_5s: 0,
            last_damage_direction: None,
        }
    }
}

#[derive(Clone, Component)]
pub struct State {
    pub inner: Arc<Mutex<ReactionState>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ReactionState::default())),
        }
    }
}

/// Called when the bot takes damage
pub fn on_damage(state: &mut ReactionState, damage_amount: f32, attacker_yaw: Option<f32>) {
    let mut rng = rand::thread_rng();

    state.phase = ReactionPhase::Panicking;
    state.damage_time = Instant::now();
    state.total_damage_taken += damage_amount;
    state.hits_in_last_5s += 1;
    state.last_damage_direction = attacker_yaw;

    // Randomize reaction time (200ms - 400ms for first hit)
    // Gets faster with repeated hits (muscle memory)
    let base_panic = if state.hits_in_last_5s > 3 {
        100 // Experienced at being hit, faster reaction
    } else {
        rng.r#gen::<u64>() % 200 + 200 // 200-400ms first time
    };

    state.panic_duration_ms = base_panic;
    state.assess_duration_ms = rng.r#gen::<u64>() % 100 + 100; // 100-200ms

    println!(
        "[REACTION] 😰 Hit! Damage: {:.1} | Panic: {}ms | Assess: {}ms",
        damage_amount, state.panic_duration_ms, state.assess_duration_ms
    );
}

/// What should the bot do RIGHT NOW based on reaction phase?
#[derive(Debug, Clone, PartialEq)]
pub enum ReactionAction {
    Nothing,            // Calm, no threats
    JumpAndRun,         // Panic: jump forward + sprint
    LookAround,         // Assess: spin around looking for threat
    FightOrFlight,      // Respond: actually engage
    Sprint,             // Quick sprint away
}

/// Get the current action based on reaction phase timing
pub fn get_reaction_action(state: &mut ReactionState) -> ReactionAction {
    let elapsed = state.damage_time.elapsed();

    match state.phase {
        ReactionPhase::Calm => ReactionAction::Nothing,

        ReactionPhase::Panicking => {
            if elapsed < Duration::from_millis(state.panic_duration_ms) {
                // Still panicking — jump and run
                ReactionAction::JumpAndRun
            } else {
                // Panic over, start assessing
                state.phase = ReactionPhase::Assessing;
                ReactionAction::LookAround
            }
        }

        ReactionPhase::Assessing => {
            let total = state.panic_duration_ms + state.assess_duration_ms;
            if elapsed < Duration::from_millis(total) {
                // Still looking around
                ReactionAction::LookAround
            } else {
                // Done assessing, now respond
                state.phase = ReactionPhase::Responding;
                ReactionAction::FightOrFlight
            }
        }

        ReactionPhase::Responding => {
            // After 2 seconds, return to calm
            if elapsed > Duration::from_secs(2) {
                state.phase = ReactionPhase::Calm;
                state.hits_in_last_5s = 0;
                ReactionAction::Nothing
            } else {
                ReactionAction::FightOrFlight
            }
        }
    }
}

/// Get a semi-random look direction during the assessment phase
/// (spinning around looking for the threat instead of instant aimbot)
pub fn get_panic_look_direction(state: &ReactionState) -> (f32, f32) {
    let mut rng = rand::thread_rng();

    let elapsed_ms = state.damage_time.elapsed().as_millis() as f32;

    match state.phase {
        ReactionPhase::Panicking => {
            // Quick wild look — slightly away from damage direction
            let base_yaw = state.last_damage_direction.unwrap_or(0.0);
            let panic_offset: f32 = rng.r#gen::<f32>() * 90.0 - 45.0;
            (base_yaw + 180.0 + panic_offset, rng.r#gen::<f32>() * 20.0 - 10.0)
        }
        ReactionPhase::Assessing => {
            // Scanning — gradually turning towards damage direction
            let base_yaw = state.last_damage_direction.unwrap_or(0.0);
            let progress = (elapsed_ms - state.panic_duration_ms as f32)
                / state.assess_duration_ms as f32;
            let scan_offset = (1.0 - progress.clamp(0.0, 1.0)) * 60.0;
            (base_yaw + 180.0 + scan_offset, 5.0) // Slowly centering on attacker
        }
        ReactionPhase::Responding => {
            // Now locked on — look at the damage direction
            let base_yaw = state.last_damage_direction.unwrap_or(0.0);
            (base_yaw + 180.0, 0.0) // Face the attacker
        }
        ReactionPhase::Calm => (0.0, 0.0),
    }
}

pub async fn handle(bot: Client, event: Event, state: State) -> anyhow::Result<()> {
    if let Event::Tick = event {
        let mut inner = state.inner.lock().unwrap();
        let action = get_reaction_action(&mut inner);

        match action {
            ReactionAction::JumpAndRun => {
                bot.jump();
                bot.sprint(SprintDirection::Forward);
                // Look direction during panic
                let (_yaw, _pitch) = get_panic_look_direction(&inner);
                // bot.set_rotation(yaw, pitch);
            }
            ReactionAction::LookAround => {
                let (_yaw, _pitch) = get_panic_look_direction(&inner);
                // bot.set_rotation(yaw, pitch);
            }
            ReactionAction::FightOrFlight => {
                // Stop sprinting — just don't call sprint
                // bot.sprint() with no-sprint not available; combat system handles this
                // Now we can fight — actual combat logic in combat.rs
            }
            ReactionAction::Sprint => {
                bot.sprint(SprintDirection::Forward);
            }
            ReactionAction::Nothing => {}
        }
    }
    Ok(())
}
