// The game side of baritone.behavior.PathingBehavior and baritone.utils.PathingControlManager: the
// processes ask for a `PathingCommand` every tick, and `Core::path_tick` hands it to the pathing behavior
// (pathing_behavior.rs) with the game wrapped as a PlayerContext (game/player_context.rs). What is left
// here is what the Java keeps in BlockBreakHelper and BlockPlaceHelper for the processes that break and
// place blocks on their own, outside a path (mining a shaft, farming, building).
use super::baritone::Core;
use super::baritone_aim::AimSettings;
use super::baritone_calculation_context::{CalculationContext, CostHooks};
use super::baritone_player_context::PlayerContext;
use super::baritone_game_ctx::GameCtx;
use super::baritone_gworld::{best_tool, block_name};
use super::baritone_inventory as inv;
use super::baritone_pathing_behavior::PathingBehavior;
use super::baritone_settings::Settings;
use super::baritone_utils::{feet_of, Pos};
use super::Game;
use horizon_world::registry::BlockStateId;

pub use super::baritone_pathing_behavior::PathingCommand;

/// baritone.api.process.IBaritoneProcess.
pub trait Process {
    fn name(&self) -> &'static str;
    /// What `#proc` says it is doing.
    fn display(&self) -> String;
    fn is_active(&self) -> bool;
    /// The one with the higher priority is asked first and wins.
    fn priority(&self) -> f64;
    /// A temporary process (combat pause, backfill) may keep control without the ones under it being cancelled.
    fn is_temporary(&self) -> bool {
        false
    }
    /// Bring `is_active` up to date: the Java's `isActive` looks at the world each time it is asked.
    fn refresh(&mut self, _core: &mut Core, _game: &mut Game<'_>) {}
    fn on_tick(&mut self, core: &mut Core, game: &mut Game<'_>, calc_failed: bool, safe_to_cancel: bool) -> PathingCommand;
    fn on_lost_control(&mut self, core: &mut Core, game: &mut Game<'_>);
    /// Stop for good (the user cancelled, or another process starts).
    fn cancel(&mut self);
}

/// A block being broken.
pub struct Dig {
    pub pos: Pos,
    pub before: BlockStateId,
    pub started: u64,
    pub slot: Option<usize>,
}

/// Blocks broken and placed by a process itself, outside any path.
#[derive(Default)]
pub struct DigState {
    pub active: Option<Dig>,
    /// Blocks broken since a process last looked: where, and what.
    pub broken: Vec<(Pos, String)>,
    pub next_click: u64,
    /// Why the last click could not be aimed.
    pub aim_error: String,
}

const DIG_LIMIT: u64 = 20 * 40;

impl Core {
    pub fn player_feet(&self, game: &Game<'_>) -> Option<Pos> {
        game.player.as_ref().map(|p| feet_of([p.position.x, p.position.y, p.position.z]))
    }

    /// Run `f` with the pathing behavior and the game as the player it walks.
    pub fn drive<R>(&mut self, game: &mut Game<'_>, f: impl FnOnce(&mut PathingBehavior, &GameCtx<'_, '_>, &Settings) -> R) -> R {
        let Core { pathing, input, settings, throwaway, aim, states, world_data, .. } = self;
        let ctx = GameCtx::new(game, input, settings, throwaway, aim, states, world_data.current.as_ref().map(|d| &d.cache));
        f(pathing, &ctx, settings)
    }

    /// WorldProvider and the chunk packer, once a tick: follow the world and dimension, queue the chunks that
    /// loaded, pack as many as there is time for, and save now and then.
    pub fn cache_tick(&mut self, game: &mut Game<'_>) {
        self.world_data.follow(game, &self.settings);
        if !self.settings.flag("chunkCaching") {
            return;
        }
        self.world_data.queue_chunks(game);
        let batch = self.world_data.take_batch(game, 4);
        if !batch.is_empty() {
            let factory = self.states.clone();
            let (done, left) = self.with_calc(game, |calc, ctx, _| ctx.with_game(|g| super::baritone_world_data::WorldProvider::pack_batch(g, calc, &factory, batch, 4)));
            self.world_data.finish_batch(done, left);
        }
        self.world_data.save_if_due(game, &self.settings);
    }

    /// Run `f` with a CalculationContext made now (`new CalculationContext(baritone)`).
    pub fn with_calc<R>(&mut self, game: &mut Game<'_>, f: impl FnOnce(&CalculationContext, &GameCtx<'_, '_>, &Settings) -> R) -> R {
        self.with_calc_hooks(game, None, f)
    }

    /// The same with a subclass's overrides.
    pub fn with_calc_hooks<R>(&mut self, game: &mut Game<'_>, hooks: Option<std::rc::Rc<dyn CostHooks>>, f: impl FnOnce(&CalculationContext, &GameCtx<'_, '_>, &Settings) -> R) -> R {
        self.drive(game, |_, ctx, s| {
            let facts = PlayerContext::calculation_facts(ctx);
            let calc = CalculationContext::new(s, PlayerContext::bsi(ctx), ctx, &facts).with_hooks(hooks);
            f(&calc, ctx, s)
        })
    }

    /// The ticks the path being walked has left (PathingBehavior.ticksRemainingInSegment).
    pub fn ticks_remaining_in_segment(&self) -> Option<f64> {
        self.pathing.ticks_remaining_in_segment()
    }

    /// PathingBehavior.estimatedTicksToGoal.
    pub fn estimated_ticks_to_goal(&mut self, game: &Game<'_>) -> Option<f64> {
        let feet = self.player_feet(game)?;
        self.pathing.estimated_ticks_to_goal(feet)
    }

    /// Following a path, searching for one, or in the middle of breaking a block.
    pub fn pathing_busy(&self) -> bool {
        self.pathing.is_busy() || self.dig.active.is_some()
    }

    /// A path can be dropped without leaving the player in mid-air.
    pub fn pathing_safe_to_cancel(&self) -> bool {
        self.pathing.is_safe_to_cancel()
    }

    /// Drop the path and the search, and let go of every key.
    pub fn pathing_cancel(&mut self, game: &mut Game<'_>) {
        self.dig.active = None;
        self.drive(game, |p, ctx, _| p.force_cancel(ctx));
        self.input.release(game);
    }

    /// Forget the goal as well.
    pub fn pathing_reset(&mut self, game: &mut Game<'_>) {
        self.pathing_cancel(game);
        self.pathing.secret_internal_set_goal(None);
    }

    /// PathingBehavior.onTick, from the first half to the last, around what the processes asked for.
    pub fn path_tick(&mut self, game: &mut Game<'_>, command: Option<&PathingCommand>, real_process_changed: bool) -> Vec<String> {
        let ticks = game.ticks;
        // LookBehavior.onTick: the noise of this game tick.
        let aim_settings = AimSettings::of(&self.settings, f64::from(game.settings.sensitivity));
        self.aim.tick(&aim_settings);
        self.drive(game, |p, ctx, s| p.begin_tick(ctx, s, ctx));
        if self.dig.active.is_some() {
            self.dig_step(game);
            self.input.apply(game);
            return Vec::new();
        }
        let pause = PathingCommand::RequestPause;
        let command = if self.user_paused { Some(&pause) } else { command };
        let hooks = self.path_hooks.take();
        self.drive(game, |p, ctx, s| {
            p.pre_tick(ctx, s, command, hooks.clone(), real_process_changed, ticks);
            p.run_tick(ctx, s, ctx, ticks);
            p.post_tick(ctx, s, command, hooks, ticks);
        });
        let lines: Vec<String> = self.pathing.log.drain(..).collect();
        self.input.apply(game);
        if self.settings.flag("chatDebug") { lines } else { Vec::new() }
    }

    // -----------------------------------------------------------------------------------
    // Breaking blocks.

    /// Start breaking the block at `pos`. False when it cannot be broken.
    pub fn start_dig(&mut self, game: &mut Game<'_>, pos: Pos) -> bool {
        let before = game.world.block(pos[0], pos[1], pos[2]);
        let Some((_, slot)) = best_tool(game, before, self.settings.flag("autoTool")) else {
            return false;
        };
        self.input.release(game);
        if let Some(slot) = slot {
            inv::select(game, slot);
        }
        self.dig.active = Some(Dig { pos, before, started: game.ticks, slot });
        true
    }

    fn dig_step(&mut self, game: &mut Game<'_>) {
        let Some(d) = self.dig.active.as_ref() else {
            return;
        };
        let (pos, before, started, slot) = (d.pos, d.before, d.started, d.slot);
        let now = game.world.block(pos[0], pos[1], pos[2]);
        if now != before {
            self.input.hold_attack(false);
            let name = game.res.facts().get(before).name.to_string();
            self.dig.broken.push((pos, name));
            self.dig.active = None;
            return;
        }
        if game.ticks.saturating_sub(started) > DIG_LIMIT {
            self.input.hold_attack(false);
            self.dig.active = None;
            self.say_red(game, "A block would not break.");
            return;
        }
        if let Some(slot) = slot {
            inv::select(game, slot);
        }
        if let Err(why) = game.bar_aim_why(pos) {
            self.input.hold_attack(false);
            self.dig.active = None;
            let feet = self.player_feet(game).unwrap_or([0, 0, 0]);
            self.say_red(game, &format!("Cannot break {} at {}, {}, {} from {}, {}, {}: {why}", block_name(game, pos).trim_start_matches("minecraft:"), pos[0], pos[1], pos[2], feet[0], feet[1], feet[2]));
            return;
        }
        self.input.hold_attack(true);
    }

    // -----------------------------------------------------------------------------------
    // Placing.

    pub fn click_ready(&self, game: &Game<'_>) -> bool {
        game.ticks >= self.dig.next_click
    }

    /// Click the `face` of `against` with what is held. False when it cannot be seen from here.
    pub fn click_block(&mut self, game: &mut Game<'_>, against: Pos, face: [i32; 3]) -> bool {
        if !self.click_ready(game) {
            return false;
        }
        let done = game.bar_place(against, face);
        if done {
            self.dig.next_click = game.ticks + 3;
        } else if let Err(why) = game.bar_place_why(against, face) {
            self.dig.aim_error = why;
        }
        done
    }
}
