// baritone.api.utils.IPlayerContext, IPlayerController, RayTraceUtils and the parts of LookBehavior,
// InventoryBehavior and InputOverrideHandler that movements use: everything a movement asks of the
// player and the game while it runs. The game implements this; the movements are written against it, so
// they can be tested with a player on paper. Every method takes `&self`: the implementation keeps the game
// behind a cell, so the world can be read (through `bsi`) while the player is being steered.
use super::baritone_block_state_interface::BlockStateInterface;
use super::baritone_calculation_context::PlayerFacts;
use super::baritone_movement_state::Input;
use super::baritone_rotation::Rotation;
use super::baritone_utils::Pos;

pub trait PlayerContext {
    /// The world as it is now (BlockStateInterface.get(ctx, pos)).
    fn bsi(&self) -> &dyn BlockStateInterface;

    // ---- the player
    fn player_feet(&self) -> Pos;
    fn position(&self) -> [f64; 3];
    /// The eyes (`playerHead`).
    fn player_head(&self) -> [f64; 3];
    fn delta_movement(&self) -> [f64; 3];
    fn player_rotations(&self) -> Rotation;
    fn on_ground(&self) -> bool;
    fn is_in_water(&self) -> bool;
    fn is_swimming(&self) -> bool;
    fn is_under_water(&self) -> bool;
    fn is_crouching(&self) -> bool;
    fn horizontal_collision(&self) -> bool;
    fn is_in_wall(&self) -> bool;
    fn food_level(&self) -> i32;
    fn air_supply(&self) -> i32;
    fn max_air_supply(&self) -> i32;
    /// `getFluidHeight(FluidTags.WATER)`.
    fn water_fluid_height(&self) -> f64;
    fn set_sprinting(&self, on: bool);
    fn is_nether(&self) -> bool;

    // ---- what the crosshair is on
    fn is_looking_at(&self, pos: Pos) -> bool;
    fn selected_block(&self) -> Option<Pos>;
    fn selected_face(&self) -> Option<[i32; 3]>;

    // ---- reach and aim
    fn block_reach_distance(&self) -> f64;
    /// RotationUtils.reachable: an aim at a block from here, if one of its faces can be hit within reach.
    fn reachable(&self, pos: Pos, reach: f64, would_sneak: bool) -> Option<Rotation>;
    /// RayTraceUtils.rayTraceTowards: the block hit by an aim, and the face.
    fn ray_trace_towards(&self, rotation: Rotation, reach: f64, would_sneak: bool) -> Option<(Pos, [i32; 3])>;
    /// RayTraceUtils.inferSneakingEyePosition.
    fn infer_sneaking_eye_position(&self) -> [f64; 3];
    /// LookBehavior.getAimProcessor().peekRotation: where an aim will really be after smoothing.
    fn peek_rotation(&self, rotation: Rotation) -> Rotation;

    /// What a search needs to know of the player besides the settings (CalculationContext's constructor).
    fn calculation_facts(&self) -> PlayerFacts;

    // ---- the inventory
    fn has_generic_throwaway(&self) -> bool;
    /// InventoryBehavior.selectThrowawayForLocation.
    fn select_throwaway_for_location(&self, select: bool, x: i32, y: i32, z: i32) -> bool;
    /// MovementHelper.switchToBestToolFor.
    fn switch_to_best_tool_for(&self, pos: Pos);
    fn water_bucket_hotbar_slot(&self) -> Option<usize>;
    fn empty_bucket_hotbar_slot(&self) -> Option<usize>;
    fn select_hotbar_slot(&self, slot: usize);

    // ---- entities
    /// Whether a falling block is on the way into this block position.
    fn falling_block_near(&self, pos: Pos) -> bool;

    // ---- what the movement asks for
    fn update_look_target(&self, rotation: Rotation, force: bool);
    fn clear_all_keys(&self);
    fn set_input_force_state(&self, input: Input, forced: bool);
    fn is_input_forced_down(&self, input: Input) -> bool;
    /// BlockBreakHelper.stopBreakingBlock.
    fn stop_breaking_block(&self);
}
