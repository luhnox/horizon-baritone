// The game as the pathfinder's PlayerContext, BlockStateInterface and ToolSet: the one place that turns
// Horizon's player, world and inventory into what the ported Baritone code asks for. Block states are
// built from Horizon's facts once per state id and shared.
use super::baritone_aim::{AimProcessor, AimSettings};
use super::baritone_block_state::{parse_props, BlockState, Class, FluidKind};
use super::baritone_block_state_interface::BlockStateInterface;
use super::baritone_calculation_context::{PlayerFacts, ToolSet};
use super::baritone_gworld::{best_tool, break_progress, loaded};
use super::baritone_input::Input as InputHandler;
use super::baritone_inventory as inv;
use super::baritone_movement_state::Input;
use super::baritone_player_context::PlayerContext;
use super::baritone_rotation::{block_pos_center, calc_rotation_from_vec3d, Rotation};
use super::baritone_settings::Settings;
use super::baritone_utils::{feet_of, Pos};
use super::Game;
use horizon_client::blocks::Fluid;
use horizon_core::geometry::Axis;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct GameCtx<'a, 'g> {
    game: RefCell<&'a mut Game<'g>>,
    input: RefCell<&'a mut InputHandler>,
    settings: &'a Settings,
    throwaway: &'a [String],
    aim: &'a AimProcessor,
    aim_settings: AimSettings,
    states: RefCell<HashMap<u32, Rc<BlockState>>>,
    tool_ticks: RefCell<HashMap<u32, f64>>,
}

impl<'a, 'g> GameCtx<'a, 'g> {
    pub fn new(game: &'a mut Game<'g>, input: &'a mut InputHandler, settings: &'a Settings, throwaway: &'a [String], aim: &'a AimProcessor) -> Self {
        let aim_settings = AimSettings::of(settings, f64::from(game.settings.sensitivity));
        Self {
            game: RefCell::new(game),
            input: RefCell::new(input),
            settings,
            throwaway,
            aim,
            aim_settings,
            states: RefCell::new(HashMap::new()),
            tool_ticks: RefCell::new(HashMap::new()),
        }
    }

    fn build_state(game: &Game<'_>, state: horizon_world::registry::BlockStateId) -> BlockState {
        let facts = game.res.facts().get(state);
        let fluid_source = facts.fluid_level == 8 && !facts.fluid_falling;
        let fluid = match facts.fluid {
            Fluid::None => FluidKind::None,
            Fluid::Water if fluid_source => FluidKind::Water,
            Fluid::Water => FluidKind::FlowingWater,
            Fluid::Lava if fluid_source => FluidKind::Lava,
            Fluid::Lava => FluidKind::FlowingLava,
        };
        let (full, empty) = match game.res.collision().cached_shapes().cached_shape(state) {
            Ok(Some(shape)) if !shape.is_empty() => ([Axis::X, Axis::Y, Axis::Z].into_iter().all(|a| shape.min(a) <= 0.0 && shape.max(a) >= 1.0), false),
            Ok(Some(_)) => (false, true),
            _ => (false, false),
        };
        BlockState {
            id: state.0 as u32,
            name: facts.name.clone(),
            classes: Class::flags_of(facts.classes.iter().map(|c| &**c)),
            props: parse_props(&game.state_string(state)),
            fluid,
            fluid_amount: facts.fluid_level,
            hardness: facts.hardness,
            replaceable: facts.replaceable,
            full_collision: full,
            sturdy_top: (facts.sturdy_full >> 1) & 1 == 1,
            // isPathfindable(LAND): nothing to collide with and no fluid; the rest are the specials the helper handles.
            pathfindable: empty && facts.fluid == Fluid::None,
        }
    }

    fn player<R>(&self, f: impl FnOnce(&super::Game<'_>) -> R) -> R {
        f(&**self.game.borrow())
    }

    fn eye_with(&self, sneak: bool) -> [f64; 3] {
        let g = self.game.borrow();
        let p = g.player.as_ref().map_or([0.0; 3], |p| [p.position.x, p.position.y, p.position.z]);
        g.bar_eye().map_or([p[0], p[1] + if sneak { 1.27 } else { 1.62 }, p[2]], |e| if sneak { [e[0], p[1] + 1.27, e[2]] } else { e })
    }

    /// RotationUtils.reachableOffset.
    fn reachable_offset(&self, pos: Pos, offset: [f64; 3], reach: f64, would_sneak: bool) -> Option<Rotation> {
        let eyes = if would_sneak { self.infer_sneaking_eye_position() } else { self.player_head() };
        let rotation = calc_rotation_from_vec3d(eyes, offset, self.player_rotations());
        let actual = self.peek_rotation(rotation);
        let (hit, _) = self.ray_trace_towards(actual, reach, would_sneak)?;
        if hit == pos {
            return Some(rotation);
        }
        let name_is_fire = self.bsi_state(pos).is(Class::BaseFire);
        if name_is_fire && hit == [pos[0], pos[1] - 1, pos[2]] {
            return Some(rotation);
        }
        None
    }

    fn bsi_state(&self, pos: Pos) -> Rc<BlockState> {
        self.get0(pos[0], pos[1], pos[2])
    }
}

impl BlockStateInterface for GameCtx<'_, '_> {
    fn get0(&self, x: i32, y: i32, z: i32) -> Rc<BlockState> {
        let g = self.game.borrow();
        let air = |g: &Game<'_>| g.world.block(x, 0, z);
        let _ = air;
        let id = if y < horizon_client::chunk::MIN_Y || y >= horizon_client::chunk::MIN_Y + horizon_client::chunk::HEIGHT || !loaded(&g, [x, y, z]) {
            g.res.facts().air
        } else {
            g.world.block(x, y, z)
        };
        let key = id.0 as u32;
        if let Some(s) = self.states.borrow().get(&key) {
            return s.clone();
        }
        let built = Rc::new(Self::build_state(&g, id));
        self.states.borrow_mut().insert(key, built.clone());
        built
    }

    fn is_loaded(&self, x: i32, z: i32) -> bool {
        loaded(&self.game.borrow(), [x, 0, z])
    }

    fn min_y(&self) -> i32 {
        horizon_client::chunk::MIN_Y
    }

    /// `BetterWorldBorder.entirelyContains`.
    fn entirely_contains(&self, x: i32, z: i32) -> bool {
        let [min_x, min_z, max_x, max_z] = self.game.borrow().border.bounds();
        f64::from(x) + 1.0 > min_x && f64::from(x) < max_x && f64::from(z) + 1.0 > min_z && f64::from(z) < max_z
    }

    /// `BetterWorldBorder.canPlaceAt`: a block cannot go at the very edge, against a block outside the border.
    fn can_place_at(&self, x: i32, z: i32) -> bool {
        let [min_x, min_z, max_x, max_z] = self.game.borrow().border.bounds();
        f64::from(x) > min_x && f64::from(x) + 1.0 < max_x && f64::from(z) > min_z && f64::from(z) + 1.0 < max_z
    }

    fn max_y(&self) -> i32 {
        horizon_client::chunk::MIN_Y + horizon_client::chunk::HEIGHT
    }
}

impl ToolSet for GameCtx<'_, '_> {
    fn str_vs_block(&self, state: &BlockState) -> f64 {
        if let Some(t) = self.tool_ticks.borrow().get(&state.id) {
            return *t;
        }
        let g = self.game.borrow();
        let id = horizon_world::registry::BlockStateId(state.id as _);
        // The best tool's progress per tick (the Java's getStrVsBlock); none can break it: 0.
        let value = break_progress(&g, id, self.settings.flag("autoTool")).map_or(0.0, |(progress, _)| progress);
        self.tool_ticks.borrow_mut().insert(state.id, value);
        value
    }
}

impl PlayerContext for GameCtx<'_, '_> {
    fn bsi(&self) -> &dyn BlockStateInterface {
        self
    }

    fn player_feet(&self) -> Pos {
        self.player(|g| g.player.as_ref().map_or([0, 0, 0], |p| feet_of([p.position.x, p.position.y, p.position.z])))
    }

    fn position(&self) -> [f64; 3] {
        self.player(|g| g.player.as_ref().map_or([0.0; 3], |p| [p.position.x, p.position.y, p.position.z]))
    }

    fn player_head(&self) -> [f64; 3] {
        self.eye_with(false)
    }

    fn delta_movement(&self) -> [f64; 3] {
        self.player(|g| g.player.as_ref().map_or([0.0; 3], |p| [p.velocity.x, p.velocity.y, p.velocity.z]))
    }

    fn player_rotations(&self) -> Rotation {
        self.player(|g| Rotation::new(g.yaw, g.pitch))
    }

    fn on_ground(&self) -> bool {
        self.player(|g| g.player.as_ref().is_some_and(|p| p.ground.on_ground))
    }

    fn is_in_water(&self) -> bool {
        self.player(|g| g.player.as_ref().is_some_and(|p| p.fluids.was_touching_water))
    }

    fn is_swimming(&self) -> bool {
        self.player(|g| g.player.as_ref().is_some_and(|p| p.swimming))
    }

    fn is_under_water(&self) -> bool {
        self.player(|g| g.player.as_ref().is_some_and(|p| p.fluids.was_eye_in_water))
    }

    fn is_crouching(&self) -> bool {
        self.player(|g| g.player.as_ref().is_some_and(|p| p.local_crouching))
    }

    fn horizontal_collision(&self) -> bool {
        self.player(|g| g.player.as_ref().is_some_and(|p| p.collision.horizontal))
    }

    fn is_in_wall(&self) -> bool {
        let eye = self.player_head();
        let s = self.get0(eye[0].floor() as i32, eye[1].floor() as i32, eye[2].floor() as i32);
        s.full_collision
    }

    fn food_level(&self) -> i32 {
        self.player(|g| g.player.as_ref().map_or(20, |p| p.health.food.food as i32))
    }

    fn air_supply(&self) -> i32 {
        self.player(|g| g.player.as_ref().map_or(300, |p| p.air))
    }

    fn max_air_supply(&self) -> i32 {
        300
    }

    fn water_fluid_height(&self) -> f64 {
        // The height of the water column the feet stand in (Entity.getFluidHeight): whole blocks of water
        // above the feet, plus the top block's level.
        let feet = self.player_feet();
        let p = self.position();
        let mut top = None;
        for y in feet[1]..feet[1] + 4 {
            let s = self.get0(feet[0], y, feet[2]);
            if matches!(s.fluid, FluidKind::Water | FluidKind::FlowingWater) {
                top = Some((y, s.fluid_amount));
            } else {
                break;
            }
        }
        top.map_or(0.0, |(y, amount)| (f64::from(y) + f64::from(amount) / 9.0 - p[1]).max(0.0))
    }

    fn set_sprinting(&self, on: bool) {
        if let Some(p) = self.game.borrow_mut().player.as_mut() {
            p.sprinting = on;
        }
    }

    fn is_nether(&self) -> bool {
        self.player(|g| g.native_player().is_some_and(|p| p.dimension == "minecraft:the_nether"))
    }

    fn is_sprinting(&self) -> bool {
        self.player(|g| g.player.as_ref().is_some_and(|p| p.sprinting))
    }

    fn is_in_lava(&self) -> bool {
        self.player(|g| g.player.as_ref().is_some_and(|p| p.is_in_lava()))
    }

    fn on_climbable(&self) -> bool {
        self.player(|g| g.player.as_ref().is_some_and(|p| p.last_climbable.is_some()))
    }

    fn has_unsupported_motion_effects(&self) -> bool {
        self.player(|g| {
            let s = g.potions.effects.status();
            s.jump_boost.is_some() || s.slow_falling || s.levitation.is_some()
        })
    }

    fn movement_speed(&self) -> f64 {
        0.13 // 0.1 base, times the sprint modifier
    }

    fn is_looking_at(&self, pos: Pos) -> bool {
        self.selected_block() == Some(pos)
    }

    fn selected_block(&self) -> Option<Pos> {
        self.selected().map(|(p, _)| p)
    }

    fn selected_face(&self) -> Option<[i32; 3]> {
        self.selected().map(|(_, f)| f)
    }

    fn block_reach_distance(&self) -> f64 {
        self.player(|g| g.bar_reach())
    }

    fn reachable(&self, pos: Pos, reach: f64, would_sneak: bool) -> Option<Rotation> {
        // remainWithExistingLookDirection
        if self.settings.flag("remainWithExistingLookDirection") && self.is_looking_at(pos) {
            let hypothetical = self.player_rotations().add(Rotation::new(0.0, 0.0001));
            if would_sneak {
                // What if we are looking at it now, but as soon as we start sneaking we no longer are?
                if let Some((hit, _)) = self.ray_trace_towards(hypothetical, reach, true) {
                    if hit == pos {
                        return Some(hypothetical);
                    }
                }
            } else {
                return Some(hypothetical);
            }
        }
        if let Some(r) = self.reachable_offset(pos, block_pos_center(pos), reach, would_sneak) {
            return Some(r);
        }
        // The sides of the block: (the Java's BLOCK_SIDE_MULTIPLIERS against a full block's shape).
        for (sx, sy, sz) in [(0.5, 0.0, 0.5), (0.5, 1.0, 0.5), (0.5, 0.5, 0.0), (0.5, 0.5, 1.0), (0.0, 0.5, 0.5), (1.0, 0.5, 0.5)] {
            let point = [f64::from(pos[0]) + (1.0 - sx), f64::from(pos[1]) + (1.0 - sy), f64::from(pos[2]) + (1.0 - sz)];
            if let Some(r) = self.reachable_offset(pos, point, reach, would_sneak) {
                return Some(r);
            }
        }
        None
    }

    fn ray_trace_towards(&self, rotation: Rotation, reach: f64, would_sneak: bool) -> Option<(Pos, [i32; 3])> {
        let eye = self.eye_with(would_sneak);
        self.player(|g| g.bar_ray(eye, rotation.yaw, rotation.pitch, reach))
    }

    fn infer_sneaking_eye_position(&self) -> [f64; 3] {
        self.eye_with(true)
    }

    fn peek_rotation(&self, rotation: Rotation) -> Rotation {
        self.aim.peek_rotation(rotation, self.player_rotations(), &self.aim_settings)
    }

    fn calculation_facts(&self) -> PlayerFacts {
        PlayerFacts {
            has_throwaway: self.has_generic_throwaway(),
            has_water_bucket: self.settings.flag("allowWaterBucketFall") && self.water_bucket_hotbar_slot().is_some() && !self.is_nether(),
            can_sprint: self.food_level() > 6,
            frost_walker: 0,
            water_speed_multiplier: 0.0,
        }
    }

    fn has_generic_throwaway(&self) -> bool {
        self.player(|g| inv::throwaway_slot(g, self.throwaway).is_some())
    }

    fn select_throwaway_for_location(&self, select: bool, _x: i32, _y: i32, _z: i32) -> bool {
        let mut g = self.game.borrow_mut();
        let Some(slot) = inv::throwaway_slot(&g, self.throwaway) else { return false };
        if select {
            inv::select(&mut g, slot);
        }
        true
    }

    fn switch_to_best_tool_for(&self, pos: Pos) {
        if !self.settings.flag("autoTool") || self.settings.flag("assumeExternalAutoTool") {
            return;
        }
        let mut g = self.game.borrow_mut();
        let state = g.world.block(pos[0], pos[1], pos[2]);
        if let Some((_, Some(slot))) = best_tool(&g, state, true) {
            inv::select(&mut g, slot);
        }
    }

    fn water_bucket_hotbar_slot(&self) -> Option<usize> {
        self.player(inv::bucket_slot)
    }

    fn empty_bucket_hotbar_slot(&self) -> Option<usize> {
        self.player(inv::empty_bucket_slot)
    }

    fn select_hotbar_slot(&self, slot: usize) {
        inv::select(&mut self.game.borrow_mut(), slot);
    }

    fn falling_block_near(&self, _pos: Pos) -> bool {
        false
    }

    fn mobs_to_avoid(&self) -> Vec<Pos> {
        // Every Mob, but a spider by day, an unangered zombified piglin and an enderman nobody is looking at,
        // none of which can be told from here.
        self.player(|g| {
            g.mobs
                .iter()
                .filter(|m| m.alive())
                .filter(|m| !matches!(&*m.kind.id, "minecraft:spider" | "minecraft:cave_spider" | "minecraft:zombified_piglin" | "minecraft:enderman"))
                .map(|m| feet_of([m.position.x, m.position.y, m.position.z]))
                .collect()
        })
    }

    fn mob_spawners_near(&self) -> Vec<Pos> {
        let feet = self.player_feet();
        let g = self.game.borrow();
        let ids: std::collections::HashSet<u32> = g.res.facts().states.iter().enumerate().filter(|(_, s)| &*s.name == "minecraft:spawner").map(|(i, _)| i as u32).collect();
        if ids.is_empty() {
            return Vec::new();
        }
        let mut found = Vec::new();
        let (cx, cz) = (feet[0] >> 4, feet[2] >> 4);
        for x in cx - 2..=cx + 2 {
            for z in cz - 2..=cz + 2 {
                let Some(chunk) = g.world.chunks.get(&(x, z)) else { continue };
                for (s, section) in chunk.sections.iter().enumerate() {
                    if !section.blocks.maybe_has(|v| ids.contains(&v)) {
                        continue;
                    }
                    let base = horizon_client::chunk::MIN_Y + s as i32 * 16;
                    for y in base..base + 16 {
                        for lz in 0..16 {
                            for lx in 0..16 {
                                if ids.contains(&chunk.get(lx, y, lz)) {
                                    found.push([x * 16 + lx, y, z * 16 + lz]);
                                }
                            }
                        }
                    }
                }
            }
        }
        found.sort_by_key(|p| (p[0] - feet[0]).pow(2) + (p[1] - feet[1]).pow(2) + (p[2] - feet[2]).pow(2));
        found.truncate(1);
        found
    }

    fn update_look_target(&self, rotation: Rotation, _force: bool) {
        // LookBehavior.updateTarget in its CLIENT mode: the camera turns (there is no silent server-side rotation
        // here), by whole mouse steps and with the noise of the aim processor.
        let actual = self.peek_rotation(rotation);
        let mut g = self.game.borrow_mut();
        g.yaw = actual.yaw;
        g.pitch = actual.pitch.clamp(-90.0, 90.0);
    }

    fn clear_all_keys(&self) {
        self.input.borrow_mut().clear_all_keys();
    }

    fn set_input_force_state(&self, input: Input, forced: bool) {
        self.input.borrow_mut().set_input_force_state(input, forced);
    }

    fn is_input_forced_down(&self, input: Input) -> bool {
        self.input.borrow().is_input_forced_down(input)
    }

    fn stop_breaking_block(&self) {
        self.input.borrow_mut().set_input_force_state(Input::ClickLeft, false);
    }
}

impl GameCtx<'_, '_> {
    fn selected(&self) -> Option<(Pos, [i32; 3])> {
        let eye = self.eye_with(false);
        self.player(|g| {
            let reach = g.bar_reach();
            g.bar_ray(eye, g.yaw, g.pitch, reach)
        })
    }
}
