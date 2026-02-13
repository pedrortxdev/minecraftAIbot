// use azalea::prelude::*;
use azalea::BlockPos;
use serde::{Deserialize, Serialize};
use rand::Rng;

// ============================================================
// SMART MINING — Veteran mining strategies
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MiningStrategy {
    StripMine,       // Y=-59 (bedrock) to Y=16, 2-block high tunnels
    BranchMine,      // Main corridor + branches every 4 blocks
    CaveExploration, // Follow exposed caves, mine visible ores
    Quarry,          // Layer-by-layer excavation for bulk materials
    TreeFarm,        // Chop trees, replant
    SurfaceGather,   // Quick surface resource collection (dirt, sand, gravel)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MiningTarget {
    Coal,
    Iron,
    Gold,
    Diamond,
    Redstone,
    Lapis,
    Emerald,
    Copper,
    AncientDebris,
    Wood,
    Stone,
    Any,
}

impl MiningTarget {
    /// Optimal Y level for this resource (1.21+)
    pub fn optimal_y(&self) -> i32 {
        match self {
            MiningTarget::Diamond => -59,
            MiningTarget::Iron => 16,
            MiningTarget::Gold => -16,
            MiningTarget::Redstone => -59,
            MiningTarget::Lapis => 0,
            MiningTarget::Emerald => 100, // Mountains only
            MiningTarget::Copper => 48,
            MiningTarget::Coal => 96,
            MiningTarget::AncientDebris => 15, // Nether
            MiningTarget::Wood => 64,    // Surface
            MiningTarget::Stone => 30,
            MiningTarget::Any => -59,
        }
    }

    /// Best strategy for this target
    pub fn best_strategy(&self) -> MiningStrategy {
        match self {
            MiningTarget::Diamond | MiningTarget::Redstone => MiningStrategy::StripMine,
            MiningTarget::Iron | MiningTarget::Gold | MiningTarget::Copper => MiningStrategy::BranchMine,
            MiningTarget::Wood => MiningStrategy::TreeFarm,
            MiningTarget::Stone => MiningStrategy::Quarry,
            _ => MiningStrategy::CaveExploration,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartMiner {
    pub current_strategy: Option<MiningStrategy>,
    pub current_target: MiningTarget,
    pub mining_origin: Option<[i32; 3]>,
    pub tunnel_direction: i32, // 0=north, 1=east, 2=south, 3=west
    pub tunnel_progress: i32,
    pub ores_found: u32,
    pub blocks_mined: u32,
    pub efficiency_score: f32, // ores_found / blocks_mined
}

impl Default for SmartMiner {
    fn default() -> Self {
        Self {
            current_strategy: None,
            current_target: MiningTarget::Any,
            mining_origin: None,
            tunnel_direction: 0,
            tunnel_progress: 0,
            ores_found: 0,
            blocks_mined: 0,
            efficiency_score: 0.0,
        }
    }
}

impl SmartMiner {
    /// Start a mining session for a specific target
    pub fn start_mining(&mut self, target: MiningTarget, current_pos: [i32; 3]) {
        let strategy = target.best_strategy();
        let y_target = target.optimal_y();
        println!(
            "[MINER] 🪨 Starting {:?} for {:?}. Target Y: {}. Current Y: {}",
            strategy, target, y_target, current_pos[1]
        );
        self.current_strategy = Some(strategy);
        self.current_target = target;
        self.mining_origin = Some(current_pos);
        self.tunnel_direction = rand::thread_rng().gen_range(0..4);
        self.tunnel_progress = 0;
        self.ores_found = 0;
        self.blocks_mined = 0;
    }

    /// Get next block to mine based on strategy
    pub fn next_block_to_mine(&mut self) -> Option<BlockPos> {
        let origin = self.mining_origin?;
        let strategy = self.current_strategy.as_ref()?;

        let pos = match strategy {
            MiningStrategy::StripMine => {
                // 1x2 tunnel in tunnel_direction
                let (dx, dz) = match self.tunnel_direction {
                    0 => (0, -1), // North
                    1 => (1, 0),  // East
                    2 => (0, 1),  // South
                    _ => (-1, 0), // West
                };
                self.tunnel_progress += 1;
                Some(BlockPos::new(
                    origin[0] + dx * self.tunnel_progress,
                    self.current_target.optimal_y(),
                    origin[2] + dz * self.tunnel_progress,
                ))
            }
            MiningStrategy::BranchMine => {
                // Main tunnel + branch every 4 blocks
                let branch = self.tunnel_progress % 8 >= 4;
                let main_progress = self.tunnel_progress / 8;
                let branch_offset = self.tunnel_progress % 4;

                let (dx, dz) = match self.tunnel_direction {
                    0 => (0, -1),
                    1 => (1, 0),
                    2 => (0, 1),
                    _ => (-1, 0),
                };

                self.tunnel_progress += 1;

                if branch {
                    // Branch goes perpendicular
                    Some(BlockPos::new(
                        origin[0] + dx * main_progress + dz * branch_offset,
                        self.current_target.optimal_y(),
                        origin[2] + dz * main_progress + dx * branch_offset,
                    ))
                } else {
                    // Main tunnel
                    Some(BlockPos::new(
                        origin[0] + dx * self.tunnel_progress,
                        self.current_target.optimal_y(),
                        origin[2] + dz * self.tunnel_progress,
                    ))
                }
            }
            _ => {
                self.tunnel_progress += 1;
                Some(BlockPos::new(origin[0], origin[1], origin[2]))
            }
        };

        if self.tunnel_progress > 200 {
            println!("[MINER] Tunnel complete (200 blocks). Switching direction.");
            self.tunnel_direction = (self.tunnel_direction + 1) % 4;
            self.tunnel_progress = 0;
        }

        pos
    }

    pub fn record_ore_found(&mut self) {
        self.ores_found += 1;
        self.blocks_mined += 1;
        self.update_efficiency();
    }

    pub fn record_block_mined(&mut self) {
        self.blocks_mined += 1;
        self.update_efficiency();
    }

    fn update_efficiency(&mut self) {
        if self.blocks_mined > 0 {
            self.efficiency_score = self.ores_found as f32 / self.blocks_mined as f32;
        }
    }

    pub fn context_summary(&self) -> String {
        match &self.current_strategy {
            Some(s) => format!(
                "Minerando: {:?} via {:?}. Progresso: {} blocos, {} minérios achados (eficiência: {:.1}%)",
                self.current_target, s, self.blocks_mined, self.ores_found, self.efficiency_score * 100.0
            ),
            None => "Não estou minerando no momento.".into(),
        }
    }
}
