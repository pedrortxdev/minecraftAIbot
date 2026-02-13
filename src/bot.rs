use crate::plugins; // Use crate::plugins

use azalea::prelude::*;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Component)] // Derive Component
pub struct State {
    pub anti_afk: plugins::anti_afk::State,
    pub mining: plugins::mining::State,
    pub brain: plugins::brain::State,
    pub ping: plugins::ping::State,
}

impl Default for State {
    fn default() -> Self {
        Self {
            anti_afk: plugins::anti_afk::State {
                last_action: Arc::new(Mutex::new(Instant::now())),
            },
            mining: plugins::mining::State::default(),
            brain: plugins::brain::State::default(),
            ping: plugins::ping::State::default(),
        }
    }
}

pub async fn handle(bot: Client, event: Event, state: State) -> anyhow::Result<()> {
    match &event {
        Event::Login => {
            println!("Bot joined the server!");
        }
        Event::Chat(chat) => {
            println!("[CHAT] {}", chat.message().to_string()); // Simplified chat logging
            // Dispatch to brain
            let _ = plugins::brain::handle(bot.clone(), event.clone(), state.brain.clone()).await;
        }
        _ => {}
    }

    // Dispatch to plugins
    // Note: In real Azalea, you might use a Plugin trait, but direct calling is fine for now.
    // launching tasks for async handlers if needed, or just awaiting them if they are fast.
    
    // checks
    if let Event::Tick = &event {
         plugins::auto_eat::handle(bot.clone(), event.clone(), ()).await?;
         plugins::anti_afk::handle(bot.clone(), event.clone(), state.anti_afk.clone()).await?;
         plugins::mining::handle(bot.clone(), event.clone(), state.mining.clone()).await?;
         plugins::inventory::handle(bot.clone(), event.clone(), ()).await?;
         plugins::ping::handle(bot.clone(), event.clone(), state.ping.clone()).await?;
    }
    
    Ok(())
}
