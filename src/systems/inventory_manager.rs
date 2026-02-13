use azalea::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;

// ============================================================
// INVENTORY MANAGER — Hotbar OCD + Chest Organization
// ============================================================

/// Ideal hotbar layout (slot 0-8)
/// Slot 0: Sword
/// Slot 1: Pickaxe
/// Slot 2: Axe
/// Slot 3: Shovel
/// Slot 4: Bow / Crossbow
/// Slot 5: Building blocks
/// Slot 6: Torch
/// Slot 7: Food (secondary)
/// Slot 8: Food (primary)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotbarPreference {
    pub slot_0: ItemCategory, // Sword
    pub slot_1: ItemCategory, // Pickaxe
    pub slot_2: ItemCategory, // Axe
    pub slot_3: ItemCategory, // Shovel
    pub slot_4: ItemCategory, // Ranged
    pub slot_5: ItemCategory, // Blocks
    pub slot_6: ItemCategory, // Torch
    pub slot_7: ItemCategory, // Food
    pub slot_8: ItemCategory, // Food
}

impl Default for HotbarPreference {
    fn default() -> Self {
        Self {
            slot_0: ItemCategory::Sword,
            slot_1: ItemCategory::Pickaxe,
            slot_2: ItemCategory::Axe,
            slot_3: ItemCategory::Shovel,
            slot_4: ItemCategory::Ranged,
            slot_5: ItemCategory::BuildingBlock,
            slot_6: ItemCategory::Torch,
            slot_7: ItemCategory::Food,
            slot_8: ItemCategory::Food,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ItemCategory {
    Sword,
    Pickaxe,
    Axe,
    Shovel,
    Ranged,       // Bow, Crossbow, Trident
    BuildingBlock,
    Torch,
    Food,
    Valuable,     // Diamonds, emeralds, enchanted items
    Junk,         // Dirt, gravel, rotten flesh
    Redstone,
    Armor,
    Tool,
    Other,
}

/// Categorize an item name
pub fn categorize_item(item_name: &str) -> ItemCategory {
    let name = item_name.to_lowercase();

    if name.contains("sword") { return ItemCategory::Sword; }
    if name.contains("pickaxe") { return ItemCategory::Pickaxe; }
    if name.contains("axe") && !name.contains("pickaxe") { return ItemCategory::Axe; }
    if name.contains("shovel") { return ItemCategory::Shovel; }
    if name.contains("bow") || name.contains("crossbow") || name.contains("trident") {
        return ItemCategory::Ranged;
    }
    if name.contains("torch") { return ItemCategory::Torch; }

    // Food items
    let foods = [
        "apple", "bread", "cooked", "steak", "porkchop", "chicken",
        "mutton", "rabbit", "salmon", "cod", "carrot", "potato",
        "melon_slice", "sweet_berries", "golden_apple", "cake",
    ];
    if foods.iter().any(|f| name.contains(f)) { return ItemCategory::Food; }

    // Valuables
    let valuables = [
        "diamond", "emerald", "gold_ingot", "iron_ingot", "netherite",
        "enchanted", "elytra", "totem", "shulker",
    ];
    if valuables.iter().any(|v| name.contains(v)) { return ItemCategory::Valuable; }

    // Junk
    let junk = [
        "dirt", "gravel", "rotten_flesh", "poisonous_potato",
        "dead_bush", "stick", "feather", "string", "bone",
    ];
    if junk.iter().any(|j| name.contains(j)) { return ItemCategory::Junk; }

    // Redstone
    let redstone = [
        "redstone", "repeater", "comparator", "piston", "observer",
        "hopper", "dropper", "dispenser", "lever", "button",
    ];
    if redstone.iter().any(|r| name.contains(r)) { return ItemCategory::Redstone; }

    // Armor
    if name.contains("helmet") || name.contains("chestplate")
        || name.contains("leggings") || name.contains("boots")
    {
        return ItemCategory::Armor;
    }

    // Building blocks (common ones)
    let blocks = [
        "planks", "log", "stone", "cobblestone", "brick", "slab",
        "stair", "glass", "wool", "concrete", "terracotta",
    ];
    if blocks.iter().any(|b| name.contains(b)) { return ItemCategory::BuildingBlock; }

    ItemCategory::Other
}

/// Chest sorting categories and their sort order
pub fn chest_sort_order(cat: &ItemCategory) -> u8 {
    match cat {
        ItemCategory::Valuable => 0,
        ItemCategory::Armor => 1,
        ItemCategory::Sword => 2,
        ItemCategory::Pickaxe => 3,
        ItemCategory::Axe => 4,
        ItemCategory::Shovel => 5,
        ItemCategory::Ranged => 6,
        ItemCategory::Tool => 7,
        ItemCategory::Redstone => 8,
        ItemCategory::Food => 9,
        ItemCategory::Torch => 10,
        ItemCategory::BuildingBlock => 11,
        ItemCategory::Other => 12,
        ItemCategory::Junk => 13,
    }
}

/// Generate a snarky comment about messy chests
pub fn chest_comment(items: &[String]) -> Option<String> {
    use rand::Rng;
    let mut rng = rand::thread_rng();

    let has_valuable = items.iter().any(|i| categorize_item(i) == ItemCategory::Valuable);
    let has_junk = items.iter().any(|i| categorize_item(i) == ItemCategory::Junk);

    if has_valuable && has_junk {
        let comments = [
            "mano, quem foi o animal que misturou terra com diamante? arrumei aqui",
            "pqp diamante junto com rotten flesh sério?? organizei",
            "q bagunça nesse bau, parece inventario de noob. arrumei",
            "ce guarda diamante do lado de dirt?? q trauma",
        ];
        return Some(comments[rng.r#gen::<usize>() % comments.len()].to_string());
    }

    if has_junk && items.len() > 15 {
        let comments = [
            "bau de entulho detectado, limpei esse lixo",
            "mn esse bau parece deposito de lixo",
        ];
        return Some(comments[rng.r#gen::<usize>() % comments.len()].to_string());
    }

    None
}

#[derive(Clone, Component)]
pub struct State {
    pub hotbar_pref: Arc<Mutex<HotbarPreference>>,
    pub last_sort: Arc<Mutex<Instant>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            hotbar_pref: Arc::new(Mutex::new(HotbarPreference::default())),
            last_sort: Arc::new(Mutex::new(Instant::now())),
        }
    }
}

pub async fn handle(_bot: Client, event: Event, _state: State) -> anyhow::Result<()> {
    if let Event::Tick = event {
        // In a real implementation:
        // 1. Check if hotbar matches preferences
        // 2. If not, swap items to correct slots
        // 3. If a chest is open, sort it by category
        // Azalea's inventory API would be used here.
    }
    Ok(())
}
