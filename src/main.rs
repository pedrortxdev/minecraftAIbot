mod bot;
mod config;
pub mod plugins; // Make plugins accessible


// use azalea::prelude::*;
use config::Config;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // tracing_subscriber::fmt::init();
    
    let config = Config::load();
    let address = format!("{}:{}", config.server_address, config.server_port);

    println!("Starting Frankfurt Sentinel...");
    println!("Target: {}", address);

    loop {
        println!("Connecting as {}...", config.bot_name);
        
        let account = if !config.bot_email.is_empty() {
            println!("Using Microsoft Authentication for {}", config.bot_email);
            azalea::Account::microsoft(&config.bot_email).await
        } else {
            println!("Using Offline Mode for {}", config.bot_name);
            Ok(azalea::Account::offline(&config.bot_name))
        };

        if let Ok(account) = account {
            let _result = azalea::ClientBuilder::new()
                .set_handler(bot::handle)
                .start(account, address.as_str())
                .await;

            println!("Bot disconnected/stopped. Reconnecting in 5 seconds...");
            tokio::time::sleep(Duration::from_secs(5)).await;
        } else {
            println!("Authentication failed: {:?}. Retrying in 10 seconds...", account.err());
            tokio::time::sleep(Duration::from_secs(5)).await;
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
