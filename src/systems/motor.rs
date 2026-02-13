use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use rand::Rng;
use azalea::prelude::*;
use azalea::BlockPos;
use azalea::pathfinder::goals::BlockPosGoal;
use azalea::pathfinder::PathfinderClientExt;

// ============================================================
// MOTOR SYSTEM — Translates intentions into actions
// "O cérebro manda, o corpo executa"
// ============================================================

#[derive(Debug, Clone)]
pub enum MotorCommand {
    /// Chat a message in-game
    Chat(String),
    /// Look at a specific yaw/pitch
    LookAt { yaw: f32, pitch: f32 },
    /// Random head movement (fidget)
    RandomLook,
    /// Jump once
    Jump,
    /// Start sprinting for N ticks
    StartSprint { duration_ticks: u32 },
    /// Sneak toggle (shift) for N ticks
    SneakPulse { duration_ticks: u32 },
    /// Walk toward a direction for N ticks (simplified)
    WalkForward { duration_ticks: u32 },
    /// Emergency: set walk direction to flee
    FleeDirection { yaw: f32 },
    /// Walk to a specific block using azalea pathfinder
    GotoBlock { x: i32, y: i32, z: i32 },
    /// Wander to a random nearby point (autonomous exploration)
    WanderRandom,
    /// Log something to console (for debugging)
    Log(String),
}

#[derive(Debug, Clone)]
struct ActiveAction {
    command: MotorCommand,
    ticks_remaining: u32,
    started_at: Instant,
}

#[derive(Clone)]
pub struct MotorState {
    pub inner: Arc<Mutex<MotorInner>>,
}

impl Default for MotorState {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MotorInner::default())),
        }
    }
}

pub struct MotorInner {
    /// Queue of commands to execute
    pub command_queue: VecDeque<MotorCommand>,
    /// Currently active timed action (sprint, sneak, walk)
    pub active_action: Option<ActiveAction>,
    /// Tick counter for fidgets
    pub tick_counter: u64,
    /// Whether there are nearby players (for social fidgets)
    pub nearby_players: bool,
    /// Total commands executed (stats)
    pub commands_executed: u64,
    /// Is the bot currently sprinting?
    pub is_sprinting: bool,
    /// Is the bot currently sneaking?
    pub is_sneaking: bool,
    /// Is the bot currently walking to a destination?
    pub is_walking: bool,
    /// Last time the bot moved (for idle detection)
    pub last_movement_time: Instant,
    /// Current bot position (updated from world state)
    pub bot_position: [f64; 3],
}

impl Default for MotorInner {
    fn default() -> Self {
        Self {
            command_queue: VecDeque::new(),
            active_action: None,
            tick_counter: 0,
            nearby_players: false,
            commands_executed: 0,
            is_sprinting: false,
            is_sneaking: false,
            is_walking: false,
            last_movement_time: Instant::now(),
            bot_position: [0.0, 64.0, 0.0],
        }
    }
}

impl MotorInner {
    /// Queue a command for execution
    pub fn queue(&mut self, cmd: MotorCommand) {
        self.command_queue.push_back(cmd);
    }

    /// Queue a command at the FRONT (high priority)
    pub fn queue_urgent(&mut self, cmd: MotorCommand) {
        self.command_queue.push_front(cmd);
    }

    /// Clear all queued commands (emergency reset)
    pub fn clear_queue(&mut self) {
        self.command_queue.clear();
        self.active_action = None;
    }

    /// How many commands are waiting?
    pub fn queue_len(&self) -> usize {
        self.command_queue.len()
    }
}

/// Main tick handler — call this every Event::Tick
pub async fn handle(bot: Client, _event: Event, state: MotorState) -> anyhow::Result<()> {
    let mut motor = state.inner.lock().unwrap();
    motor.tick_counter += 1;

    // === 1. HUMAN FIDGETS (random look, shift toggle) ===
    inject_fidgets(&mut motor);

    // === 2. PROCESS ACTIVE TIMED ACTION ===
    if let Some(ref mut action) = motor.active_action {
        action.ticks_remaining = action.ticks_remaining.saturating_sub(1);
        if action.ticks_remaining == 0 {
            // Action finished — clean up
            match &action.command {
                MotorCommand::StartSprint { .. } => {
                    motor.is_sprinting = false;
                    // bot.sprint(SprintDirection::Stop); // Azalea sprint stop
                    println!("[MOTOR] 🏃 Sprint finished");
                }
                MotorCommand::SneakPulse { .. } => {
                    motor.is_sneaking = false;
                    // bot.set_sneaking(false);
                    println!("[MOTOR] 🧎 Sneak pulse finished");
                }
                MotorCommand::WalkForward { .. } => {
                    // bot.walk(WalkDirection::None);
                    println!("[MOTOR] 🚶 Walk finished");
                }
                _ => {}
            }
            motor.active_action = None;
        } else {
            // Action still running, skip processing new commands
            return Ok(());
        }
    }

    // === 3. DEQUEUE AND EXECUTE NEXT COMMAND ===
    if let Some(cmd) = motor.command_queue.pop_front() {
        motor.commands_executed += 1;

        match cmd {
            MotorCommand::Chat(ref msg) => {
                println!("[MOTOR] 💬 Sending chat: {}", msg);
                bot.chat(msg);
            }
            MotorCommand::LookAt { yaw, pitch } => {
                // Clamp pitch to valid range
                let pitch = pitch.clamp(-90.0, 90.0);
                // bot.set_rotation(yaw, pitch); // Azalea rotation
                println!("[MOTOR] 👀 Looking at yaw:{:.1} pitch:{:.1}", yaw, pitch);
            }
            MotorCommand::RandomLook => {
                let mut rng = rand::thread_rng();
                let _yaw_delta: f32 = rng.gen_range(-60.0..60.0);
                let _pitch_delta: f32 = rng.gen_range(-20.0..20.0);
                // bot.set_rotation(current_yaw + yaw_delta, current_pitch + pitch_delta);
                println!("[MOTOR] 🔄 Random look: yaw±{:.0}° pitch±{:.0}°", _yaw_delta, _pitch_delta);
            }
            MotorCommand::Jump => {
                bot.jump();
                println!("[MOTOR] ⬆️ Jump");
            }
            MotorCommand::StartSprint { duration_ticks } => {
                motor.is_sprinting = true;
                // bot.sprint(SprintDirection::Forward);
                motor.active_action = Some(ActiveAction {
                    command: cmd,
                    ticks_remaining: duration_ticks,
                    started_at: Instant::now(),
                });
                println!("[MOTOR] 🏃 Sprint started ({} ticks)", duration_ticks);
            }
            MotorCommand::SneakPulse { duration_ticks } => {
                motor.is_sneaking = true;
                // bot.set_sneaking(true);
                motor.active_action = Some(ActiveAction {
                    command: cmd,
                    ticks_remaining: duration_ticks,
                    started_at: Instant::now(),
                });
                println!("[MOTOR] 🧎 Sneak pulse ({} ticks)", duration_ticks);
            }
            MotorCommand::WalkForward { duration_ticks } => {
                // bot.walk(WalkDirection::Forward);
                motor.active_action = Some(ActiveAction {
                    command: cmd,
                    ticks_remaining: duration_ticks,
                    started_at: Instant::now(),
                });
                println!("[MOTOR] 🚶 Walk forward ({} ticks)", duration_ticks);
            }
            MotorCommand::FleeDirection { yaw } => {
                // bot.set_rotation(yaw, 0.0);
                motor.is_sprinting = true;
                motor.active_action = Some(ActiveAction {
                    command: MotorCommand::StartSprint { duration_ticks: 40 },
                    ticks_remaining: 40,
                    started_at: Instant::now(),
                });
                println!("[MOTOR] 🏃💨 FLEE! yaw:{:.1}", yaw);
            }
            MotorCommand::GotoBlock { x, y, z } => {
                println!("[MOTOR] 🚶 Goto ({}, {}, {})", x, y, z);
                motor.is_walking = true;
                motor.last_movement_time = Instant::now();
                let target = BlockPosGoal(BlockPos::new(x, y, z));
                // Drop the lock before calling start_goto (it's non-blocking)
                drop(motor);
                bot.start_goto(target);
                return Ok(());
            }
            MotorCommand::WanderRandom => {
                let mut rng = rand::thread_rng();
                let pos = motor.bot_position;
                let dx: i32 = rng.gen_range(-25..25);
                let dz: i32 = rng.gen_range(-25..25);
                let target_x = pos[0] as i32 + dx;
                let target_y = pos[1] as i32;
                let target_z = pos[2] as i32 + dz;
                println!("[MOTOR] 🌍 Wander to ({}, {}, {})", target_x, target_y, target_z);
                motor.is_walking = true;
                motor.last_movement_time = Instant::now();
                let target = BlockPosGoal(BlockPos::new(target_x, target_y, target_z));
                drop(motor);
                bot.start_goto(target);
                return Ok(());
            }
            MotorCommand::Log(ref msg) => {
                println!("[MOTOR] 📋 {}", msg);
            }
        }
    }

    Ok(())
}

/// Inject natural human fidgets into the command queue
fn inject_fidgets(motor: &mut MotorInner) {
    let mut rng = rand::thread_rng();

    // Don't fidget if we're busy executing something or queue is full
    if motor.active_action.is_some() || motor.queue_len() > 5 {
        return;
    }

    // 1% chance per tick (~once per 5 seconds) — random head look
    if rng.gen_bool(0.01) {
        motor.queue(MotorCommand::RandomLook);
    }

    // 5% chance per tick of sneak pulse IF players are nearby
    if motor.nearby_players && rng.gen_bool(0.002) {
        motor.queue(MotorCommand::SneakPulse { duration_ticks: 4 }); // ~200ms
    }

    // 0.1% chance per tick — random jump (very rare fidget)
    if rng.gen_bool(0.001) {
        motor.queue(MotorCommand::Jump);
    }
}
