use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
};

use crate::grid::{Grid2D, GridConfig, TileId};

#[derive(Copy, Clone, Eq, PartialEq)]
struct Node {
    tile: TileId,
    g: u32,
    h: u32,
}

impl Node {
    fn f(&self) -> u32 {
        self.g + self.h
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap behaviour using BinaryHeap (which is max-heap)
        other
            .f()
            .cmp(&self.f())
            .then_with(|| other.h.cmp(&self.h))
            .then_with(|| other.tile.y.cmp(&self.tile.y))
            .then_with(|| other.tile.x.cmp(&self.tile.x))
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn heuristic(a: TileId, b: TileId) -> u32 {
    let dx = (a.x - b.x).unsigned_abs();
    let dy = (a.y - b.y).unsigned_abs();
    let min = dx.min(dy);
    let max = dx.max(dy);
    14 * min + 10 * (max - min)
}

/// 8-way deterministic A* over a walkability grid.
pub fn astar(
    grid: &Grid2D<bool>,
    start: TileId,
    goal: TileId,
) -> Option<Vec<TileId>> {
    let cfg = grid.cfg;
    if !cfg.in_bounds(start) || !cfg.in_bounds(goal) {
        return None;
    }

    let mut open = BinaryHeap::new();
    let mut came_from: HashMap<TileId, TileId> = HashMap::new();
    let mut g_score: HashMap<TileId, u32> = HashMap::new();

    g_score.insert(start, 0);
    open.push(Node {
        tile: start,
        g: 0,
        h: heuristic(start, goal),
    });

    while let Some(current) = open.pop() {
        if current.tile == goal {
            // reconstruct
            let mut path = vec![goal];
            let mut cur = goal;
            while let Some(prev) = came_from.get(&cur) {
                path.push(*prev);
                cur = *prev;
            }
            path.reverse();
            return Some(path);
        }
        let curr_g = g_score[&current.tile];
        for n in neighbors(cfg, current.tile) {
            if !grid.get(n).copied().unwrap_or(false) {
                continue;
            }
            let tentative = curr_g + step_cost(current.tile, n);
            if tentative < *g_score.get(&n).unwrap_or(&u32::MAX) {
                came_from.insert(n, current.tile);
                g_score.insert(n, tentative);
                open.push(Node {
                    tile: n,
                    g: tentative,
                    h: heuristic(n, goal),
                });
            }
        }
    }
    None
}

fn step_cost(a: TileId, b: TileId) -> u32 {
    if a.x == b.x || a.y == b.y {
        10
    } else {
        14
    }
}

fn neighbors(cfg: GridConfig, tile: TileId) -> Vec<TileId> {
    let mut out = Vec::with_capacity(8);
    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }
            let n = TileId::new(tile.x + dx, tile.y + dy);
            if cfg.in_bounds(n) {
                out.push(n);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_path() {
        let cfg = GridConfig {
            width: 5,
            height: 5,
        };
        let mut walk = Grid2D::new(cfg, true);
        // wall column at x=1 except opening at y=4
        *walk.get_mut(TileId::new(1, 0)).unwrap() = false;
        *walk.get_mut(TileId::new(1, 1)).unwrap() = false;
        *walk.get_mut(TileId::new(1, 2)).unwrap() = false;
        *walk.get_mut(TileId::new(1, 3)).unwrap() = false;

        let start = TileId::new(0, 0);
        let goal = TileId::new(4, 4);
        let p1 = astar(&walk, start, goal).unwrap();
        let p2 = astar(&walk, start, goal).unwrap();
        assert_eq!(p1, p2);
        assert_eq!(p1.first().copied(), Some(start));
        assert_eq!(p1.last().copied(), Some(goal));
    }
}
