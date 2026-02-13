use std::env;

pub struct Config {
    pub server_address: String,
    pub server_port: u16,
    pub bot_email: String,
    pub bot_name: String,
    pub gemini_api_key: String,
    pub model_flash: String,
    pub model_pro: String,
}

impl Config {
    pub fn load() -> Self {
        Self {
            server_address: env::var("MC_SERVER").unwrap_or_else(|_| "duiker.aternos.host".to_string()),
            server_port: env::var("MC_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(35809),
            bot_email: env::var("MS_EMAIL").unwrap_or_default(), // Empty for offline
            bot_name: env::var("BOT_NAME").unwrap_or_else(|_| "PedroRTX".to_string()),
            gemini_api_key: env::var("GEMINI_API_KEY").unwrap_or_else(|_| "AIzaSyAQsaKY12g9teuuWgsNBVt-wxSWyrIZnWY".to_string()),
            model_flash: env::var("MODEL_FLASH").unwrap_or_else(|_| "gemini-2.0-flash".to_string()),
            model_pro: env::var("MODEL_PRO").unwrap_or_else(|_| "gemini-2.5-pro".to_string()),
        }
    }
}
