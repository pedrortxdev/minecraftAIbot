use azalea::prelude::*;
use serde::{Deserialize, Serialize};
use crate::config::Config;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: u32,
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

#[derive(Clone, Component)]
pub struct State {
    pub last_chat: Arc<Mutex<Instant>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            last_chat: Arc::new(Mutex::new(Instant::now() - Duration::from_secs(60))), // Ready immediately
        }
    }
}

pub async fn handle(_bot: Client, event: Event, state: State) -> anyhow::Result<()> {
    if let Event::Chat(chat) = event {
        let message = chat.message().to_string();
        // Ignore self
        if message.contains("PedroRTX") && message.contains("<") { 
            // Basic simplistic check, real check involves Sender
        }
        
        let triggers = ["lag", "tps", "java", "code", "bot", "daniel", "frankfurt"];
        if triggers.iter().any(|&t| message.to_lowercase().contains(t)) {
             let mut last_chat = state.last_chat.lock().unwrap();
             if last_chat.elapsed() < Duration::from_secs(10) {
                 return Ok(()); // Rate limit
             }
             *last_chat = Instant::now();
             drop(last_chat); // release lock before await

             let config = Config::load();
             let use_pro = message.to_lowercase().contains("java") || message.to_lowercase().contains("code");
             let model = if use_pro { config.model_pro.clone() } else { config.model_flash.clone() };
             
             println!("[BRAIN] Triggered! Using model: {}", model);

             let system_prompt = if use_pro {
                 "You are PedroRTX, an arrogant sysadmin bot. You are elitist about Rust code and hate Java garbage collection. You give detailed, technical, but condescending explanations."
             } else {
                 "You are PedroRTX, a toxic Minecraft bot hosted in Frankfurt. You allow no lag. You respond with short, sharp insults or status reports."
             };

             let prompt = format!("{}\nUser said: {}", system_prompt, message);

             // Spawn async task to not block bot tick
             tokio::spawn(async move {
                 let client = reqwest::Client::new();
                 let url = format!(
                     "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                     model, config.gemini_api_key
                 );

                 let request_body = GeminiRequest {
                     contents: vec![Content {
                         parts: vec![Part { text: prompt }],
                     }],
                     generation_config: GenerationConfig { max_output_tokens: 100 },
                 };

                 match client.post(&url).json(&request_body).send().await {
                     Ok(resp) => {
                         if let Ok(json) = resp.json::<GeminiResponse>().await {
                             if let Some(candidates) = json.candidates {
                                 if let Some(first) = candidates.first() {
                                     if let Some(part) = first.content.parts.first() {
                                         let reply = part.text.trim();
                                         println!("[BRAIN] Reply: {}", reply);
                                         // bot.chat(reply); // In real usage
                                     }
                                 }
                             }
                         }
                     }
                     Err(e) => println!("[BRAIN] API Error: {}", e),
                 }
             });
        }
    }
    Ok(())
}
