// baritone.behavior.InventoryBehavior: which hotbar slot holds what a move needs. Items are used from
// the hotbar; the main inventory is not searched.
use super::baritone_gworld::{bare, hotbar_slot};
use super::Game;

/// A block that is fine to put down and lose, from the acceptableThrowawayItems setting.
pub fn throwaway_slot(game: &Game<'_>, acceptable: &[String]) -> Option<usize> {
    hotbar_slot(game, |n| acceptable.iter().any(|a| a == bare(n)))
}

pub fn bucket_slot(game: &Game<'_>) -> Option<usize> {
    hotbar_slot(game, |n| n == "minecraft:water_bucket")
}

pub fn empty_bucket_slot(game: &Game<'_>) -> Option<usize> {
    hotbar_slot(game, |n| n == "minecraft:bucket")
}

pub fn firework_slot(game: &Game<'_>) -> Option<usize> {
    hotbar_slot(game, |n| n == "minecraft:firework_rocket")
}

/// The slot holding exactly this item (`minecraft:wheat_seeds`).
pub fn slot_of(game: &Game<'_>, item: &str) -> Option<usize> {
    hotbar_slot(game, |n| n == item)
}

/// Hold the stack in `slot`.
pub fn select(game: &mut Game<'_>, slot: usize) {
    game.selected = slot;
}
