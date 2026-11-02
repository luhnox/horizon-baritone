// baritone.utils.BlockStateInterface: what the pathfinder reads of the world, one block state at a
// time. The game answers it from the loaded chunks; the tests answer it from a map.
use super::baritone_block_state::BlockState;
use std::rc::Rc;

pub trait BlockStateInterface {
    /// `get0`: the block at the position, air where nothing is loaded (the Java does the same).
    fn get0(&self, x: i32, y: i32, z: i32) -> Rc<BlockState>;

    /// Whether the chunk holding this column is loaded.
    fn is_loaded(&self, x: i32, z: i32) -> bool;

    /// `worldContainsLoadedChunk`.
    fn world_contains_loaded_chunk(&self, x: i32, z: i32) -> bool {
        self.is_loaded(x, z)
    }

    /// The lowest block y of the world.
    fn min_y(&self) -> i32;

    /// One above the highest block y of the world, as `BlockStateInterface.maxY`.
    fn max_y(&self) -> i32;

    /// `BetterWorldBorder.entirelyContains`: the whole block column is inside the world border.
    fn entirely_contains(&self, _x: i32, _z: i32) -> bool {
        true
    }

    /// `BetterWorldBorder.canPlaceAt`: inside the world border.
    fn can_place_at(&self, _x: i32, _z: i32) -> bool {
        true
    }
}
