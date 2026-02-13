use azalea::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq)]
pub enum MiningState {
    Idle,
    FindingTree,
    Chopping,
    Crafting,
    MiningStone,
}

#[derive(Clone, Component)]
pub struct State {
    pub current: Arc<Mutex<MiningState>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            current: Arc::new(Mutex::new(MiningState::Idle)),
        }
    }
}

pub async fn handle(_bot: Client, event: Event, state: State) -> anyhow::Result<()> {
    match event {
        Event::Tick => {
            let mut current = state.current.lock().unwrap();
            match *current {
                MiningState::Idle => {
                    // Do nothing
                }
                MiningState::FindingTree => {
                    // Placeholder logic
                    println!("Searching for tree...");
                    // Change state
                    *current = MiningState::Idle; 
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}
