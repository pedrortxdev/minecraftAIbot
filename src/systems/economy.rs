use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ============================================================
// ECONOMY — Debt, Favors, Negotiation & Loan Sharking
// "Me arruma 5 ouros que a gente conversa"
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Debt {
    pub item: String,
    pub quantity: u32,
    pub created_at: DateTime<Utc>,
    pub reason: String,
    pub paid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Favor {
    pub description: String,
    pub weight: i32, // Positive = they owe us, negative = we owe them
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerLedger {
    pub debts_owed_to_us: Vec<Debt>,     // Player owes the bot
    pub debts_we_owe: Vec<Debt>,          // Bot owes the player
    pub favors: Vec<Favor>,
    pub total_given_to_them: HashMap<String, u32>,  // item -> count
    pub total_received_from_them: HashMap<String, u32>,
    pub credit_score: i32,                // -100 (deadbeat) to 100 (reliable)
    pub trade_count: u32,
}

impl PlayerLedger {
    /// Calculate the net balance (positive = they owe us more)
    pub fn net_balance(&self) -> i32 {
        let owed_to_us: i32 = self.debts_owed_to_us.iter()
            .filter(|d| !d.paid)
            .map(|d| d.quantity as i32)
            .sum();
        let we_owe: i32 = self.debts_we_owe.iter()
            .filter(|d| !d.paid)
            .map(|d| d.quantity as i32)
            .sum();
        owed_to_us - we_owe
    }

    /// How much of a specific item have we given without return?
    pub fn unreturned_amount(&self, item: &str) -> u32 {
        let given = self.total_given_to_them.get(item).copied().unwrap_or(0);
        let received = self.total_received_from_them.get(item).copied().unwrap_or(0);
        given.saturating_sub(received)
    }

    /// Update credit score based on behavior
    pub fn update_credit_score(&mut self) {
        let unpaid_debts: u32 = self.debts_owed_to_us.iter()
            .filter(|d| !d.paid)
            .map(|d| d.quantity)
            .sum();

        let paid_debts: u32 = self.debts_owed_to_us.iter()
            .filter(|d| d.paid)
            .map(|d| d.quantity)
            .sum();

        let old_debts = self.debts_owed_to_us.iter()
            .filter(|d| !d.paid)
            .filter(|d| Utc::now().signed_duration_since(d.created_at).num_hours() > 24)
            .count() as i32;

        self.credit_score = (paid_debts as i32 * 5 - unpaid_debts as i32 * 3 - old_debts * 10)
            .clamp(-100, 100);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Economy {
    pub ledgers: HashMap<String, PlayerLedger>,
    pub item_values: HashMap<String, u32>, // Subjective item value
    pub total_trades: u32,
}

impl Economy {
    pub fn new() -> Self {
        let mut item_values = HashMap::new();
        // Base item values (in "iron ingot equivalents")
        item_values.insert("diamond".into(), 10);
        item_values.insert("iron_ingot".into(), 1);
        item_values.insert("gold_ingot".into(), 3);
        item_values.insert("emerald".into(), 8);
        item_values.insert("netherite_ingot".into(), 50);
        item_values.insert("coal".into(), 0);  // Worthless
        item_values.insert("cobblestone".into(), 0);
        item_values.insert("oak_log".into(), 0);
        item_values.insert("bread".into(), 0);
        item_values.insert("cooked_porkchop".into(), 1);
        item_values.insert("enchanted_golden_apple".into(), 100);
        item_values.insert("elytra".into(), 200);
        item_values.insert("totem_of_undying".into(), 80);
        item_values.insert("redstone".into(), 0);

        Economy {
            ledgers: HashMap::new(),
            item_values,
            total_trades: 0,
        }
    }

    pub fn get_ledger(&mut self, player: &str) -> &mut PlayerLedger {
        self.ledgers.entry(player.to_string()).or_insert_with(PlayerLedger::default)
    }

    /// Record that we gave an item to a player
    pub fn record_gift(&mut self, player: &str, item: &str, quantity: u32, reason: &str) {
        let ledger = self.get_ledger(player);
        *ledger.total_given_to_them.entry(item.to_string()).or_insert(0) += quantity;
        ledger.debts_owed_to_us.push(Debt {
            item: item.to_string(),
            quantity,
            created_at: Utc::now(),
            reason: reason.to_string(),
            paid: false,
        });
        ledger.update_credit_score();
        println!("[ECONOMY] 📝 {} agora deve {} x{} (razão: {})", player, item, quantity, reason);
    }

    /// Record that a player gave us something
    pub fn record_received(&mut self, player: &str, item: &str, quantity: u32) {
        let ledger = self.get_ledger(player);
        *ledger.total_received_from_them.entry(item.to_string()).or_insert(0) += quantity;

        // Try to mark debts as paid
        let mut remaining = quantity;
        for debt in ledger.debts_we_owe.iter_mut().filter(|d| !d.paid && d.item == item) {
            if remaining >= debt.quantity {
                remaining -= debt.quantity;
                debt.paid = true;
            }
        }

        ledger.update_credit_score();
        self.total_trades += 1;
    }

    /// Should we give this player what they asked for?
    pub fn evaluate_request(&self, player: &str, item: &str, quantity: u32) -> TradeDecision {
        let ledger = match self.ledgers.get(player) {
            Some(l) => l,
            None => return TradeDecision::Cautious("nunca negociei com vc antes".into()),
        };

        // Check credit score
        if ledger.credit_score < -20 {
            let unpaid: u32 = ledger.debts_owed_to_us.iter()
                .filter(|d| !d.paid)
                .map(|d| d.quantity)
                .sum();
            return TradeDecision::Refuse(format!(
                "mano me deve {} itens ainda e quer mais?? paga primeiro",
                unpaid
            ));
        }

        // Check if they have unpaid debts
        let unpaid_count = ledger.debts_owed_to_us.iter().filter(|d| !d.paid).count();
        if unpaid_count > 2 {
            return TradeDecision::Negotiate(format!(
                "te dei {} coisas e tu n devolveu nada. me arruma algo primeiro",
                unpaid_count
            ));
        }

        // Check item value
        let value = self.item_values.get(item).copied().unwrap_or(1) * quantity;
        if value > 20 {
            return TradeDecision::Negotiate(format!(
                "{} x{} é muito caro. o que vc tem pra trocar?",
                item, quantity
            ));
        }

        if value == 0 {
            // Cheap item, give freely
            return TradeDecision::Accept("toma ai, isso n vale nada mesmo".into());
        }

        // Fair trade
        if ledger.credit_score > 30 {
            TradeDecision::Accept("toma, vc é gnt boa".into())
        } else {
            TradeDecision::Negotiate("depende, o que vc me dá em troca?".into())
        }
    }

    /// Proactive: Should the bot offer a trade to a player?
    pub fn find_trade_opportunity(
        &self,
        player: &str,
        player_needs: &str,  // What we think they need
        we_have: &[String],  // Items in our inventory
        they_have: &[String], // Items we think they have
    ) -> Option<String> {
        let ledger = match self.ledgers.get(player) {
            Some(l) => l,
            None => return None,
        };

        // Only trade with people we somewhat trust
        if ledger.credit_score < -10 {
            return None;
        }

        // Check if we have what they need
        let we_have_it = we_have.iter().any(|i| i.contains(player_needs));
        if !we_have_it {
            return None;
        }

        // What do they have that we want?
        let valuable_they_have: Vec<&String> = they_have.iter()
            .filter(|i| {
                self.item_values.get(i.as_str()).copied().unwrap_or(0) > 3
            })
            .collect();

        if let Some(want) = valuable_they_have.first() {
            Some(format!(
                "eai {}, vi que vc ta precisando de {}. te faço por {} unidades de {}, bora?",
                player, player_needs, 1, want
            ))
        } else {
            None
        }
    }

    pub fn context_summary(&self) -> String {
        let mut s = format!("Total trades: {}\n", self.total_trades);
        for (player, ledger) in &self.ledgers {
            let balance = ledger.net_balance();
            let credit = ledger.credit_score;
            s.push_str(&format!(
                "  {}: balanço={} crédito={}\n",
                player,
                if balance > 0 { format!("devem {} itens", balance) } else if balance < 0 { format!("devemos {} itens", balance.abs()) } else { "zerado".into() },
                credit
            ));
        }
        s
    }
}

#[derive(Debug, Clone)]
pub enum TradeDecision {
    Accept(String),    // Give with a comment
    Refuse(String),    // Deny with a reason
    Negotiate(String), // Counter-offer
    Cautious(String),  // Unsure, proceed carefully
}
