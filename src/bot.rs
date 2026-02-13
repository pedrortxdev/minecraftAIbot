use crate::plugins;
use crate::cognitive;
// use crate::systems;

use azalea::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Component)]
pub struct State {
    pub anti_afk: plugins::anti_afk::State,
    pub brain: plugins::brain::State, // Brain V2 now holds everything
    pub ping: plugins::ping::State,
}

impl Default for State {
    fn default() -> Self {
        Self {
            anti_afk: plugins::anti_afk::State {
                last_action: Arc::new(Mutex::new(Instant::now())),
            },
            brain: plugins::brain::State::default(),
            ping: plugins::ping::State::default(),
        }
    }
}

pub async fn handle(bot: Client, event: Event, state: State) -> anyhow::Result<()> {
    match &event {
        Event::Login => {
            println!("[BOT] ✅ Joined the server!");
            // Record in memory
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
            println!("[CHAT] {}", chat.message().to_string());
            // Brain handles everything (social, personality, memory, response)
            let _ = plugins::brain::handle(bot.clone(), event.clone(), state.brain.clone()).await;
        }
        Event::Disconnect(reason) => {
            println!("[DISCONNECT] Bot kicked/disconnected!");
            if let Some(r) = reason {
                println!("[DISCONNECT] Reason: {}", r.to_string());
            } else {
                println!("[DISCONNECT] No reason provided.");
            }
            // Save memory on disconnect
            let memory = state.brain.memory.lock().unwrap();
            memory.save();
            println!("[BOT] 💾 Memory saved on disconnect.");
        }
        _ => {}
    }

    // Tick-based plugins
    if let Event::Tick = &event {
        plugins::auto_eat::handle(bot.clone(), event.clone(), ()).await?;
        plugins::anti_afk::handle(bot.clone(), event.clone(), state.anti_afk.clone()).await?;
        plugins::ping::handle(bot.clone(), event.clone(), state.ping.clone()).await?;
        // Brain tick (personality decay)
        let _ = plugins::brain::handle(bot.clone(), event.clone(), state.brain.clone()).await;
    }

    Ok(())
}
