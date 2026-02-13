use serde::{Deserialize, Serialize};
use azalea::BlockPos;

// ============================================================
// FARMER — Automated farming knowledge
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CropType {
    Wheat,
    Carrot,
    Potato,
    Beetroot,
    SugarCane,
    Melon,
    Pumpkin,
    Bamboo,
    NetherWart,
}

impl CropType {
    pub fn growth_stages(&self) -> u8 {
        match self {
            CropType::Wheat | CropType::Carrot | CropType::Potato | CropType::Beetroot => 7,
            CropType::SugarCane | CropType::Bamboo => 3, // Harvest at 3-tall
            CropType::Melon | CropType::Pumpkin => 7,
            CropType::NetherWart => 3,
        }
    }

    pub fn needs_water(&self) -> bool {
        match self {
            CropType::SugarCane => true, // Adjacent water
            CropType::NetherWart => false,
            CropType::Bamboo => false,
            _ => true, // Farmland needs water within 4 blocks
        }
    }

    pub fn seed_name(&self) -> &str {
        match self {
            CropType::Wheat => "wheat_seeds",
            CropType::Carrot => "carrot",
            CropType::Potato => "potato",
            CropType::Beetroot => "beetroot_seeds",
            CropType::SugarCane => "sugar_cane",
            CropType::Melon => "melon_seeds",
            CropType::Pumpkin => "pumpkin_seeds",
            CropType::Bamboo => "bamboo",
            CropType::NetherWart => "nether_wart",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FarmPlot {
    pub origin: [i32; 3],
    pub crop: CropType,
    pub size: [i32; 2], // width, depth
    pub last_harvest: Option<chrono::DateTime<chrono::Utc>>,
    pub total_harvests: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FarmState {
    Idle,
    Planting,
    WaitingForGrowth,
    Harvesting,
    Replanting,
    LookingForSeeds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Farmer {
    pub state: FarmState,
    pub farms: Vec<FarmPlot>,
    pub current_farm_index: Option<usize>,
    pub crops_harvested: u32,
    pub food_produced: u32,
}

impl Default for Farmer {
    fn default() -> Self {
        Self {
            state: FarmState::Idle,
            farms: vec![],
            current_farm_index: None,
            crops_harvested: 0,
            food_produced: 0,
        }
    }
}

impl Farmer {
    /// Register a new farm
    pub fn register_farm(&mut self, origin: [i32; 3], crop: CropType, size: [i32; 2]) {
        println!("[FARMER] 🌾 Registered {:?} farm at {:?} ({}x{})", crop, origin, size[0], size[1]);
        self.farms.push(FarmPlot {
            origin,
            crop,
            size,
            last_harvest: None,
            total_harvests: 0,
        });
    }

    /// Get blocks that need planting in a farm
    pub fn get_planting_positions(&self, farm_index: usize) -> Vec<BlockPos> {
        let farm = match self.farms.get(farm_index) {
            Some(f) => f,
            None => return vec![],
        };

        let mut positions = vec![];
        for x in 0..farm.size[0] {
            for z in 0..farm.size[1] {
                // Skip water center for standard 9x9
                if farm.size[0] == 9 && farm.size[1] == 9 && x == 4 && z == 4 {
                    continue;
                }
                positions.push(BlockPos::new(
                    farm.origin[0] + x,
                    farm.origin[1],
                    farm.origin[2] + z,
                ));
            }
        }
        positions
    }

    /// Get blocks to harvest (fully grown crops)
    pub fn get_harvest_positions(&self, farm_index: usize) -> Vec<BlockPos> {
        // In reality, we'd check block state for growth stage
        // For now, return all crop positions
        self.get_planting_positions(farm_index)
    }

    pub fn record_harvest(&mut self, farm_index: usize) {
        if let Some(farm) = self.farms.get_mut(farm_index) {
            farm.total_harvests += 1;
            farm.last_harvest = Some(chrono::Utc::now());
        }
        self.crops_harvested += 1;
        self.food_produced += 1;
    }

    /// Should we check farms? (every 5 min after last harvest)
    pub fn should_check_farms(&self) -> bool {
        if self.farms.is_empty() {
            return false;
        }
        // Check if any farm hasn't been harvested in 5+ minutes
        self.farms.iter().any(|f| {
            f.last_harvest
                .map(|t| chrono::Utc::now().signed_duration_since(t).num_seconds() > 300)
                .unwrap_or(true)
        })
    }

    pub fn context_summary(&self) -> String {
        if self.farms.is_empty() {
            return "Não tenho farms ainda.".into();
        }
        let mut s = format!("{} farms registradas. ", self.farms.len());
        for (i, farm) in self.farms.iter().enumerate() {
            s.push_str(&format!(
                "Farm {}: {:?} ({}x{}, {} colheitas). ",
                i, farm.crop, farm.size[0], farm.size[1], farm.total_harvests
            ));
        }
        s.push_str(&format!("Total colhido: {}", self.crops_harvested));
        s
    }
}
