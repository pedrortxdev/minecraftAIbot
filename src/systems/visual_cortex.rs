use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::config::Config;

// ============================================================
// VISUAL CORTEX — Architectural Judgment via Gemini
// Scans 16x16x16, builds a heatmap, asks Gemini to judge
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockScan {
    pub block_counts: HashMap<String, u32>,
    pub total_blocks: u32,
    pub air_percentage: f32,
    pub light_avg: f32,
    pub unique_types: u32,
    pub center: [i32; 3],
}

impl BlockScan {
    /// Analyze scanned blocks into a human-readable summary
    pub fn to_summary(&self) -> String {
        let mut lines = vec![];

        // Material composition
        let mut sorted: Vec<_> = self.block_counts.iter()
            .filter(|(k, _)| *k != "air" && *k != "cave_air")
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));

        if sorted.is_empty() {
            return "Área vazia, só ar.".into();
        }

        lines.push(format!("Posição: [{}, {}, {}]", self.center[0], self.center[1], self.center[2]));
        lines.push(format!("Blocos sólidos: {} | Tipos únicos: {}", self.total_blocks, self.unique_types));

        // Top 8 materials
        lines.push("Materiais principais:".into());
        for (block, count) in sorted.iter().take(8) {
            let pct = (**count as f32 / self.total_blocks.max(1) as f32) * 100.0;
            lines.push(format!("  - {}: {} ({:.0}%)", block, count, pct));
        }

        // Structural analysis
        let structure = self.detect_structure_type();
        lines.push(format!("Estrutura detectada: {}", structure));

        // Quality assessment
        let quality = self.assess_quality();
        lines.push(format!("Qualidade estimada: {}", quality));

        lines.join("\n")
    }

    /// Detect what kind of structure this is
    fn detect_structure_type(&self) -> &str {
        let has = |name: &str| self.block_counts.get(name).copied().unwrap_or(0) > 0;
        let count = |name: &str| self.block_counts.get(name).copied().unwrap_or(0);

        // Dirt house
        if count("dirt") > 20 && self.unique_types < 5 {
            return "Casa de Noob (muita dirt, pouca variedade)";
        }

        // Cobblestone box
        if count("cobblestone") > 30 && self.unique_types < 4 {
            return "Caixa de Cobble (funcional mas feio)";
        }

        // Farm
        if count("farmland") > 10 || count("wheat") > 5 {
            if has("water") {
                return "Farm (com irrigação)";
            }
            return "Farm (SEM irrigação, eficiência péssima)";
        }

        // Redstone contraption
        let redstone_total = count("redstone_wire") + count("repeater")
            + count("comparator") + count("piston") + count("observer");
        if redstone_total > 10 {
            if has("comparator") && has("observer") {
                return "Circuito de Redstone (avançado)";
            }
            return "Circuito de Redstone (básico)";
        }

        // Well-built house
        if self.unique_types > 6 && (has("glass") || has("glass_pane"))
            && (has("oak_door") || has("spruce_door") || has("iron_door"))
        {
            return "Casa decorada (esforço detectado)";
        }

        // Storage area
        if count("chest") > 4 || count("barrel") > 4 {
            return "Área de armazenamento";
        }

        // Enchanting room
        if has("enchanting_table") && count("bookshelf") > 5 {
            return "Sala de encantamento";
        }

        // Nether portal
        if count("obsidian") >= 10 {
            return "Portal do Nether";
        }

        // TNT
        if count("tnt") > 2 {
            return "⚠️ TNT detectada (possível grief)";
        }

        if self.total_blocks < 10 {
            return "Área quase vazia";
        }

        "Estrutura desconhecida"
    }

    /// Quick quality score without Gemini
    fn assess_quality(&self) -> &str {
        let variety = self.unique_types;
        let has_glass = self.block_counts.get("glass").copied().unwrap_or(0)
            + self.block_counts.get("glass_pane").copied().unwrap_or(0);
        let has_slabs = self.block_counts.contains_key("oak_slab")
            || self.block_counts.contains_key("stone_slab")
            || self.block_counts.contains_key("spruce_slab");
        let has_stairs = self.block_counts.keys().any(|k| k.contains("stairs"));

        if variety > 8 && has_glass > 3 && has_slabs && has_stairs {
            "⭐⭐⭐⭐⭐ Obra de arte"
        } else if variety > 5 && (has_glass > 0 || has_stairs) {
            "⭐⭐⭐⭐ Decente, esforço visível"
        } else if variety > 3 {
            "⭐⭐⭐ Medíocre, básico mas funcional"
        } else if variety > 1 {
            "⭐⭐ Fraco, quase zero criatividade"
        } else {
            "⭐ Vergonha alheia"
        }
    }
}

/// Build the Gemini prompt for architectural judgment
pub fn build_judgment_prompt(scan: &BlockScan) -> String {
    format!(
r#"Você é um crítico de arquitetura de Minecraft. Você é veterano desde a beta.
Analise essa estrutura e dê sua opinião CURTA (1-2 linhas) em português informal brasileiro.
Seja honesto, sarcástico se for ruim, elogioso se for bom.
Use gírias: "mn", "slk", "kkkk", "pqp", "mds", "bora".
NÃO use linguagem formal. Fale como jogador real.

SCAN DA ÁREA:
{}

Responda SOMENTE o comentário que o jogador diria no chat."#,
        scan.to_summary()
    )
}

/// Decide if we should scan and judge (not too often)
#[derive(Debug, Clone)]
pub struct VisualCortexState {
    pub last_scan_pos: Option<[i32; 3]>,
    pub scans_done: u32,
    pub cooldown_ticks: u32,
    pub tick_counter: u32,
}

impl Default for VisualCortexState {
    fn default() -> Self {
        Self {
            last_scan_pos: None,
            scans_done: 0,
            cooldown_ticks: 0,
            tick_counter: 0,
        }
    }
}

impl VisualCortexState {
    /// Should we do a scan this tick?
    pub fn should_scan(&mut self, current_pos: [i32; 3]) -> bool {
        self.tick_counter += 1;

        if self.cooldown_ticks > 0 {
            self.cooldown_ticks -= 1;
            return false;
        }

        // Only scan every ~60 seconds
        if self.tick_counter % 1200 != 0 {
            return false;
        }

        // Don't re-scan the same area
        if let Some(last) = &self.last_scan_pos {
            let dx = (last[0] - current_pos[0]).abs();
            let dz = (last[2] - current_pos[2]).abs();
            if dx < 20 && dz < 20 {
                return false; // Too close to last scan
            }
        }

        self.last_scan_pos = Some(current_pos);
        self.scans_done += 1;
        self.cooldown_ticks = 600; // 30 second cooldown after scan
        true
    }
}

/// Send scan to Gemini for judgment (async, non-blocking)
pub async fn judge_with_gemini(scan: &BlockScan) -> Option<String> {
    let config = Config::load();
    let prompt = build_judgment_prompt(scan);

    let client = reqwest::Client::new();
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        config.model_pro, config.gemini_api_key
    );

    #[derive(serde::Serialize)]
    struct Req { contents: Vec<C>, #[serde(rename = "generationConfig")] generation_config: G }
    #[derive(serde::Serialize)]
    struct C { parts: Vec<P> }
    #[derive(serde::Serialize)]
    struct P { text: String }
    #[derive(serde::Serialize)]
    struct G { #[serde(rename = "maxOutputTokens")] max: u32, temperature: f32 }

    let body = Req {
        contents: vec![C { parts: vec![P { text: prompt }] }],
        generation_config: G { max: 80, temperature: 0.9 },
    };

    match client.post(&url).json(&body).send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                json["candidates"][0]["content"]["parts"][0]["text"]
                    .as_str()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        }
        Err(e) => {
            println!("[VISUAL] ❌ Gemini error: {}", e);
            None
        }
    }
}
