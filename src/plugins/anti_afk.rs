use azalea::prelude::*;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct State {
    pub last_action: Arc<Mutex<Instant>>,
}

pub async fn handle(bot: Client, event: Event, state: State) -> anyhow::Result<()> {
    if let Event::Tick = event {
        let mut last_action = state.last_action.lock().unwrap();
        if last_action.elapsed() > Duration::from_secs(60) {
            bot.jump();
            *last_action = Instant::now();
        }
    }
    Ok(())
}
