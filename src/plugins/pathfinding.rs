use azalea::prelude::*;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::cmp::Ordering;
use azalea::BlockPos;

// Simple Node struct for A*
#[derive(Clone, Copy, Eq, PartialEq)]
struct Node {
    pos: BlockPos,
    cost: u32,
    heuristic: u32,
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse because BinaryHeap is max-heap
        (other.cost + other.heuristic).cmp(&(self.cost + self.heuristic))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Pathfinder;

impl Pathfinder {
    pub fn compute_path(start: BlockPos, end: BlockPos) -> Option<Vec<BlockPos>> {
        // Very simplified A* for now (Manhattan distance, only horizontal moves + 1 up/down)
        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<BlockPos, BlockPos> = HashMap::new();
        let mut g_score: HashMap<BlockPos, u32> = HashMap::new();

        g_score.insert(start, 0);
        open_set.push(Node {
            pos: start,
            cost: 0,
            heuristic: Self::heuristic(start, end),
        });

        let mut visited = HashSet::new();

        while let Some(current) = open_set.pop() {
            if current.pos == end {
                return Some(Self::reconstruct_path(came_from, current.pos));
            }

            if !visited.insert(current.pos) {
                continue;
            }
            
            // Limit search depth/nodes to avoid lag
            if visited.len() > 1000 {
                return None;
            }

            for neighbor in Self::get_neighbors(current.pos) {
                 // Check if neighbor is passable (requires world access which we don't have easily in this static function)
                 // For this POC, we assume air. In reality, we need `bot.world().read()` access.
                 // This is a placeholder for the actual pathfinding logic.
                 
                 let tentative_g_score = g_score.get(&current.pos).unwrap() + 1;
                 if tentative_g_score < *g_score.get(&neighbor).unwrap_or(&u32::MAX) {
                     came_from.insert(neighbor, current.pos);
                     g_score.insert(neighbor, tentative_g_score);
                     open_set.push(Node {
                         pos: neighbor,
                         cost: tentative_g_score,
                         heuristic: Self::heuristic(neighbor, end),
                     });
                 }
            }
        }
        
        None
    }

    fn heuristic(a: BlockPos, b: BlockPos) -> u32 {
        ((a.x - b.x).abs() + (a.y - b.y).abs() + (a.z - b.z).abs()) as u32
    }
    
    fn get_neighbors(pos: BlockPos) -> Vec<BlockPos> {
        let offsets = [
            (1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1),
            (1, 1, 0), (1, -1, 0)
        ];
        offsets.iter().map(|(dx, dy, dz)| {
            BlockPos::new(pos.x + dx, pos.y + dy, pos.z + dz)
        }).collect()
    }

    fn reconstruct_path(mut came_from: HashMap<BlockPos, BlockPos>, mut current: BlockPos) -> Vec<BlockPos> {
        let mut path = vec![current];
        while let Some(prev) = came_from.remove(&current) {
            current = prev;
            path.push(current);
        }
        path.reverse();
        path
    }
}

// Public helper to be called from bot state
pub async fn goto_block(bot: Client, target: BlockPos) {
    let start = bot.position().into(); // approximate to BlockPos
    if let Some(path) = Pathfinder::compute_path(start, target) {
        println!("Path found with {} steps", path.len());
        for step in path {
             // bot.look_at(step.center());
             println!("Walking to {:?}", step); 
             // bot.walk_start();
             // In reality we need to wait until we reach the block.
             // This is a naive implementation that just enables walking.
             // Real implementation requires a tick loop.
             tokio::time::sleep(std::time::Duration::from_millis(300)).await; 
             // bot.walk_stop();
        }
    } else {
        println!("No path found to {:?}", target);
    }
}
