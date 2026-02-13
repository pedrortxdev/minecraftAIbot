use serde::{Deserialize, Serialize};
use crate::cognitive::memory::{SocialMemory, Relationship};
use rand::Rng;

// ============================================================
// SOCIAL ENGINE — Natural social behavior
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialEngine {
    pub nearby_players: Vec<String>,
    pub conversations_active: Vec<String>, // Player names we're chatting with
    pub total_messages_sent: u32,
    pub help_requests_made: u32,
    pub help_threshold: u32, // How many failures before asking
}

impl Default for SocialEngine {
    fn default() -> Self {
        Self {
            nearby_players: vec![],
            conversations_active: vec![],
            total_messages_sent: 0,
            help_requests_made: 0,
            help_threshold: 3,
        }
    }
}

impl SocialEngine {
    /// Decide how to respond to a message based on relationship
    pub fn should_respond(&self, player: &str, social: &SocialMemory) -> ResponseStyle {
        let profile = social.players.get(player);

        match profile {
            None => {
                // Unknown player — be cautious
                ResponseStyle::Cautious
            }
            Some(p) => match p.relationship {
                Relationship::BestFriend => ResponseStyle::Friendly,
                Relationship::Friend => ResponseStyle::Casual,
                Relationship::Acquaintance => ResponseStyle::Casual,
                Relationship::Stranger => ResponseStyle::Cautious,
                Relationship::Rival => ResponseStyle::Cold,
                Relationship::Enemy => ResponseStyle::Hostile,
            },
        }
    }

    /// Should we greet a player? (first time seeing them this session)
    pub fn should_greet(&self, player: &str, social: &SocialMemory) -> bool {
        if self.nearby_players.contains(&player.to_string()) {
            return false; // Already greeted
        }
        // Only greet if relationship is positive
        social
            .players
            .get(player)
            .map(|p| p.trust_level >= 0)
            .unwrap_or(true) // Greet strangers
    }

    /// Generate a greeting based on relationship
    pub fn generate_greeting(&self, player: &str, social: &SocialMemory) -> String {
        let mut rng = rand::thread_rng();
        let profile = social.players.get(player);

        match profile {
            None => {
                // Never met
                let greetings = [
                    format!("eai {}", player),
                    format!("salve {}", player),
                    format!("opa {}", player),
                    format!("fala {}", player),
                ];
                greetings[rng.gen_range(0..greetings.len())].clone()
            }
            Some(p) if p.relationship == Relationship::BestFriend => {
                let greetings = [
                    format!("EEEEE {} tmjjj", player),
                    format!("salveee {} bora jogar", player),
                    format!("ae {} cê sumiu hein", player),
                ];
                greetings[rng.gen_range(0..greetings.len())].clone()
            }
            Some(p) if p.relationship == Relationship::Friend => {
                let greetings = [
                    format!("eai {} blz", player),
                    format!("fala {}", player),
                    format!("salve {} quanto tempo", player),
                ];
                greetings[rng.gen_range(0..greetings.len())].clone()
            }
            Some(p) if p.relationship == Relationship::Enemy => {
                let greetings = [
                    format!("..."),
                    format!("la vem"),
                ];
                greetings[rng.gen_range(0..greetings.len())].clone()
            }
            _ => format!("eai {}", player),
        }
    }

    /// Should the bot ask for help?
    pub fn should_ask_for_help(&self, _task: &str, failures: u32) -> bool {
        failures >= self.help_threshold
    }

    /// Generate a help request (reluctant, realistic)
    pub fn generate_help_request(&self, player: &str, item: &str, social: &SocialMemory) -> Option<String> {
        let mut rng = rand::thread_rng();
        let profile = social.players.get(player)?;

        // Only ask friends+ for help
        if profile.trust_level < 40 {
            return None;
        }

        let requests = [
            format!("{} ce tem {} sobrando? to precisando mn", player, item),
            format!("mn {} me empresta {} rapidao? dps eu devolvo", player, item),
            format!("{} eu tentei de tudo mas n to achando {}, ce me ajuda?", player, item),
        ];

        self.help_requests_made;
        Some(requests[rng.gen_range(0..requests.len())].clone())
    }

    /// Should we warn a player about danger?
    pub fn should_warn_player(&self, player: &str, social: &SocialMemory) -> bool {
        social
            .players
            .get(player)
            .map(|p| p.trust_level > 10) // Warn anyone who's not an enemy
            .unwrap_or(true)
    }

    pub fn context_summary(&self) -> String {
        format!(
            "Jogadores próximos: {} | Msgs enviadas: {} | Pedidos de ajuda: {}",
            if self.nearby_players.is_empty() {
                "nenhum".into()
            } else {
                self.nearby_players.join(", ")
            },
            self.total_messages_sent,
            self.help_requests_made,
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseStyle {
    Friendly,   // Talkative, uses emoji, shares info
    Casual,     // Normal conversation
    Cautious,   // Short answers, asks questions
    Cold,       // Minimal interaction
    Hostile,    // Aggressive or ignoring
}
