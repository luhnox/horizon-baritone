// The test crate for the parts of Baritone that use no game types. In the game the same files are
// modules of game.rs (see horizon-mixin.toml); here they are modules of one crate with the same
// names, so `use super::baritone_goals::...` reads the same in both.
#[path = "api/utils.rs"]
pub mod baritone_utils;
#[path = "pathing/costs.rs"]
pub mod baritone_costs;
#[path = "api/goals.rs"]
pub mod baritone_goals;
#[path = "pathing/helper.rs"]
pub mod baritone_legacy_helper;
#[path = "api/settings.rs"]
pub mod baritone_settings;
#[path = "selection/selection.rs"]
pub mod baritone_selection;
#[path = "schematic/schematic.rs"]
pub mod baritone_schematic;
#[path = "cache/waypoints.rs"]
pub mod baritone_waypoints;
#[path = "command/argument.rs"]
pub mod baritone_argument;
#[path = "utils/block_state.rs"]
pub mod baritone_block_state;
#[path = "utils/block_state_interface.rs"]
pub mod baritone_block_state_interface;
#[path = "pathing/movement/calculation_context.rs"]
pub mod baritone_calculation_context;
#[path = "pathing/movement/movement_helper.rs"]
pub mod baritone_movement_helper;
#[path = "pathing/movement/movements/traverse.rs"]
pub mod baritone_movement_traverse;
#[path = "pathing/movement/movements/ascend.rs"]
pub mod baritone_movement_ascend;
#[path = "pathing/movement/movements/descend.rs"]
pub mod baritone_movement_descend;
#[path = "pathing/movement/movements/diagonal.rs"]
pub mod baritone_movement_diagonal;
#[path = "pathing/movement/movements/downward.rs"]
pub mod baritone_movement_downward;
#[path = "pathing/movement/movements/parkour.rs"]
pub mod baritone_movement_parkour;
#[path = "pathing/movement/movements/pillar.rs"]
pub mod baritone_movement_pillar;
#[path = "pathing/movement/moves.rs"]
pub mod baritone_moves;
#[path = "pathing/movement/movement.rs"]
pub mod baritone_movement;
#[path = "pathing/calc/a_star.rs"]
pub mod baritone_astar;
#[path = "pathing/calc/path.rs"]
pub mod baritone_calc_path;
#[path = "utils/rotation.rs"]
pub mod baritone_rotation;
#[path = "behavior/movement_state.rs"]
pub mod baritone_movement_state;
#[path = "behavior/player_context.rs"]
pub mod baritone_player_context;
#[path = "pathing/movement/movement_exec.rs"]
pub mod baritone_movement_exec;
#[path = "pathing/path/executor.rs"]
pub mod baritone_executor;
#[path = "behavior/pathing_behavior.rs"]
pub mod baritone_pathing_behavior;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_pathing;
