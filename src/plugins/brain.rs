use azalea::prelude::*;
use serde::{Deserialize, Serialize};
use crate::config::Config;
use crate::cognitive::memory::Memory;
use crate::cognitive::personality::{Personality, PersonalityEvent};
use crate::cognitive::goal_planner::GoalPlanner;
use crate::systems::world_scanner::WorldState;
use crate::systems::social::{SocialEngine, ResponseStyle};
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

// ============================================================
// GEMINI API TYPES
// ============================================================

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct GContent {
    role: String,
    parts: Vec<GPart>,
}

#[derive(Serialize)]
struct GPart {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
    temperature: f32,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: ContentResponse,
}

#[derive(Deserialize)]
struct ContentResponse {
    parts: Vec<PartResponse>,
}

#[derive(Deserialize)]
struct PartResponse {
    text: String,
}

// ============================================================
// BRAIN V2 — The Cortex
// ============================================================

#[derive(Clone, Component)]
pub struct State {
    pub memory: Arc<Mutex<Memory>>,
    pub personality: Arc<Mutex<Personality>>,
    pub goals: Arc<Mutex<GoalPlanner>>,
    pub world: Arc<Mutex<WorldState>>,
    pub social: Arc<Mutex<SocialEngine>>,
    pub last_chat: Arc<Mutex<Instant>>,
    pub chat_history: Arc<Mutex<Vec<String>>>, // Last N chat messages for context
    pub save_counter: Arc<Mutex<u32>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            memory: Arc::new(Mutex::new(Memory::load())),
            personality: Arc::new(Mutex::new(Personality::default())),
            goals: Arc::new(Mutex::new(GoalPlanner::default())),
            world: Arc::new(Mutex::new(WorldState::default())),
            social: Arc::new(Mutex::new(SocialEngine::default())),
            last_chat: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(60))),
            chat_history: Arc::new(Mutex::new(Vec::new())),
            save_counter: Arc::new(Mutex::new(0)),
        }
    }
}

/// Build the full context string for the AI
fn build_context(state: &State, incoming_message: &str, sender: &str) -> String {
    let memory = state.memory.lock().unwrap();
    let personality = state.personality.lock().unwrap();
    let goals = state.goals.lock().unwrap();
    let world = state.world.lock().unwrap();
    let social_engine = state.social.lock().unwrap();
    let chat_history = state.chat_history.lock().unwrap();

    // Get relationship context
    let relationship_ctx = memory.social.players.get(sender).map(|p| {
        format!(
            "Relação com {}: {:?} (confiança: {}). Já se viram {}x. Notas: {:?}",
            sender, p.relationship, p.trust_level, p.times_met, p.notes
        )
    }).unwrap_or_else(|| format!("{} é um desconhecido. Primeira vez que vocês conversam.", sender));

    // Recent chat for context
    let recent_chat = if chat_history.is_empty() {
        "Nenhuma mensagem recente.".into()
    } else {
        chat_history.iter().rev().take(10).cloned().collect::<Vec<_>>().join("\n")
    };

    format!(
r#"{}

=== ESTADO ATUAL ===
{}
{}
{}

=== CONTEXTO SOCIAL ===
{}
{}

=== CHAT RECENTE ===
{}

=== MENSAGEM PRA RESPONDER ===
<{}> {}"#,
        personality.system_prompt(),
        world.context_summary(),
        goals.context_summary(),
        memory.episodes.context_summary(3),
        relationship_ctx,
        social_engine.context_summary(),
        recent_chat,
        sender,
        incoming_message,
    )
}

/// Extract sender name from chat message (format: <PlayerName> message)
fn extract_sender(message: &str) -> Option<(&str, &str)> {
    if let Some(start) = message.find('<') {
        if let Some(end) = message.find('>') {
            let sender = &message[start + 1..end];
            let content = message[end + 1..].trim();
            return Some((sender, content));
        }
    }
    None
}

pub async fn handle(_bot: Client, event: Event, state: State) -> anyhow::Result<()> {
    match event {
        Event::Chat(chat) => {
            let raw_message = chat.message().to_string();

            // Add to chat history
            {
                let mut history = state.chat_history.lock().unwrap();
                history.push(raw_message.clone());
                if history.len() > 20 {
                    history.drain(0..10);
                }
            }

            // Extract sender
            let (sender, content) = match extract_sender(&raw_message) {
                Some(s) => s,
                None => return Ok(()), // System message or unparseable
            };

            // Ignore self
            let config = Config::load();
            if sender == config.bot_name {
                return Ok(());
            }

            // Update social memory
            {
                let mut memory = state.memory.lock().unwrap();
                memory.social.record_interaction(sender, 1); // +1 trust for chatting
                let player = memory.social.get_or_create(sender);
                player.add_message(content);
            }

            // Personality event
            {
                let mut personality = state.personality.lock().unwrap();
                personality.on_event(&PersonalityEvent::ReceivedChat);
            }

            // Decide if we should respond
            let should_respond = {
                let social_engine = state.social.lock().unwrap();
                let memory = state.memory.lock().unwrap();
                let style = social_engine.should_respond(sender, &memory.social);

                // Always respond to direct mentions
                let mentions_us = content.to_lowercase().contains(&config.bot_name.to_lowercase());

                match style {
                    ResponseStyle::Friendly => true,
                    ResponseStyle::Casual => mentions_us || rand::random::<f32>() < 0.6,
                    ResponseStyle::Cautious => mentions_us || rand::random::<f32>() < 0.3,
                    ResponseStyle::Cold => mentions_us,
                    ResponseStyle::Hostile => false,
                }
            };

            // Check triggers (broader than before — responds to more things)
            let triggers = [
                "lag", "tps", "java", "code", "bot", "pedro", "frankfurt",
                "farm", "mine", "build", "help", "ajuda", "diamante",
                "redstone", "encantamento", "casa", "base", "oi", "eai",
                "salve", "fala", "bora", "vem", "cadê", "morri",
            ];
            let has_trigger = triggers.iter().any(|&t| content.to_lowercase().contains(t));
            let mentions_us = content.to_lowercase().contains(&config.bot_name.to_lowercase());

            if !should_respond && !has_trigger && !mentions_us {
                return Ok(());
            }

            // Rate limit
            {
                let mut last_chat = state.last_chat.lock().unwrap();
                if last_chat.elapsed() < Duration::from_secs(5) {
                    return Ok(());
                }
                *last_chat = Instant::now();
            }

            // Build context and call Gemini
            let context = build_context(&state, content, sender);
            let use_pro = content.to_lowercase().contains("java")
                || content.to_lowercase().contains("code")
                || content.to_lowercase().contains("redstone")
                || content.len() > 100; // Long messages get Pro

            let model = if use_pro {
                config.model_pro.clone()
            } else {
                config.model_flash.clone()
            };

            let api_key = config.gemini_api_key.clone();
            let bot_name = config.bot_name.clone();

            println!("[BRAIN] 🧠 Responding to <{}> using {}", sender, model);

            // Spawn async to not block
            let state_clone = state.clone();
            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let url = format!(
                    "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                    model, api_key
                );

                let request_body = GeminiRequest {
                    contents: vec![GContent {
                        role: "user".into(),
                        parts: vec![GPart { text: context }],
                    }],
                    generation_config: GenerationConfig {
                        max_output_tokens: 60, // Short like a real player
                        temperature: 0.9,       // Creative
                    },
                };

                match client.post(&url).json(&request_body).send().await {
                    Ok(resp) => {
                        if let Ok(json) = resp.json::<GeminiResponse>().await {
                            if let Some(candidates) = json.candidates {
                                if let Some(first) = candidates.first() {
                                    if let Some(part) = first.content.parts.first() {
                                        let reply = part.text.trim().to_string();
                                        // Truncate to MC chat limit (256 chars)
                                        let reply = if reply.len() > 250 {
                                            reply[..250].to_string()
                                        } else {
                                            reply
                                        };
                                        println!("[BRAIN] 💬 Reply: {}", reply);
                                        // bot.chat(&reply); // Enable when connected to real server

                                        // Add to history
                                        let mut history = state_clone.chat_history.lock().unwrap();
                                        history.push(format!("<{}> {}", bot_name, reply));
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => println!("[BRAIN] ❌ API Error: {}", e),
                }

                // Auto-save memory periodically
                let mut counter = state_clone.save_counter.lock().unwrap();
                *counter += 1;
                if *counter % 10 == 0 {
                    let memory = state_clone.memory.lock().unwrap();
                    memory.save();
                    println!("[BRAIN] 💾 Memory saved.");
                }
            });
        }
        Event::Tick => {
            // Personality decay (moods fade over time)
            let mut personality = state.personality.lock().unwrap();
            personality.on_event(&PersonalityEvent::TimePassed);
        }
        _ => {}
    }
    Ok(())
}
