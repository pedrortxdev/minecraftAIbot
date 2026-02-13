use azalea::prelude::*;
use std::time::{Instant, Duration};
use std::sync::{Arc, Mutex};

#[derive(Clone, Component)]
pub struct State {
    pub last_ping: Arc<Mutex<Instant>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            last_ping: Arc::new(Mutex::new(Instant::now())),
        }
    }
}

pub async fn handle(_bot: Client, event: Event, state: State) -> anyhow::Result<()> {
    if let Event::Tick = event {
        let mut last_ping = state.last_ping.lock().unwrap();
        if last_ping.elapsed() >= Duration::from_secs(10) {
            println!("[HEARTBEAT] Bot is alive.");
            *last_ping = Instant::now();
        }
    }
    Ok(())
}
