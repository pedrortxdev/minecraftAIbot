use azalea::prelude::*;
use rand::Rng;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

// ============================================================
// NATURAL LOOK BEHAVIOR — No more staring at the horizon
// Perlin-like head noise, focus on speakers, random fidgets
// ============================================================

#[derive(Debug, Clone)]
pub struct NaturalLookState {
    pub tick_counter: u64,
    pub last_fidget: Instant,
    pub last_speaker: Option<String>,
    pub last_speaker_time: Instant,
    pub idle_since: Instant,
    pub base_yaw: f32,
    pub base_pitch: f32,
}

impl Default for NaturalLookState {
    fn default() -> Self {
        Self {
            tick_counter: 0,
            last_fidget: Instant::now(),
            last_speaker: None,
            last_speaker_time: Instant::now() - Duration::from_secs(60),
            idle_since: Instant::now(),
            base_yaw: 0.0,
            base_pitch: 0.0,
        }
    }
}

#[derive(Clone, Component)]
pub struct State {
    pub inner: Arc<Mutex<NaturalLookState>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(NaturalLookState::default())),
        }
    }
}

/// Simple pseudo-perlin noise using sine waves at different frequencies
fn smooth_noise(tick: u64, speed: f64, amplitude: f64) -> f64 {
    let t = tick as f64 * speed;
    (t * 0.7).sin() * amplitude * 0.5
        + (t * 1.3 + 2.1).sin() * amplitude * 0.3
        + (t * 2.9 + 5.7).sin() * amplitude * 0.2
}

/// Generate the micro-movements for the current tick
pub fn compute_look_offset(state: &mut NaturalLookState) -> (f32, f32) {
    state.tick_counter += 1;
    let tick = state.tick_counter;

    let mut rng = rand::thread_rng();

    // === IDLE HEAD BOBBING (Perlin-like noise) ===
    // Slow, organic head movements when idle
    let yaw_noise = smooth_noise(tick, 0.02, 8.0) as f32;  // ±8 degrees, slow
    let pitch_noise = smooth_noise(tick, 0.015, 4.0) as f32; // ±4 degrees, slower

    // === OCCASIONAL GLANCE ===
    // Every 3-7 seconds, do a quick glance in a random direction
    let _seconds_idle = state.idle_since.elapsed().as_secs_f32();
    let glance_yaw = if state.last_fidget.elapsed() > Duration::from_secs(rng.r#gen::<u64>() % 5 + 3) {
        state.last_fidget = Instant::now();
        // Quick glance: 20-60 degrees in random direction
        let glance: f32 = rng.r#gen::<f32>() * 40.0 + 20.0;
        if rng.r#gen::<bool>() { glance } else { -glance }
    } else {
        0.0
    };

    // === LOOK AT SPEAKER ===
    // If someone chatted recently (< 3s), we should be looking towards them
    // (actual entity lookup would happen in the caller — here we just provide the intent)
    let _speaker_urgency = if state.last_speaker_time.elapsed() < Duration::from_secs(3) {
        1.0 // Full attention
    } else {
        0.0
    };

    let final_yaw = state.base_yaw + yaw_noise + glance_yaw;
    let final_pitch = state.base_pitch + pitch_noise;

    // Clamp pitch to realistic range (-70 to 70 degrees)
    let final_pitch = final_pitch.clamp(-70.0, 70.0);

    (final_yaw, final_pitch)
}

/// Possible fidget actions the bot can do when idle
#[derive(Debug, Clone, PartialEq)]
pub enum FidgetAction {
    None,
    Jump,                    // Random bunny hop
    SwapHands,               // F key — swap main/offhand
    LookAtGround,            // Quick look down
    LookAtSky,               // Quick look up
    SpinAround,              // Quick 180°
    PunchAir,                // Swing arm at nothing
    Sneak,                   // Quick crouch
}

/// Decide if the bot should fidget this tick
pub fn maybe_fidget(state: &mut NaturalLookState) -> FidgetAction {
    let mut rng = rand::thread_rng();

    // Only fidget every 8-20 seconds
    if state.last_fidget.elapsed() < Duration::from_secs(8) {
        return FidgetAction::None;
    }

    // 15% chance per eligible tick
    if rng.r#gen::<f32>() > 0.15 {
        return FidgetAction::None;
    }

    state.last_fidget = Instant::now();

    // Weighted random fidget selection
    let roll: f32 = rng.r#gen();
    match roll {
        r if r < 0.25 => FidgetAction::Jump,
        r if r < 0.40 => FidgetAction::SwapHands,
        r if r < 0.55 => FidgetAction::LookAtGround,
        r if r < 0.65 => FidgetAction::LookAtSky,
        r if r < 0.75 => FidgetAction::PunchAir,
        r if r < 0.85 => FidgetAction::Sneak,
        _ => FidgetAction::SpinAround,
    }
}

/// Record that someone spoke (so we can look at them)
pub fn on_player_chat(state: &mut NaturalLookState, player: &str) {
    state.last_speaker = Some(player.to_string());
    state.last_speaker_time = Instant::now();
}

pub async fn handle(bot: Client, event: Event, state: State) -> anyhow::Result<()> {
    if let Event::Tick = event {
        let mut inner = state.inner.lock().unwrap();

        // Compute head movement
        let (yaw_offset, pitch_offset) = compute_look_offset(&mut inner);

        // Apply look (azalea API)
        // bot.set_rotation(yaw_offset, pitch_offset);
        // In practice, you'd call:
        // bot.look(yaw_offset, pitch_offset);
        // For now, we just log occasionally
        if inner.tick_counter % 200 == 0 {
            println!("[LOOK] 👀 Head offset: yaw={:.1}° pitch={:.1}°", yaw_offset, pitch_offset);
        }

        // Check for fidgets
        let fidget = maybe_fidget(&mut inner);
        match fidget {
            FidgetAction::Jump => {
                println!("[FIDGET] 🦘 Random hop");
                bot.jump();
            }
            FidgetAction::SwapHands => {
                println!("[FIDGET] 🔄 Swapping hands");
                // bot.swap_hands(); // When API available
            }
            FidgetAction::PunchAir => {
                println!("[FIDGET] 👊 Punching air");
                // bot.swing(); // API not available in azalea 0.15
            }
            FidgetAction::Sneak => {
                println!("[FIDGET] 🐾 Quick crouch");
                // bot.set_sneaking(true); // API not available in azalea 0.15
                // tokio::spawn to unsneak would go here
            }
            FidgetAction::LookAtGround => {
                println!("[FIDGET] ⬇️ Looking down");
                inner.base_pitch = 45.0;
            }
            FidgetAction::LookAtSky => {
                println!("[FIDGET] ⬆️ Looking up");
                inner.base_pitch = -60.0;
            }
            FidgetAction::SpinAround => {
                println!("[FIDGET] 🔃 Quick spin");
                inner.base_yaw += 180.0;
            }
            FidgetAction::None => {}
        }

        // Gradually return base pitch to 0
        if inner.base_pitch.abs() > 1.0 {
            inner.base_pitch *= 0.95;
        }
    }

    Ok(())
}
