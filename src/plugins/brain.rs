use azalea::prelude::*;
use serde::{Deserialize, Serialize};
use crate::config::Config;
use crate::cognitive::memory::Memory;
use crate::cognitive::personality::{Personality, PersonalityEvent};
use crate::cognitive::goal_planner::GoalPlanner;
use crate::systems::world_scanner::WorldState;
use crate::systems::social::{SocialEngine, ResponseStyle};
use crate::systems::typos;
use crate::systems::economy::Economy;
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
    pub economy: Arc<Mutex<Economy>>,
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
            economy: Arc::new(Mutex::new(Economy::new())),
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
    let economy = state.economy.lock().unwrap();
    let chat_history = state.chat_history.lock().unwrap();

    // Get relationship context
    let relationship_ctx = memory.social.players.get(sender).map(|p| {
        format!(
            "Relação com {}: {:?} (confiança: {}). Já se viram {}x. Notas: {:?}",
            sender, p.relationship, p.trust_level, p.times_met, p.notes
        )
    }).unwrap_or_else(|| format!("{} é um desconhecido. Primeira vez que vocês conversam.", sender));

    // Economy context: debts, credit, trade decisions
    let economy_ctx = economy.context_summary();

    // Detect trade requests and inject trade decision
    let trade_keywords = ["me dá", "me da", "empresta", "troca", "preciso de", "tem sobrando", "arruma"];
    let msg_lower = incoming_message.to_lowercase();
    let trade_hint = if trade_keywords.iter().any(|kw| msg_lower.contains(kw)) {
        // Try to extract what item they want (very basic extraction)
        let items = ["diamante", "ferro", "ouro", "esmeralda", "netherite", "comida",
                     "diamond", "iron", "gold", "emerald", "bread", "redstone"];
        let requested_item = items.iter()
            .find(|i| msg_lower.contains(*i))
            .unwrap_or(&"item");
        let decision = economy.evaluate_request(sender, requested_item, 1);
        format!("\n⚠️ TRADE REQUEST DETECTADO: {} quer {}. Sua decisão econômica: {:?}", sender, requested_item, decision)
    } else {
        String::new()
    };

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

=== ECONOMIA (Dívidas e Favores) ===
{}{}

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
        economy_ctx,
        trade_hint,
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

/// Public wrapper for bot.rs to use
pub fn extract_sender_pub(message: &str) -> Option<(&str, &str)> {
    extract_sender(message)
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
            let bot_clone = _bot.clone();  // Clone bot so we can chat inside spawn
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

                println!("[BRAIN] 📡 Calling Gemini API...");

                // Retry loop for rate limits (429)
                let max_retries = 3;
                let mut attempt = 0;
                let response_result = loop {
                    attempt += 1;
                    match client.post(&url).json(&request_body).send().await {
                        Ok(resp) => {
                            let status = resp.status();
                            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                                let body = resp.text().await.unwrap_or_default();
                                if attempt < max_retries {
                                    let wait_secs = 2u64.pow(attempt as u32); // 2s, 4s, 8s
                                    println!("[BRAIN] ⏳ Rate limited (429), retry {}/{} in {}s...", attempt, max_retries, wait_secs);
                                    tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;
                                    continue;
                                } else {
                                    println!("[BRAIN] ❌ Rate limited (429) after {} retries. Quota esgotada.", max_retries);
                                    println!("[BRAIN] 📋 {}", &body[..body.len().min(200)]);
                                    break None;
                                }
                            }
                            if !status.is_success() {
                                let body = resp.text().await.unwrap_or_else(|_| "<failed to read body>".into());
                                println!("[BRAIN] ❌ API HTTP Error {}: {}", status, body);
                                break None;
                            }
                            break Some(resp);
                        }
                        Err(e) => {
                            println!("[BRAIN] ❌ API Network Error: {}", e);
                            println!("[BRAIN] 🔌 Check internet connection and API key");
                            break None;
                        }
                    }
                };

                let resp = match response_result {
                    Some(r) => r,
                    None => return, // All retries failed or error
                };
                let body_text = match resp.text().await {
                    Ok(t) => t,
                    Err(e) => {
                        println!("[BRAIN] ❌ Failed to read response body: {}", e);
                        return;
                    }
                };
                match serde_json::from_str::<GeminiResponse>(&body_text) {
                    Ok(json) => {
                        match json.candidates {
                            Some(candidates) if !candidates.is_empty() => {
                                let first = &candidates[0];
                                if let Some(part) = first.content.parts.first() {
                                    let raw_reply = part.text.trim().to_string();

                                    // === TYPOS MIDDLEWARE ===
                                    let current_mood = {
                                        let p = state_clone.personality.lock().unwrap();
                                        p.mood.clone()
                                    };
                                    let reply = typos::apply_typos(&raw_reply, &current_mood);

                                    // Truncate to MC chat limit (256 chars)
                                    let reply = if reply.len() > 250 {
                                        reply[..250].to_string()
                                    } else {
                                        reply
                                    };
                                    println!("[BRAIN] 💬 Raw: {}", raw_reply);
                                    println!("[BRAIN] 🤙 Sent: {}", reply);
                                    bot_clone.chat(&reply); // 🔊 FALA, PEDRTX!

                                    // Add to history
                                    let mut history = state_clone.chat_history.lock().unwrap();
                                    history.push(format!("<{}> {}", bot_name, reply));
                                } else {
                                    println!("[BRAIN] ⚠️ Gemini returned candidate with no parts");
                                }
                            }
                            Some(_) => {
                                println!("[BRAIN] ⚠️ Gemini returned empty candidates array");
                            }
                            None => {
                                println!("[BRAIN] ⚠️ Gemini returned NO candidates. Body: {}", &body_text[..body_text.len().min(500)]);
                            }
                        }
                    }
                    Err(e) => {
                        println!("[BRAIN] ❌ Failed to parse Gemini JSON: {}", e);
                        println!("[BRAIN] 📋 Response body: {}", &body_text[..body_text.len().min(500)]);
                    }
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
