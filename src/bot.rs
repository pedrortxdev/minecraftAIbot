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
    }

    Ok(())
}
