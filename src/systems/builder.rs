use serde::{Deserialize, Serialize};
use azalea::BlockPos;
use std::collections::HashMap;

// ============================================================
// BUILDER — Blueprint-based construction
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockPlacement {
    pub offset: [i32; 3], // Relative to blueprint origin
    pub block: String,     // e.g., "oak_planks", "cobblestone", "glass"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blueprint {
    pub name: String,
    pub description: String,
    pub size: [i32; 3], // width, height, depth
    pub blocks: Vec<BlockPlacement>,
    pub required_materials: HashMap<String, u32>,
    pub build_order: Vec<usize>, // Indices into blocks, bottom-up layer order
}

impl Blueprint {
    /// Create a simple 5x4x5 survival house
    pub fn survival_house() -> Self {
        let mut blocks = vec![];
        let mut materials: HashMap<String, u32> = HashMap::new();

        // Floor (5x5 at y=0)
        for x in 0..5 {
            for z in 0..5 {
                blocks.push(BlockPlacement {
                    offset: [x, 0, z],
                    block: "oak_planks".into(),
                });
                *materials.entry("oak_planks".into()).or_insert(0) += 1;
            }
        }

        // Walls (y=1 to y=3, perimeter only)
        for y in 1..=3 {
            for x in 0..5 {
                for z in 0..5 {
                    if x == 0 || x == 4 || z == 0 || z == 4 {
                        // Door opening at (2, 1..2, 0)
                        if z == 0 && x == 2 && y <= 2 {
                            continue;
                        }
                        // Window at (0, 2, 2) and (4, 2, 2)
                        if y == 2 && ((x == 0 && z == 2) || (x == 4 && z == 2)) {
                            blocks.push(BlockPlacement {
                                offset: [x, y, z],
                                block: "glass_pane".into(),
                            });
                            *materials.entry("glass_pane".into()).or_insert(0) += 1;
                            continue;
                        }
                        blocks.push(BlockPlacement {
                            offset: [x, y, z],
                            block: "cobblestone".into(),
                        });
                        *materials.entry("cobblestone".into()).or_insert(0) += 1;
                    }
                }
            }
        }

        // Roof (5x5 at y=4)
        for x in 0..5 {
            for z in 0..5 {
                blocks.push(BlockPlacement {
                    offset: [x, 4, z],
                    block: "oak_slab".into(),
                });
                *materials.entry("oak_slab".into()).or_insert(0) += 1;
            }
        }

        // Build order: bottom to top
        let build_order: Vec<usize> = (0..blocks.len()).collect();

        Blueprint {
            name: "Casa de Sobrevivência".into(),
            description: "Casa 5x5 básica com porta, janelas e teto".into(),
            size: [5, 5, 5],
            blocks,
            required_materials: materials,
            build_order,
        }
    }

    /// Create a 9x1x9 wheat farm with water center
    pub fn wheat_farm() -> Self {
        let mut blocks = vec![];
        let mut materials: HashMap<String, u32> = HashMap::new();

        for x in 0..9 {
            for z in 0..9 {
                if x == 4 && z == 4 {
                    // Water source in center
                    blocks.push(BlockPlacement {
                        offset: [x, 0, z],
                        block: "water".into(),
                    });
                } else {
                    blocks.push(BlockPlacement {
                        offset: [x, 0, z],
                        block: "farmland".into(),
                    });
                    *materials.entry("wheat_seeds".into()).or_insert(0) += 1;
                }
            }
        }

        let build_order: Vec<usize> = (0..blocks.len()).collect();

        Blueprint {
            name: "Farm de Trigo 9x9".into(),
            description: "Farm padrão com água no centro, 80 plots".into(),
            size: [9, 1, 9],
            blocks,
            required_materials: materials,
            build_order,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BuildState {
    Idle,
    GatheringMaterials,
    Placing,
    Finished,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Builder {
    pub state: BuildState,
    pub current_blueprint: Option<Blueprint>,
    pub build_origin: Option<[i32; 3]>,
    pub blocks_placed: usize,
    pub total_blocks: usize,
    pub builds_completed: u32,
    pub available_blueprints: Vec<String>,
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            state: BuildState::Idle,
            current_blueprint: None,
            build_origin: None,
            blocks_placed: 0,
            total_blocks: 0,
            builds_completed: 0,
            available_blueprints: vec![
                "Casa de Sobrevivência".into(),
                "Farm de Trigo 9x9".into(),
                "Sala de Storage".into(),
                "Torre de Vigia".into(),
                "Sala de Encantamento".into(),
            ],
        }
    }
}

impl Builder {
    pub fn start_build(&mut self, blueprint: Blueprint, origin: [i32; 3]) {
        println!("[BUILDER] 🏗 Starting: {} at {:?}", blueprint.name, origin);
        println!("[BUILDER] Materials needed:");
        for (mat, count) in &blueprint.required_materials {
            println!("  - {} x{}", mat, count);
        }
        self.total_blocks = blueprint.blocks.len();
        self.blocks_placed = 0;
        self.current_blueprint = Some(blueprint);
        self.build_origin = Some(origin);
        self.state = BuildState::GatheringMaterials;
    }

    /// Get the next block to place
    pub fn next_placement(&self) -> Option<(BlockPos, &str)> {
        let blueprint = self.current_blueprint.as_ref()?;
        let origin = self.build_origin?;

        if self.blocks_placed >= blueprint.blocks.len() {
            return None;
        }

        let idx = blueprint.build_order.get(self.blocks_placed)?;
        let placement = blueprint.blocks.get(*idx)?;

        let pos = BlockPos::new(
            origin[0] + placement.offset[0],
            origin[1] + placement.offset[1],
            origin[2] + placement.offset[2],
        );

        Some((pos, &placement.block))
    }

    pub fn record_placement(&mut self) {
        self.blocks_placed += 1;
        if self.blocks_placed >= self.total_blocks {
            self.state = BuildState::Finished;
            self.builds_completed += 1;
            if let Some(bp) = &self.current_blueprint {
                println!("[BUILDER] ✅ Build complete: {}", bp.name);
            }
        }
    }

    pub fn context_summary(&self) -> String {
        match self.state {
            BuildState::Idle => "Não estou construindo nada.".into(),
            BuildState::GatheringMaterials => {
                format!("Juntando materiais pra: {}", 
                    self.current_blueprint.as_ref().map(|b| b.name.as_str()).unwrap_or("?"))
            }
            BuildState::Placing => {
                format!("Construindo: {} ({}/{} blocos)",
                    self.current_blueprint.as_ref().map(|b| b.name.as_str()).unwrap_or("?"),
                    self.blocks_placed, self.total_blocks)
            }
            BuildState::Finished => format!("Terminei! {} builds completos.", self.builds_completed),
            BuildState::Paused => "Build pausado.".into(),
        }
    }
}
