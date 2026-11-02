// baritone.behavior.LookBehavior: where the player faces. Turns are a bounded number of degrees a
// tick so a move reads as a player's, and aiming at a block (to break or place against it) is the
// agent module's, reached through bridge.rs.
use super::baritone_utils::{approach_yaw, wrap_degrees};
use super::Game;

/// Turn towards `yaw` by at most `max` degrees. The yaw after the turn.
pub fn turn_to(game: &mut Game<'_>, yaw: f32, max: f32) -> f32 {
    game.yaw = approach_yaw(game.yaw, yaw, max);
    game.yaw
}

/// Lean the pitch towards `pitch` by at most `max` degrees.
pub fn pitch_to(game: &mut Game<'_>, pitch: f32, max: f32) {
    game.pitch += (pitch - game.pitch).clamp(-max, max);
}

/// How far, in degrees, the player faces from `yaw`.
pub fn off_by(game: &Game<'_>, yaw: f32) -> f32 {
    wrap_degrees(yaw - game.yaw).abs()
}
