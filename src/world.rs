//! Grid simulation, intentionally independent of rendering and Bevy.
use std::collections::{HashMap, HashSet};

pub const CHUNK_SIZE: i64 = 16;
pub const ACTIVE_RADIUS: i64 = 24;
pub const STEP_SECONDS: f64 = 0.12;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cell {
    pub x: i64,
    pub y: i64,
}

impl Cell {
    pub const fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }

    pub fn offset(self, x: i64, y: i64) -> Self {
        Self::new(self.x + x, self.y + y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    Empty,
    Dirt,
    Stone,
}

#[derive(Clone, Copy, Debug)]
pub struct StoneMotion {
    pub from: Cell,
    pub to: Cell,
}

pub struct World {
    pub seed: u64,
    chunks: HashSet<Cell>,
    tiles: HashMap<Cell, Tile>,
    physics_tick: u64,
    pub player: Cell,
    pub previous_player: Cell,
    pub stone_motion: Vec<StoneMotion>,
    pub dead: bool,
}

impl World {
    pub fn new(seed: u64) -> Self {
        let mut world = Self {
            seed,
            chunks: HashSet::new(),
            tiles: HashMap::new(),
            physics_tick: 0,
            player: Cell::default(),
            previous_player: Cell::default(),
            stone_motion: Vec::new(),
            dead: false,
        };
        world.ensure_area(world.player, ACTIVE_RADIUS + 2);
        // A safe pocket with dirt supporting the surrounding stones.
        for y in -1..=1 {
            for x in -2..=2 {
                world.tiles.insert(Cell::new(x, y), Tile::Empty);
            }
        }
        for x in -2..=2 {
            world.tiles.insert(Cell::new(x, 2), Tile::Dirt);
        }
        world
    }

    /// None means unknown, which acts as solid at the simulation boundary.
    pub fn tile(&self, cell: Cell) -> Option<Tile> {
        self.tiles.get(&cell).copied()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn ensure_area(&mut self, center: Cell, radius: i64) {
        let min_x = (center.x - radius).div_euclid(CHUNK_SIZE);
        let max_x = (center.x + radius).div_euclid(CHUNK_SIZE);
        let min_y = (center.y - radius).div_euclid(CHUNK_SIZE);
        let max_y = (center.y + radius).div_euclid(CHUNK_SIZE);
        for cy in min_y..=max_y {
            for cx in min_x..=max_x {
                if !self.chunks.insert(Cell::new(cx, cy)) {
                    continue;
                }
                for y in 0..CHUNK_SIZE {
                    for x in 0..CHUNK_SIZE {
                        let cell = Cell::new(cx * CHUNK_SIZE + x, cy * CHUNK_SIZE + y);
                        let tile = if cell_hash(self.seed, cell) % 100 < 18 {
                            Tile::Stone
                        } else {
                            Tile::Dirt
                        };
                        self.tiles.insert(cell, tile);
                    }
                }
            }
        }
    }

    pub fn step(&mut self, direction: Cell) {
        self.previous_player = self.player;
        if self.dead {
            self.stone_motion.clear();
            return;
        }
        // A stone can crush the player only after falling through an empty
        // cell on the previous tick. Resting stones are supported by the player.
        let falling: HashSet<_> = std::mem::take(&mut self.stone_motion)
            .into_iter()
            .filter(|motion| motion.to.y < motion.from.y)
            .map(|motion| motion.to)
            .collect();
        self.physics_tick = self.physics_tick.wrapping_add(1);
        self.ensure_area(self.player, ACTIVE_RADIUS + 2);
        let mut pushed = None;
        if direction.x.abs() + direction.y.abs() == 1 {
            let destination = self.player.offset(direction.x, direction.y);
            if matches!(self.tile(destination), Some(Tile::Empty | Tile::Dirt)) {
                self.tiles.insert(destination, Tile::Empty);
                self.player = destination;
            } else if direction.y == 0 && self.tile(destination) == Some(Tile::Stone) {
                let beyond = destination.offset(direction.x, 0);
                if self.tile(beyond) == Some(Tile::Empty) {
                    self.tiles.insert(destination, Tile::Empty);
                    self.tiles.insert(beyond, Tile::Stone);
                    self.player = destination;
                    pushed = Some(StoneMotion {
                        from: destination,
                        to: beyond,
                    });
                }
            }
        }
        // Generate before physics, including a halo beyond the active region.
        self.ensure_area(self.player, ACTIVE_RADIUS + 2);
        let mut stones = Vec::new();
        for y in self.player.y - ACTIVE_RADIUS..=self.player.y + ACTIVE_RADIUS {
            for x in self.player.x - ACTIVE_RADIUS..=self.player.x + ACTIVE_RADIUS {
                let cell = Cell::new(x, y);
                // A pushed stone already used its one-cell move this tick.
                if self.tile(cell) == Some(Tile::Stone)
                    && pushed.as_ref().is_none_or(|motion| motion.to != cell)
                {
                    stones.push(cell);
                }
            }
        }
        // Decide all moves from the same snapshot. A stone moves at most one
        // cell per tick; upper stones wait until the next tick to follow.
        let mut moves: Vec<_> = stones
            .iter()
            .filter_map(|&from| {
                let to = from.offset(0, -1);
                let leaving_player =
                    self.player != self.previous_player && to == self.previous_player;
                let supported_by_player = to == self.player && !falling.contains(&from);
                (self.tile(to) == Some(Tile::Empty) && !leaving_player && !supported_by_player)
                    .then_some(StoneMotion { from, to })
            })
            .collect();
        moves.extend(pushed);
        // Falling has priority over slipping. Reserve destinations so a rolling
        // stone cannot collide with a falling stone or another rolling stone.
        let mut reserved: HashSet<_> = moves.iter().map(|motion| motion.to).collect();
        for from in stones {
            if self.tile(from.offset(0, -1)) != Some(Tile::Stone) {
                continue;
            }
            let preferred = if cell_hash(self.seed.wrapping_add(self.physics_tick), from) & 1 == 0 {
                -1
            } else {
                1
            };
            for dx in [preferred, -preferred] {
                let side = from.offset(dx, 0);
                let below_side = side.offset(0, -1);
                let clear = [side, below_side].into_iter().all(|cell| {
                    self.tile(cell) == Some(Tile::Empty)
                        && cell != self.player
                        && cell != self.previous_player
                        && !reserved.contains(&cell)
                });
                if clear {
                    // Slide horizontally first, then fall on a later tick.
                    // Horizontal motion alone does not give crushing momentum.
                    reserved.insert(side);
                    moves.push(StoneMotion { from, to: side });
                    break;
                }
            }
        }
        for motion in &moves {
            self.tiles.insert(motion.from, Tile::Empty);
            self.tiles.insert(motion.to, Tile::Stone);
            if motion.to == self.player {
                self.dead = true;
            }
        }
        self.stone_motion = moves;
    }
}

/// Coordinate hashing makes generation independent of exploration order,
/// including across negative coordinates. No external RNG state is needed.
pub fn cell_hash(seed: u64, cell: Cell) -> u64 {
    let mut value = seed
        ^ (cell.x as u64).wrapping_mul(0x9e3779b97f4a7c15)
        ^ (cell.y as u64).wrapping_mul(0xd1b54a32d192ed03);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_test_area(world: &mut World) {
        for y in -4..=4 {
            for x in -4..=4 {
                world.tiles.insert(Cell::new(x, y), Tile::Dirt);
            }
        }
        world.tiles.insert(world.player, Tile::Empty);
    }

    #[test]
    fn generation_is_repeatable_and_order_independent() {
        let mut a = World::new(42);
        let mut b = World::new(42);
        let west = Cell::new(-100, -100);
        let east = Cell::new(100, 100);
        a.ensure_area(west, 20);
        a.ensure_area(east, 20);
        b.ensure_area(east, 20);
        b.ensure_area(west, 20);
        assert_eq!(a.tiles, b.tiles);
        assert_ne!(World::new(43).tiles, World::new(42).tiles);
    }

    #[test]
    fn digging_persists_and_stones_block_movement() {
        let mut world = World::new(1);
        clear_test_area(&mut world);
        world.step(Cell::new(1, 0));
        assert_eq!(world.player, Cell::new(1, 0));
        assert_eq!(world.tile(world.player), Some(Tile::Empty));
        world.ensure_area(Cell::new(500, 0), 24);
        world.ensure_area(Cell::default(), 24);
        assert_eq!(world.tile(Cell::new(1, 0)), Some(Tile::Empty));
        world.tiles.insert(Cell::new(2, 0), Tile::Stone);
        world.step(Cell::new(1, 0));
        assert_eq!(world.player, Cell::new(1, 0));
    }

    #[test]
    fn player_pushes_left_and_right_into_empty_cells() {
        for dx in [-1, 1] {
            let mut world = World::new(1);
            clear_test_area(&mut world);
            world.tiles.insert(Cell::new(dx, 0), Tile::Stone);
            world.tiles.insert(Cell::new(2 * dx, 0), Tile::Empty);
            world.tiles.insert(Cell::new(3 * dx, 0), Tile::Empty);
            for distance in [1, 2] {
                world.step(Cell::new(dx, 0));
                assert!(!world.dead);
                assert_eq!(world.player, Cell::new(distance * dx, 0));
                assert_eq!(world.previous_player, Cell::new((distance - 1) * dx, 0));
                assert_eq!(world.tile(world.player), Some(Tile::Empty));
                assert_eq!(
                    world.tile(Cell::new((distance + 1) * dx, 0)),
                    Some(Tile::Stone)
                );
                assert_eq!(world.stone_motion.len(), 1);
                assert_eq!(world.stone_motion[0].from, world.player);
                assert_eq!(world.stone_motion[0].to, Cell::new((distance + 1) * dx, 0));
            }
        }
    }

    #[test]
    fn dirt_and_stones_block_pushing_without_being_cleared() {
        for dx in [-1, 1] {
            for blocker in [Tile::Dirt, Tile::Stone] {
                let mut world = World::new(1);
                clear_test_area(&mut world);
                world.tiles.insert(Cell::new(dx, 0), Tile::Stone);
                world.tiles.insert(Cell::new(2 * dx, 0), blocker);
                world.step(Cell::new(dx, 0));
                assert_eq!(world.player, Cell::default());
                assert_eq!(world.tile(Cell::new(dx, 0)), Some(Tile::Stone));
                assert_eq!(world.tile(Cell::new(2 * dx, 0)), Some(blocker));
            }
        }
    }

    #[test]
    fn player_cannot_push_stones_vertically() {
        for dy in [-1, 1] {
            let mut world = World::new(1);
            clear_test_area(&mut world);
            world.tiles.insert(Cell::new(0, dy), Tile::Stone);
            world.tiles.insert(Cell::new(0, 2 * dy), Tile::Empty);
            world.step(Cell::new(0, dy));
            assert!(!world.dead);
            assert_eq!(world.player, Cell::default());
            if dy > 0 {
                assert_eq!(world.tile(Cell::new(0, dy)), Some(Tile::Stone));
                assert_eq!(world.tile(Cell::new(0, 2 * dy)), Some(Tile::Empty));
            } else {
                // Gravity can move the lower stone, but the player must not
                // move into its old cell as though pushing it downwards.
                assert_eq!(world.tile(Cell::new(0, dy)), Some(Tile::Empty));
                assert_eq!(world.tile(Cell::new(0, 2 * dy)), Some(Tile::Stone));
            }
        }
    }

    #[test]
    fn pushed_stone_waits_until_next_tick_to_fall() {
        let mut world = World::new(1);
        clear_test_area(&mut world);
        world.tiles.insert(Cell::new(1, 0), Tile::Stone);
        world.tiles.insert(Cell::new(2, 0), Tile::Empty);
        world.tiles.insert(Cell::new(2, -1), Tile::Empty);
        world.step(Cell::new(1, 0));
        assert_eq!(world.tile(Cell::new(2, 0)), Some(Tile::Stone));
        assert_eq!(world.tile(Cell::new(2, -1)), Some(Tile::Empty));
        assert_eq!(world.stone_motion.len(), 1);
        world.step(Cell::default());
        assert_eq!(world.tile(Cell::new(2, 0)), Some(Tile::Empty));
        assert_eq!(world.tile(Cell::new(2, -1)), Some(Tile::Stone));
        assert!(!world.dead);
    }

    #[test]
    fn push_destination_is_reserved_against_falling_stones() {
        let mut world = World::new(1);
        clear_test_area(&mut world);
        world.tiles.insert(Cell::new(1, 0), Tile::Stone);
        world.tiles.insert(Cell::new(2, 0), Tile::Empty);
        world.tiles.insert(Cell::new(2, 1), Tile::Stone);
        world.step(Cell::new(1, 0));
        assert_eq!(world.tile(Cell::new(2, 0)), Some(Tile::Stone));
        assert_eq!(world.tile(Cell::new(2, 1)), Some(Tile::Stone));
        assert_eq!(world.stone_motion.len(), 1);
        assert!(!world.dead);
    }

    #[test]
    fn stones_fall_one_cell_and_dirt_supports_them() {
        let mut world = World::new(1);
        clear_test_area(&mut world);
        world.tiles.insert(Cell::new(2, 2), Tile::Stone);
        world.tiles.insert(Cell::new(2, 1), Tile::Empty);
        world.tiles.insert(Cell::new(2, 0), Tile::Empty);
        world.step(Cell::default());
        assert_eq!(world.tile(Cell::new(2, 1)), Some(Tile::Stone));
        assert_eq!(world.tile(Cell::new(2, 0)), Some(Tile::Empty));
        world.step(Cell::default());
        assert_eq!(world.tile(Cell::new(2, 0)), Some(Tile::Stone));
        world.step(Cell::default());
        assert!(world.stone_motion.is_empty());
    }

    #[test]
    fn stacked_stones_use_the_same_snapshot() {
        let mut world = World::new(1);
        clear_test_area(&mut world);
        world.tiles.insert(Cell::new(2, 2), Tile::Stone);
        world.tiles.insert(Cell::new(2, 1), Tile::Stone);
        world.tiles.insert(Cell::new(2, 0), Tile::Empty);
        world.step(Cell::default());
        assert_eq!(world.tile(Cell::new(2, 2)), Some(Tile::Stone));
        assert_eq!(world.tile(Cell::new(2, 1)), Some(Tile::Empty));
        assert_eq!(world.tile(Cell::new(2, 0)), Some(Tile::Stone));
        world.step(Cell::default());
        assert_eq!(world.tile(Cell::new(2, 1)), Some(Tile::Stone));
    }

    fn slipping_test_world(seed: u64, sides: &[i64]) -> World {
        let mut world = World::new(seed);
        clear_test_area(&mut world);
        world.tiles.insert(Cell::new(0, 2), Tile::Stone);
        world.tiles.insert(Cell::new(0, 1), Tile::Stone);
        for &dx in sides {
            world.tiles.insert(Cell::new(dx, 2), Tile::Empty);
            world.tiles.insert(Cell::new(dx, 1), Tile::Empty);
        }
        world
    }

    #[test]
    fn stones_slip_to_the_only_open_side_then_fall() {
        for dx in [-1, 1] {
            let mut world = slipping_test_world(1, &[dx]);
            world.step(Cell::default());
            assert_eq!(world.tile(Cell::new(0, 2)), Some(Tile::Empty));
            assert_eq!(world.tile(Cell::new(dx, 2)), Some(Tile::Stone));
            assert_eq!(world.tile(Cell::new(dx, 1)), Some(Tile::Empty));
            world.step(Cell::default());
            assert_eq!(world.tile(Cell::new(dx, 1)), Some(Tile::Stone));
        }
    }

    #[test]
    fn dirt_support_and_blocked_clearance_prevent_slipping() {
        for blocker in [Cell::new(0, 1), Cell::new(1, 2), Cell::new(1, 1)] {
            let mut world = slipping_test_world(1, &[1]);
            world.tiles.insert(blocker, Tile::Dirt);
            world.step(Cell::default());
            assert_eq!(world.tile(Cell::new(0, 2)), Some(Tile::Stone));
        }
    }

    #[test]
    fn slipping_can_choose_both_sides_and_replays_from_seed() {
        let mut directions = HashSet::new();
        for seed in 0..32 {
            let mut a = slipping_test_world(seed, &[-1, 1]);
            let mut b = slipping_test_world(seed, &[-1, 1]);
            a.step(Cell::default());
            b.step(Cell::default());
            assert_eq!(a.tiles, b.tiles);
            for dx in [-1, 1] {
                if a.tile(Cell::new(dx, 2)) == Some(Tile::Stone) {
                    directions.insert(dx);
                }
            }
        }
        assert_eq!(directions, HashSet::from([-1, 1]));
    }

    #[test]
    fn competing_slips_and_falls_never_share_a_destination() {
        for falling_above in [false, true] {
            let mut world = World::new(1);
            clear_test_area(&mut world);
            for x in [-1, 1] {
                for y in [1, 2] {
                    world.tiles.insert(Cell::new(x, y), Tile::Stone);
                }
            }
            world.tiles.insert(Cell::new(0, 2), Tile::Empty);
            world.tiles.insert(Cell::new(0, 1), Tile::Empty);
            if falling_above {
                world.tiles.insert(Cell::new(0, 3), Tile::Stone);
            }
            let count_stones = |world: &World| {
                world
                    .tiles
                    .values()
                    .filter(|&&tile| tile == Tile::Stone)
                    .count()
            };
            let count = count_stones(&world);
            world.step(Cell::default());
            assert_eq!(count_stones(&world), count);
            let arrivals: Vec<_> = world
                .stone_motion
                .iter()
                .filter(|motion| motion.to == Cell::new(0, 2))
                .collect();
            assert_eq!(arrivals.len(), 1);
            if falling_above {
                assert_eq!(arrivals[0].from, Cell::new(0, 3));
            }
        }
    }

    #[test]
    fn slipping_respects_player_and_does_not_count_as_falling() {
        let mut world = slipping_test_world(1, &[1]);
        world.player = Cell::new(1, 2);
        world.step(Cell::default());
        assert!(!world.dead);
        assert_eq!(world.tile(Cell::new(0, 2)), Some(Tile::Stone));

        let mut world = slipping_test_world(1, &[1]);
        world.step(Cell::default());
        // Step up from below into the gap under the newly slipped stone.
        world.player = Cell::new(1, 0);
        world.tiles.insert(world.player, Tile::Empty);
        world.step(Cell::new(0, 1));
        assert!(!world.dead);
        assert_eq!(world.tile(Cell::new(1, 2)), Some(Tile::Stone));
        assert_eq!(world.tile(world.player), Some(Tile::Empty));
    }

    #[test]
    fn distant_stones_wait_until_player_approaches() {
        let mut world = World::new(1);
        let stone = Cell::new(100, 0);
        world.ensure_area(stone, 4);
        world.tiles.insert(stone, Tile::Stone);
        world.tiles.insert(stone.offset(0, -1), Tile::Empty);
        world.step(Cell::default());
        assert_eq!(world.tile(stone), Some(Tile::Stone));
        world.player = stone.offset(-3, 0);
        world.tiles.insert(world.player, Tile::Empty);
        world.tiles.insert(world.player.offset(0, 1), Tile::Dirt);
        world.step(Cell::default());
        assert_eq!(world.tile(stone), Some(Tile::Empty));
        assert_eq!(world.tile(stone.offset(0, -1)), Some(Tile::Stone));
    }

    #[test]
    fn falling_stone_crushes_player_and_death_freezes_world() {
        let mut world = World::new(1);
        clear_test_area(&mut world);
        world.tiles.insert(Cell::new(0, 2), Tile::Stone);
        world.tiles.insert(Cell::new(0, 1), Tile::Empty);
        world.step(Cell::default());
        assert!(!world.dead);
        assert_eq!(world.tile(Cell::new(0, 1)), Some(Tile::Stone));
        world.step(Cell::default());
        assert!(world.dead);
        let tiles = world.tiles.clone();
        world.step(Cell::new(1, 0));
        assert_eq!(world.player, Cell::default());
        assert_eq!(world.tiles, tiles);
    }

    #[test]
    fn digging_directly_under_resting_stone_is_safe() {
        let mut world = World::new(1);
        clear_test_area(&mut world);
        let stone = Cell::new(1, 1);
        world.tiles.insert(stone, Tile::Stone);
        world.step(Cell::new(1, 0));
        for _ in 0..4 {
            world.step(Cell::default());
            assert!(!world.dead);
            assert_eq!(world.tile(stone), Some(Tile::Stone));
            assert_eq!(world.tile(world.player), Some(Tile::Empty));
        }
        world.step(Cell::new(1, 0));
        assert_eq!(world.tile(stone), Some(Tile::Stone));
        world.step(Cell::default());
        assert_eq!(world.tile(stone.offset(0, -1)), Some(Tile::Stone));
        assert!(!world.dead);
    }

    #[test]
    fn stone_that_landed_on_dirt_is_no_longer_falling() {
        let mut world = World::new(1);
        clear_test_area(&mut world);
        world.tiles.insert(Cell::new(1, 2), Tile::Stone);
        world.tiles.insert(Cell::new(1, 1), Tile::Empty);
        world.step(Cell::default());
        assert_eq!(world.tile(Cell::new(1, 1)), Some(Tile::Stone));
        world.step(Cell::default());
        world.step(Cell::new(1, 0));
        assert!(!world.dead);
        assert_eq!(world.tile(Cell::new(1, 1)), Some(Tile::Stone));
        assert_eq!(world.tile(world.player), Some(Tile::Empty));
    }

    #[test]
    fn stepping_under_an_already_falling_stone_is_fatal() {
        let mut world = World::new(1);
        clear_test_area(&mut world);
        world.tiles.insert(Cell::new(1, 2), Tile::Stone);
        world.tiles.insert(Cell::new(1, 1), Tile::Empty);
        world.step(Cell::default());
        world.step(Cell::new(1, 0));
        assert!(world.dead);
        assert_eq!(world.player, Cell::new(1, 0));
    }

    #[test]
    fn leaving_cell_is_reserved_until_animation_finishes() {
        let mut world = World::new(1);
        clear_test_area(&mut world);
        world.tiles.insert(Cell::new(0, 1), Tile::Stone);
        world.step(Cell::new(1, 0));
        assert!(!world.dead);
        assert_eq!(world.tile(Cell::new(0, 1)), Some(Tile::Stone));
        world.step(Cell::default());
        assert_eq!(world.tile(Cell::new(0, 0)), Some(Tile::Stone));
    }

    #[test]
    fn exploration_crosses_negative_chunk_boundaries_without_overwriting() {
        let mut world = World::new(1);
        let chunks = world.chunk_count();
        for x in -100..0 {
            world.ensure_area(Cell::new(x, 0), 1);
            world.tiles.insert(Cell::new(x, 0), Tile::Dirt);
            // Keep this generation/movement test's tunnel roof supported.
            world.tiles.insert(Cell::new(x, 1), Tile::Dirt);
        }
        for _ in 0..90 {
            world.step(Cell::new(-1, 0));
        }
        assert_eq!(world.player, Cell::new(-90, 0));
        assert!(world.chunk_count() > chunks);
        assert_eq!(world.tile(Cell::new(-16, 0)), Some(Tile::Empty));
    }
}
