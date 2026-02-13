use crate::plugins;
use crate::cognitive;
use crate::systems;

use azalea::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Component)]
pub struct State {
    pub anti_afk: plugins::anti_afk::State,
    pub brain: plugins::brain::State,
    pub ping: plugins::ping::State,
    pub natural_look: systems::natural_look::State,
    pub inventory_mgr: systems::inventory_manager::State,
    pub reaction: systems::reaction_delay::State,
    // === NEW SYSTEMS ===
    pub motor: systems::motor::MotorState,
    pub visual_cortex: Arc<Mutex<systems::visual_cortex::VisualCortexState>>,
    pub spider_sense: Arc<Mutex<systems::spider_sense::SpiderSense>>,
    pub dreamer: Arc<Mutex<cognitive::dreamer::DreamerState>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            anti_afk: plugins::anti_afk::State {
                last_action: Arc::new(Mutex::new(Instant::now())),
            },
            brain: plugins::brain::State::default(),
            ping: plugins::ping::State::default(),
            natural_look: systems::natural_look::State::default(),
            inventory_mgr: systems::inventory_manager::State::default(),
            reaction: systems::reaction_delay::State::default(),
            // === NEW SYSTEMS ===
            motor: systems::motor::MotorState::default(),
            visual_cortex: Arc::new(Mutex::new(systems::visual_cortex::VisualCortexState::default())),
            spider_sense: Arc::new(Mutex::new(systems::spider_sense::SpiderSense::default())),
            dreamer: Arc::new(Mutex::new(cognitive::dreamer::DreamerState::default())),
        }
    }
}

pub async fn handle(bot: Client, event: Event, state: State) -> anyhow::Result<()> {
    match &event {
        Event::Login => {
            println!("[BOT] ✅ Joined the server!");
            let mut memory = state.brain.memory.lock().unwrap();
            memory.episodes.add(cognitive::memory::Episode {
                timestamp: chrono::Utc::now(),
                event_type: cognitive::memory::EpisodeType::ServerJoin,
                description: "Entrei no servidor".into(),
                location: None,
                players_involved: vec![],
                emotional_impact: 1,
            });
        }
        Event::Chat(chat) => {
            let msg = chat.message().to_string();
            println!("[CHAT] {}", msg);

            // Tell NaturalLook who spoke (so we look at them)
            if let Some((sender, _)) = plugins::brain::extract_sender_pub(&msg) {
                let mut look = state.natural_look.inner.lock().unwrap();
                systems::natural_look::on_player_chat(&mut look, sender);
            }

            // Brain handles the rest
            let _ = plugins::brain::handle(bot.clone(), event.clone(), state.brain.clone()).await;
        }
        Event::Disconnect(reason) => {
            println!("[DISCONNECT] Bot kicked/disconnected!");
            if let Some(r) = reason {
                println!("[DISCONNECT] Reason: {}", r.to_string());
            } else {
                println!("[DISCONNECT] No reason provided.");
            }
            let memory = state.brain.memory.lock().unwrap();
            memory.save();
            println!("[BOT] 💾 Memory saved on disconnect.");
        }
        _ => {}
    }

    // Tick-based systems
    if let Event::Tick = &event {
        // === EXISTING SYSTEMS ===
        plugins::auto_eat::handle(bot.clone(), event.clone(), ()).await?;
        plugins::anti_afk::handle(bot.clone(), event.clone(), state.anti_afk.clone()).await?;
        plugins::ping::handle(bot.clone(), event.clone(), state.ping.clone()).await?;
        // Brain tick (personality decay)
        let _ = plugins::brain::handle(bot.clone(), event.clone(), state.brain.clone()).await;
        // Natural look behavior (head bobbing, fidgets)
        let _ = systems::natural_look::handle(bot.clone(), event.clone(), state.natural_look.clone()).await;
        // Inventory management (hotbar sorting)
        let _ = systems::inventory_manager::handle(bot.clone(), event.clone(), state.inventory_mgr.clone()).await;
        // Reaction delay (humanized damage response)
        let _ = systems::reaction_delay::handle(bot.clone(), event.clone(), state.reaction.clone()).await;

        // === [6] SPIDER SENSE — Threat prediction ===
        {
            let world = state.brain.world.lock().unwrap();
            let memory = state.brain.memory.lock().unwrap();
            let spider = state.spider_sense.lock().unwrap();
            let mut motor = state.motor.inner.lock().unwrap();

            // Check each nearby player for threats
            for player_name in &world.nearby_players {
                let trust = memory.social.players.get(player_name)
                    .map(|p| p.trust_level)
                    .unwrap_or(20);

                // Simplified: we don't have held_item info from Azalea yet,
                // but the prediction engine is ready for when we do
                if let Some(threat) = spider.predict_player_threat(
                    player_name,
                    "unknown",  // held_item — will be real once we read entity data
                    15.0,       // distance — placeholder
                    true,       // approaching — conservative
                    trust,
                    &memory.social,
                ) {
                    println!("[SPIDER] 🕷️ {:?}: {} → {:?}",
                        threat.level, threat.description, threat.recommended_action);

                    // Translate threat actions into motor commands
                    match &threat.recommended_action {
                        systems::spider_sense::PredictedAction::Sprint => {
                            motor.queue_urgent(systems::motor::MotorCommand::StartSprint {
                                duration_ticks: 40
                            });
                        }
                        systems::spider_sense::PredictedAction::AttackFirst => {
                            motor.queue_urgent(systems::motor::MotorCommand::Log(
                                format!("ATTACK: {}", player_name)
                            ));
                        }
                        systems::spider_sense::PredictedAction::WarnChat(msg) => {
                            motor.queue(systems::motor::MotorCommand::Chat(msg.clone()));
                        }
                        systems::spider_sense::PredictedAction::PlaceTorch => {
                            motor.queue_urgent(systems::motor::MotorCommand::Log(
                                "PLACE TORCH (anti-gravel)".into()
                            ));
                        }
                        systems::spider_sense::PredictedAction::EatNow => {
                            motor.queue_urgent(systems::motor::MotorCommand::Log(
                                "EAT NOW (starvation risk)".into()
                            ));
                        }
                        _ => {}
                    }
                }
            }

            // Starvation check (placeholder values until we read real player data)
            if let Some(threat) = spider.predict_starvation(20, 20.0, true) {
                if threat.level == systems::spider_sense::ThreatLevel::Critical
                    || threat.level == systems::spider_sense::ThreatLevel::High
                {
                    motor.queue_urgent(systems::motor::MotorCommand::Log(
                        format!("STARVATION: {}", threat.description)
                    ));
                }
            }

            // Update motor's nearby_players flag for social fidgets
            motor.nearby_players = !world.nearby_players.is_empty();
        }

        // === [7] VISUAL CORTEX — Periodic area scan + Gemini judging ===
        {
            let pos = {
                let world = state.brain.world.lock().unwrap();
                world.current_position
            };

            let should_scan = {
                let mut vc = state.visual_cortex.lock().unwrap();
                vc.should_scan(pos)
            };

            if should_scan {
                println!("[VISUAL] 👁️ Scanning area around [{}, {}, {}]...", pos[0], pos[1], pos[2]);

                // Build a basic scan from the world state
                // In production, read actual blocks via bot.world().read()
                let scan = systems::visual_cortex::BlockScan {
                    block_counts: std::collections::HashMap::new(),
                    total_blocks: 0,
                    air_percentage: 100.0,
                    light_avg: 15.0,
                    unique_types: 0,
                    center: pos,
                };

                let summary = scan.to_summary();
                if summary != "Área vazia, só ar." {
                    let motor_state = state.motor.clone();
                    tokio::spawn(async move {
                        if let Some(judgment) = systems::visual_cortex::judge_with_gemini(&scan).await {
                            let mut motor = motor_state.inner.lock().unwrap();
                            motor.queue(systems::motor::MotorCommand::Chat(judgment));
                        }
                    });
                }
            }
        }

        // === [8] DREAMER — Metacognition / Boredom → Spontaneous goals ===
        {
            let has_active_goal = {
                let planner = state.brain.goals.lock().unwrap();
                planner.current_goal().is_some()
            };

            let should_dream = {
                let mut dreamer = state.dreamer.lock().unwrap();
                if has_active_goal {
                    dreamer.reset_idle();
                } else {
                    dreamer.tick_idle();
                }
                dreamer.is_bored() && dreamer.can_dream()
            };

            if should_dream {
                let mood = {
                    let p = state.brain.personality.lock().unwrap();
                    p.mood.clone()
                };
                let memory = state.brain.memory.lock().unwrap();
                let mut planner = state.brain.goals.lock().unwrap();
                let mut dreamer = state.dreamer.lock().unwrap();

                if let Some(chat_msg) = cognitive::dreamer::maybe_dream(
                    &mut dreamer,
                    &mood,
                    &memory,
                    &mut planner,
                ) {
                    let mut motor = state.motor.inner.lock().unwrap();
                    motor.queue(systems::motor::MotorCommand::Chat(chat_msg));
                }
            }
        }

        // === [8.5] UPDATE BOT POSITION for motor ===
        {
            let pos = bot.position();
            let mut motor = state.motor.inner.lock().unwrap();
            motor.bot_position = [pos.x, pos.y, pos.z];
        }

        // === [8.6] AUTONOMOUS WANDERING — If idle too long, explore! ===
        {
            let should_wander = {
                let motor = state.motor.inner.lock().unwrap();
                let planner = state.brain.goals.lock().unwrap();
                let idle_secs = motor.last_movement_time.elapsed().as_secs();

                // Wander if: idle >60s, not already walking, no active goals, queue empty
                idle_secs > 60
                    && !motor.is_walking
                    && planner.current_goal().is_none()
                    && motor.queue_len() == 0
            };

            if should_wander {
                let mut motor = state.motor.inner.lock().unwrap();
                motor.queue(systems::motor::MotorCommand::WanderRandom);
                println!("[BOT] 🦶 Idle too long, time to explore!");
            }
        }

        // === [9] MOTOR — Execute queued commands + human fidgets ===
        let _ = systems::motor::handle(bot.clone(), event.clone(), state.motor.clone()).await;
    }

    Ok(())
}
