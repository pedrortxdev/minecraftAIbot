use rand::Rng;

// ============================================================
// JUDGE SYSTEM — Vinicius13 criticizes builds
// Reads nearby block patterns and generates roasts/comments
// ============================================================

#[derive(Debug, Clone, PartialEq)]
pub enum BuildQuality {
    Masterpiece,   // Great variety, complex patterns
    Decent,        // Reasonable effort
    Mediocre,      // Basic but functional
    Noob,          // Dirt houses, no design
    Griefed,       // Looks destroyed
}

#[derive(Debug, Clone)]
pub struct BlockPattern {
    pub block_type: String,
    pub count: u32,
    pub area: [i32; 3], // Bounding box of the detected pattern
}

/// Analyze a collection of nearby blocks and detect patterns
pub fn analyze_blocks(blocks: &[(String, [i32; 3])]) -> Vec<BuildJudgment> {
    let mut judgments = vec![];

    // Count block types
    let mut block_counts: std::collections::HashMap<&str, u32> = std::collections::HashMap::new();
    for (block, _pos) in blocks {
        *block_counts.entry(block.as_str()).or_insert(0) += 1;
    }

    // === DIRT HOUSE DETECTION ===
    let dirt_count = block_counts.get("dirt").copied().unwrap_or(0)
        + block_counts.get("dirt_path").copied().unwrap_or(0);
    if dirt_count > 20 {
        judgments.push(BuildJudgment {
            quality: BuildQuality::Noob,
            category: "Casa de Dirt".into(),
            comments: vec![
                "mds que casebre feio, quem construiu isso?".into(),
                "casa de terra em pleno 2025? serio msm?".into(),
                "ate eu na minha primeira vez fiz melhor q isso kkkk".into(),
                "mano isso ai n aguenta nem um creeper".into(),
                "q pena q n da pra dar F3 na vida real pra apagar isso".into(),
            ],
        });
    }

    // === COBBLESTONE BOX ===
    let cobble_count = block_counts.get("cobblestone").copied().unwrap_or(0);
    if cobble_count > 30 && block_counts.len() <= 3 {
        judgments.push(BuildJudgment {
            quality: BuildQuality::Mediocre,
            category: "Caixa de Cobble".into(),
            comments: vec![
                "caixa de cobble classica, faltou criatividade ne".into(),
                "pelo menos n eh de terra mas ta feio igual".into(),
                "se vc botasse umas janela ja melhorava 200%".into(),
            ],
        });
    }

    // === REDSTONE DETECTION ===
    let redstone_count = block_counts.get("redstone_wire").copied().unwrap_or(0)
        + block_counts.get("repeater").copied().unwrap_or(0)
        + block_counts.get("comparator").copied().unwrap_or(0)
        + block_counts.get("redstone_torch").copied().unwrap_or(0);
    if redstone_count > 5 {
        let has_repeaters = block_counts.get("repeater").copied().unwrap_or(0) > 0;
        let has_comparators = block_counts.get("comparator").copied().unwrap_or(0) > 0;

        if has_comparators {
            judgments.push(BuildJudgment {
                quality: BuildQuality::Decent,
                category: "Circuito de Redstone".into(),
                comments: vec![
                    "hmm, ta usando comparator, pelo menos sabe o basico".into(),
                    "esse circuito ta interessante, deixa eu ver".into(),
                    "quase bom, mas da pra otimizar com menos repeater".into(),
                ],
            });
        } else if has_repeaters {
            judgments.push(BuildJudgment {
                quality: BuildQuality::Mediocre,
                category: "Redstone Básica".into(),
                comments: vec![
                    "hmm, esse repetidor ta no delay errado hein".into(),
                    "redstone assim n escala, confia".into(),
                    "vc ta fazendo redstone ou espaguete?".into(),
                ],
            });
        }
    }

    // === FARM DETECTION ===
    let wheat_count = block_counts.get("wheat").copied().unwrap_or(0)
        + block_counts.get("farmland").copied().unwrap_or(0);
    if wheat_count > 10 {
        let has_water = block_counts.get("water").copied().unwrap_or(0) > 0;
        if !has_water {
            judgments.push(BuildJudgment {
                quality: BuildQuality::Noob,
                category: "Farm sem Água".into(),
                comments: vec![
                    "mano vc fez farm sem agua???? kkkkkkk".into(),
                    "eficiencia duvidosa isso ai".into(),
                    "planta sem agua, genio da agronomia".into(),
                ],
            });
        } else {
            judgments.push(BuildJudgment {
                quality: BuildQuality::Decent,
                category: "Farm".into(),
                comments: vec![
                    "farm ok mas podia ser automatica".into(),
                    "ta bom pra começo mas escala isso ai".into(),
                ],
            });
        }
    }

    // === GLASS/AESTHETIC BUILD ===
    let glass_count = block_counts.get("glass").copied().unwrap_or(0)
        + block_counts.get("glass_pane").copied().unwrap_or(0)
        + block_counts.get("tinted_glass").copied().unwrap_or(0);
    let slab_count = block_counts.get("oak_slab").copied().unwrap_or(0)
        + block_counts.get("stone_slab").copied().unwrap_or(0)
        + block_counts.get("spruce_slab").copied().unwrap_or(0);

    if glass_count > 5 && slab_count > 5 && block_counts.len() > 5 {
        judgments.push(BuildJudgment {
            quality: BuildQuality::Masterpiece,
            category: "Build Estético".into(),
            comments: vec![
                "opa, alguem q sabe construir aqui".into(),
                "hmm bonito, te dou um 7/10".into(),
                "finalmente alguem com bom gosto nesse server".into(),
            ],
        });
    }

    // === TNT / GRIEF ===
    let tnt_count = block_counts.get("tnt").copied().unwrap_or(0);
    if tnt_count > 3 {
        judgments.push(BuildJudgment {
            quality: BuildQuality::Griefed,
            category: "TNT Detectada".into(),
            comments: vec![
                "ala, tnt no mapa, quem eh o griefer?".into(),
                "se alguem explodir minha base eu juro".into(),
                "report nisso ae".into(),
            ],
        });
    }

    judgments
}

#[derive(Debug, Clone)]
pub struct BuildJudgment {
    pub quality: BuildQuality,
    pub category: String,
    pub comments: Vec<String>,
}

impl BuildJudgment {
    /// Pick a random comment from this judgment
    pub fn random_comment(&self) -> &str {
        let mut rng = rand::thread_rng();
        let idx = rng.r#gen::<usize>() % self.comments.len();
        &self.comments[idx]
    }
}

/// Decide if the bot should comment on something it sees
pub fn should_comment(judgments: &[BuildJudgment]) -> Option<&BuildJudgment> {
    let mut rng = rand::thread_rng();

    for judgment in judgments {
        let comment_chance = match judgment.quality {
            BuildQuality::Noob => 0.7,        // Almost always roast
            BuildQuality::Griefed => 0.9,      // Always comment on grief
            BuildQuality::Masterpiece => 0.5,  // Sometimes compliment
            BuildQuality::Mediocre => 0.3,     // Occasional "meh"
            BuildQuality::Decent => 0.2,       // Rarely comment on OK builds
        };

        if rng.r#gen::<f32>() < comment_chance {
            return Some(judgment);
        }
    }
    None
}
