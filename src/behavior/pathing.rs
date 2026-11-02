// baritone.behavior.PathingBehavior and utils.PathingControlManager: the goal, the search that finds
// a path to it over a few ticks, the executor that walks it, and the things a path asks of the
// player on the way: break a block, put one down, open a door.
//
// The processes (process/) decide WHERE to go and hand this a `PathingCommand` every tick; this
// decides HOW. Everything here is a method of `Core`, so a process can be given the whole of it.
use super::baritone::Core;
use super::baritone_legacy_calc::{AStar, Plan, Status};
use super::baritone_goals::GoalRef;
use super::baritone_gworld::{best_tool, block_name, hotbar_slot, GameTerrain};
use super::baritone_legacy_helper::{CalcSettings, Cell, Terrain};
use super::baritone_inventory as inv;
use super::baritone_look as look;
use super::baritone_legacy_movement::{required_free, Mv};
use super::baritone_legacy_path::{Controls, PathExecutor, Snapshot, Step};
use super::baritone_utils::{feet_of, Pos};
use super::Game;
use horizon_world::registry::BlockStateId;
use std::collections::HashMap;

/// What a process asks of the pathing behavior this tick.
pub enum PathingCommand {
    /// Head for the goal: search if there is no path, and walk it.
    SetGoalAndPath(GoalRef),
    /// Hold still and keep the path.
    RequestPause,
    /// Drop the path and take this goal.
    CancelAndSetGoal(GoalRef),
    /// Take this goal, and drop the path if it no longer leads there.
    RevalidateGoalAndPath(GoalRef),
    /// The same, and search at once.
    ForceRevalidateGoalAndPath(GoalRef),
    SetGoalAndPause(GoalRef),
    /// This process has nothing to say: the next one decides.
    Defer,
}

impl PathingCommand {
    /// How `#proc` prints it.
    pub fn describe(&self) -> String {
        let (kind, goal) = match self {
            PathingCommand::SetGoalAndPath(g) => ("SET_GOAL_AND_PATH", Some(g)),
            PathingCommand::RequestPause => ("REQUEST_PAUSE", None),
            PathingCommand::CancelAndSetGoal(g) => ("CANCEL_AND_SET_GOAL", Some(g)),
            PathingCommand::RevalidateGoalAndPath(g) => ("REVALIDATE_GOAL_AND_PATH", Some(g)),
            PathingCommand::ForceRevalidateGoalAndPath(g) => ("FORCE_REVALIDATE_GOAL_AND_PATH", Some(g)),
            PathingCommand::SetGoalAndPause(g) => ("SET_GOAL_AND_PAUSE", Some(g)),
            PathingCommand::Defer => ("DEFER", None),
        };
        format!("PathingCommand{{commandType={kind}, goal={}}}", goal.map_or("null".to_owned(), |g| g.label()))
    }
}

/// baritone.api.process.IBaritoneProcess.
pub trait Process {
    fn name(&self) -> &'static str;
    /// What `#proc` says it is doing.
    fn display(&self) -> String;
    fn is_active(&self) -> bool;
    /// The one with the higher priority is asked first and wins.
    fn priority(&self) -> f64;
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

/// What the path is going to break and place, for drawing.
#[derive(Default)]
pub struct Marks {
    pub breaks: Vec<Pos>,
    pub places: Vec<Pos>,
}

struct Search {
    astar: AStar,
    started: u64,
    /// The search began at the end of the path being walked.
    ahead: bool,
}

#[derive(Default)]
pub struct PathingState {
    pub goal: Option<GoalRef>,
    search: Option<Search>,
    pub executor: Option<PathExecutor>,
    /// The path being walked reaches the goal.
    pub complete: bool,
    /// The last search found nowhere to go.
    pub calc_failed: bool,
    pub paused: bool,
    pub marks: Marks,
    pub dig: Option<Dig>,
    /// Blocks broken since a process last looked: where, and what.
    pub broken: Vec<(Pos, String)>,
    pub next_click: u64,
    /// Why the last click could not be aimed.
    pub aim_error: String,
    pub clicks: u32,
    pub stuck: u32,
    door_tries: HashMap<Pos, u32>,
    replans: u32,
    stalls: u32,
    last_end: Option<Pos>,
    /// The ticks the path being walked costs, and the tick it was found.
    pub plan_cost: f64,
    pub plan_found: u64,
    /// A path found while the last one was still being walked.
    pending: Option<(Plan, Marks)>,
}

const MAX_REPLANS: u32 = 12;
const NODES_PER_TICK: usize = 3000;
const PRIMARY_TIMEOUT: u64 = 10;
const FAILURE_TIMEOUT: u64 = 40;
const DIG_LIMIT: u64 = 20 * 40;
const TURN: f32 = 45.0;

impl Core {
    pub fn calc_settings(&self, game: &Game<'_>) -> CalcSettings {
        let s = &self.settings;
        let blocks = inv::throwaway_slot(game, &self.throwaway).is_some();
        let bucket = inv::bucket_slot(game).is_some();
        CalcSettings {
            max_nodes: 30_000,
            max_fall: s.int("maxFallHeightNoWater").clamp(0, 30),
            max_fall_into_water: s.int("maxFallHeightBucket").clamp(0, 60),
            max_fall_bucket: if bucket && s.flag("allowWaterBucketFall") { s.int("maxFallHeightBucket").clamp(0, 60) } else { 0 },
            allow_swim: s.flag("allowSwimming"),
            allow_sprint: s.flag("allowSprint"),
            allow_break: s.flag("allowBreak"),
            break_penalty: s.float("blockBreakAdditionalPenalty").max(0.0),
            allow_place: blocks && s.flag("allowPlace"),
            place_cost: s.float("blockPlacementPenalty").max(0.0),
            allow_parkour: s.flag("allowParkour"),
            jump_penalty: s.float("jumpPenalty").max(0.0),
            allow_downward: s.flag("allowDownward"),
        }
    }

    pub fn player_feet(&self, game: &Game<'_>) -> Option<Pos> {
        game.player.as_ref().map(|p| feet_of([p.position.x, p.position.y, p.position.z]))
    }

    pub fn pathing_in_goal(&self, game: &Game<'_>) -> bool {
        match (&self.pathing.goal, self.player_feet(game)) {
            (Some(g), Some(f)) => g.is_in_goal(f),
            _ => false,
        }
    }

    /// The ticks the path being walked has left (PathingBehavior.ticksRemainingInSegment).
    pub fn ticks_remaining_in_segment(&self) -> Option<f64> {
        let ex = self.pathing.executor.as_ref()?;
        let total = ex.path.len().max(1) as f64;
        let left = (ex.path.len() - ex.index.min(ex.path.len())) as f64;
        Some(self.pathing.plan_cost * left / total)
    }

    /// The ticks to the goal: this segment, and what the goal's heuristic says is left past its end.
    pub fn estimated_ticks_to_goal(&self) -> Option<f64> {
        let segment = self.ticks_remaining_in_segment()?;
        let ex = self.pathing.executor.as_ref()?;
        let beyond = match (&self.pathing.goal, ex.last()) {
            (Some(goal), Some(end)) if !self.pathing.complete => goal.heuristic(end),
            _ => 0.0,
        };
        Some(segment + beyond)
    }

    pub fn pathing_calculating(&self) -> bool {
        self.pathing.search.is_some()
    }

    /// Following a path, searching for one, or in the middle of breaking a block.
    pub fn pathing_busy(&self) -> bool {
        self.pathing.search.is_some() || self.pathing.executor.is_some() || self.pathing.dig.is_some()
    }

    /// A path can be dropped without leaving the player in mid-air.
    pub fn pathing_safe_to_cancel(&self, game: &Game<'_>) -> bool {
        match (&self.pathing.executor, game.player.as_ref()) {
            (Some(_), Some(p)) => p.ground.on_ground || p.swimming,
            _ => true,
        }
    }

    pub fn pathing_cancel(&mut self, game: &mut Game<'_>) {
        let p = &mut self.pathing;
        p.search = None;
        p.executor = None;
        p.pending = None;
        p.dig = None;
        p.complete = false;
        p.replans = 0;
        p.stalls = 0;
        p.last_end = None;
        p.marks = Marks::default();
        p.door_tries.clear();
        p.clicks = 0;
        p.stuck = 0;
        self.input.release(game);
        if game.bar_attacking() {
            game.bar_attack(false);
        }
    }

    /// Forget the goal as well.
    pub fn pathing_reset(&mut self, game: &mut Game<'_>) {
        self.pathing_cancel(game);
        self.pathing.goal = None;
        self.pathing.paused = false;
        self.pathing.calc_failed = false;
    }

    pub fn pathing_command(&mut self, game: &mut Game<'_>, command: PathingCommand) {
        match command {
            PathingCommand::Defer => {}
            PathingCommand::RequestPause => self.pathing.paused = true,
            PathingCommand::SetGoalAndPause(goal) => {
                self.pathing.goal = Some(goal);
                self.pathing.paused = true;
            }
            PathingCommand::CancelAndSetGoal(goal) => {
                self.pathing_cancel(game);
                self.pathing.goal = Some(goal);
                self.pathing.paused = false;
            }
            PathingCommand::SetGoalAndPath(goal) => {
                self.pathing.paused = false;
                self.pathing.goal = Some(goal);
                self.pathing_start_if_idle(game);
            }
            PathingCommand::RevalidateGoalAndPath(goal) | PathingCommand::ForceRevalidateGoalAndPath(goal) => {
                self.pathing.paused = false;
                // A whole path that no longer ends in the goal leads to the wrong place.
                let stale = self.pathing.complete && self.pathing.executor.as_ref().and_then(|e| e.last()).is_some_and(|end| !goal.is_in_goal(end));
                self.pathing.goal = Some(goal);
                if stale {
                    self.pathing.executor = None;
                    self.pathing.pending = None;
                    self.pathing.complete = false;
                    self.input.release(game);
                }
                self.pathing_start_if_idle(game);
            }
        }
    }

    fn pathing_start_if_idle(&mut self, game: &mut Game<'_>) {
        if self.pathing.executor.is_none() && self.pathing.search.is_none() && self.pathing.dig.is_none() && !self.pathing_in_goal(game) {
            if let Some(feet) = self.player_feet(game) {
                self.start_search(game, feet, false);
            }
        }
    }

    fn start_search(&mut self, game: &mut Game<'_>, from: Pos, ahead: bool) {
        let Some(goal) = self.pathing.goal.clone() else {
            return;
        };
        let settings = self.calc_settings(game);
        let g = &*game;
        let mut terrain = GameTerrain::new(g, &mut self.cells, &self.avoid_breaking, self.settings.flag("autoTool"));
        match AStar::new(&mut terrain, goal, from, settings) {
            Some(astar) => {
                self.pathing.search = Some(Search { astar, started: game.ticks, ahead });
                if !ahead {
                    self.pathing.calc_failed = false;
                }
            }
            None => {
                if !ahead {
                    self.pathing.calc_failed = true;
                }
            }
        }
    }

    pub fn pathing_tick(&mut self, game: &mut Game<'_>) {
        if self.pathing.paused {
            self.input.release(game);
            if game.bar_attacking() {
                game.bar_attack(false);
            }
            return;
        }
        if self.pathing.dig.is_some() {
            self.dig_step(game);
            return;
        }
        if self.pathing.search.is_some() {
            self.advance_search(game);
        }
        if self.pathing.executor.is_some() {
            self.run_executor(game);
        } else if self.pathing.search.is_some() || self.pathing.dig.is_some() {
            // Waiting for a path: stand still.
            self.input.release(game);
        }
    }

    fn advance_search(&mut self, game: &mut Game<'_>) {
        let tick = game.ticks;
        let auto = self.settings.flag("autoTool");
        let status = {
            let g = &*game;
            let Core { cells, avoid_breaking, pathing, .. } = self;
            let Some(search) = pathing.search.as_mut() else {
                return;
            };
            let mut terrain = GameTerrain::new(g, cells, avoid_breaking, auto);
            match search.astar.run(&mut terrain, NODES_PER_TICK) {
                Status::Running => {
                    let elapsed = tick.saturating_sub(search.started);
                    // Baritone's timeouts: after the primary one any path that goes somewhere will do,
                    // after the failure one, any path at all.
                    let best = search.astar.best_so_far();
                    match best {
                        Some(p) if elapsed >= PRIMARY_TIMEOUT && p.path.len() >= 6 => Status::Done(p),
                        Some(p) if elapsed >= FAILURE_TIMEOUT => Status::Done(p),
                        None if elapsed >= FAILURE_TIMEOUT => Status::Failed,
                        _ => Status::Running,
                    }
                }
                other => other,
            }
        };
        match status {
            Status::Running => {}
            Status::Failed => {
                self.pathing.search = None;
                self.pathing.calc_failed = true;
            }
            Status::Done(plan) => {
                let ahead = self.pathing.search.take().is_some_and(|s| s.ahead);
                self.accept(game, plan, ahead);
            }
        }
    }

    fn accept(&mut self, game: &mut Game<'_>, plan: Plan, ahead: bool) {
        let tick = game.ticks;
        let marks = {
            let g = &*game;
            let mut terrain = GameTerrain::new(g, &mut self.cells, &self.avoid_breaking, self.settings.flag("autoTool"));
            marks_of(&mut terrain, &plan.path, &plan.moves)
        };
        let p = &mut self.pathing;
        if !plan.complete {
            let end = plan.path.last().copied();
            p.stalls = if p.last_end == end { p.stalls + 1 } else { 0 };
            p.last_end = end;
            if p.stalls >= 3 {
                p.stalls = 0;
                p.calc_failed = true;
                p.executor = None;
                p.pending = None;
                return;
            }
        } else {
            p.stalls = 0;
        }
        p.calc_failed = false;
        if ahead && p.executor.is_some() {
            p.pending = Some((plan, marks));
            return;
        }
        p.complete = plan.complete;
        p.plan_cost = plan.cost;
        p.plan_found = tick;
        p.marks = marks;
        p.clicks = 0;
        p.stuck = 0;
        p.executor = Some(PathExecutor::with_moves(plan.path, plan.moves, tick));
    }

    fn run_executor(&mut self, game: &mut Game<'_>) {
        let tick = game.ticks;
        let (position, on_ground, blocked, swimming) = {
            let Some(p) = game.player.as_ref() else {
                return;
            };
            ([p.position.x, p.position.y, p.position.z], p.ground.on_ground, p.collision.horizontal, p.swimming)
        };
        let here = feet_of(position);
        let Some(mut executor) = self.pathing.executor.take() else {
            return;
        };
        let outcome = {
            let g = &*game;
            let mut terrain = GameTerrain::new(g, &mut self.cells, &self.avoid_breaking, self.settings.flag("autoTool"));
            let inside = terrain.cell(here);
            let snapshot = Snapshot {
                pos: position,
                on_ground,
                in_water: inside == Cell::Water || swimming,
                climbing: inside == Cell::Climb,
                blocked,
                tick,
            };
            executor.step(&snapshot, &mut terrain)
        };
        // Look for the next stretch while this one is still being walked.
        let remaining = executor.remaining().len();
        let end = executor.last();
        let ahead = !self.pathing.complete && remaining <= 20 && self.pathing.search.is_none() && self.pathing.pending.is_none();
        self.pathing.executor = Some(executor);
        if ahead {
            if let Some(end) = end {
                self.start_search(game, end, true);
            }
        }
        match outcome {
            Step::Go(controls) => {
                self.pathing.stuck = 0;
                self.apply_controls(game, controls);
            }
            Step::Dig(pos) => {
                if !self.start_dig(game, pos) {
                    self.drop_path(game, "I cannot break that");
                }
            }
            Step::Place { pos, against, face } => self.place_block(game, pos, against, face),
            Step::Pillar { pos, against } => self.pillar(game, pos, against),
            Step::Interact(pos) => self.interact(game, pos),
            Step::Arrived => {
                self.input.release(game);
                self.pathing.replans = 0;
                match self.pathing.pending.take() {
                    Some((plan, marks)) if !self.pathing.complete && plan.path.first() == self.pathing.executor.as_ref().and_then(|e| e.last()).as_ref() => {
                        self.pathing.complete = plan.complete;
                        self.pathing.plan_cost = plan.cost;
                        self.pathing.marks = marks;
                        self.pathing.executor = Some(PathExecutor::with_moves(plan.path, plan.moves, tick));
                    }
                    _ => self.pathing.executor = None,
                }
            }
            Step::Replan(why) => self.drop_path(game, why),
        }
    }

    /// The path no longer fits: drop it, and give up when that keeps happening.
    fn drop_path(&mut self, game: &mut Game<'_>, why: &str) {
        let p = &mut self.pathing;
        p.executor = None;
        p.pending = None;
        p.complete = false;
        p.replans += 1;
        p.stuck = 0;
        if p.replans > MAX_REPLANS {
            p.replans = 0;
            p.calc_failed = true;
            self.input.release(game);
            self.say_red(game, &format!("Stopped: I keep losing the path ({why})."));
        }
    }

    fn apply_controls(&mut self, game: &mut Game<'_>, controls: Controls) {
        look::turn_to(game, controls.yaw, TURN);
        // A look down left by placing or breaking comes back up while walking.
        look::pitch_to(game, 10.0, 15.0);
        // Facing within reach of the way: push; a wide turn first.
        let facing = look::off_by(game, controls.yaw) < 50.0;
        let sprint = controls.sprint && facing && self.settings.flag("allowSprint");
        self.input.walk(game, controls.forward && facing, sprint, controls.jump, controls.sneak);
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
        self.pathing.dig = Some(Dig { pos, before, started: game.ticks, slot });
        true
    }

    fn dig_step(&mut self, game: &mut Game<'_>) {
        let Some(d) = self.pathing.dig.as_ref() else {
            return;
        };
        let (pos, before, started, slot) = (d.pos, d.before, d.started, d.slot);
        let now = game.world.block(pos[0], pos[1], pos[2]);
        if now != before {
            // Broken.
            if game.bar_attacking() {
                game.bar_attack(false);
            }
            let name = game.res.facts().get(before).name.to_string();
            self.pathing.broken.push((pos, name));
            self.pathing.dig = None;
            return;
        }
        if game.ticks.saturating_sub(started) > DIG_LIMIT {
            if game.bar_attacking() {
                game.bar_attack(false);
            }
            self.pathing.dig = None;
            self.drop_path(game, "a block would not break");
            return;
        }
        if let Some(slot) = slot {
            inv::select(game, slot);
        }
        if let Err(why) = game.bar_aim_why(pos) {
            if game.bar_attacking() {
                game.bar_attack(false);
            }
            self.pathing.dig = None;
            let feet = self.player_feet(game).unwrap_or([0, 0, 0]);
            self.say_red(game, &format!("Cannot break {} at {}, {}, {} from {}, {}, {}: {why}", block_name(game, pos).trim_start_matches("minecraft:"), pos[0], pos[1], pos[2], feet[0], feet[1], feet[2]));
            self.drop_path(game, "a block was out of reach");
            return;
        }
        if !game.bar_attacking() {
            game.bar_attack(true);
        }
    }

    // -----------------------------------------------------------------------------------
    // Placing, pillaring, opening.

    pub fn click_ready(&self, game: &Game<'_>) -> bool {
        game.ticks >= self.pathing.next_click
    }

    /// Click the `face` of `against` with what is held. False when it cannot be seen from here.
    pub fn click_block(&mut self, game: &mut Game<'_>, against: Pos, face: [i32; 3]) -> bool {
        if !self.click_ready(game) {
            return false;
        }
        let done = game.bar_place(against, face);
        if done {
            self.pathing.next_click = game.ticks + 3;
        } else if let Err(why) = game.bar_place_why(against, face) {
            self.pathing.aim_error = why;
        }
        done
    }

    /// A bridge block: walk to the edge of the floor, sneaking so as not to fall, until the face of
    /// the block under the feet can be seen from there, and click it.
    fn place_block(&mut self, game: &mut Game<'_>, _pos: Pos, against: Pos, face: [i32; 3]) {
        let Some(slot) = inv::throwaway_slot(game, &self.throwaway) else {
            self.drop_path(game, "there are no blocks in the hotbar to place");
            self.say_red(game, "No blocks to place in the hotbar: put cobblestone or dirt in it.");
            return;
        };
        inv::select(game, slot);
        self.pathing.stuck += 1;
        if self.pathing.stuck > 140 || self.pathing.clicks > 30 {
            self.drop_path(game, "a block would not go down");
            return;
        }
        if !self.click_ready(game) {
            self.input.only(game, true, false, false);
            return;
        }
        if game.bar_place(against, face) {
            self.pathing.next_click = game.ticks + 4;
            self.pathing.clicks += 1;
            self.input.only(game, true, false, false);
        } else {
            // Not in sight yet: edge forward.
            look::turn_to(game, super::baritone_utils::yaw_to(f64::from(face[0]), f64::from(face[2])), TURN);
            look::pitch_to(game, 70.0, 20.0);
            self.input.only(game, true, true, false);
        }
    }

    /// Pillar up: look down, jump, and put the block into the cell the feet left, against the block
    /// that was under them.
    fn pillar(&mut self, game: &mut Game<'_>, pos: Pos, against: Pos) {
        let Some(slot) = inv::throwaway_slot(game, &self.throwaway) else {
            self.drop_path(game, "there are no blocks in the hotbar to place");
            self.say_red(game, "No blocks to place in the hotbar: put cobblestone or dirt in it.");
            return;
        };
        inv::select(game, slot);
        let (y, on_ground) = match game.player.as_ref() {
            Some(p) => (p.position.y, p.ground.on_ground),
            None => return,
        };
        self.pathing.stuck += 1;
        if self.pathing.stuck > 160 || self.pathing.clicks > 12 {
            self.drop_path(game, "I could not pillar up");
            return;
        }
        game.pitch = 90.0;
        // The body is clear of the cell once the feet are above it.
        if y > f64::from(pos[1]) + 1.0 + 0.002 && self.click_ready(game) {
            if game.bar_place(against, [0, 1, 0]) {
                self.pathing.next_click = game.ticks + 2;
                self.pathing.clicks += 1;
            }
            self.input.only(game, false, false, false);
        } else {
            self.input.only(game, false, false, on_ground);
        }
    }

    /// A closed door in the way: click it open, a few times at most.
    fn interact(&mut self, game: &mut Game<'_>, pos: Pos) {
        self.input.only(game, false, false, false);
        if !self.click_ready(game) {
            return;
        }
        let tries = self.pathing.door_tries.entry(pos).or_insert(0);
        *tries += 1;
        if *tries > 5 {
            self.pathing.door_tries.clear();
            self.drop_path(game, "I cannot open a door");
            self.say_red(game, &format!("I cannot open the door at {}, {}, {}.", pos[0], pos[1], pos[2]));
            return;
        }
        let opened = game.bar_interact(pos);
        self.pathing.next_click = game.ticks + if opened { 8 } else { 2 };
    }

    /// The first hotbar slot with an item, for processes that put things down.
    pub fn hotbar_item(&self, game: &Game<'_>, item: &str) -> Option<usize> {
        hotbar_slot(game, |n| n == item)
    }

    /// The name of the block at `pos`.
    pub fn block_at(&self, game: &Game<'_>, pos: Pos) -> String {
        block_name(game, pos)
    }
}

/// What a path will break and place, for drawing.
pub fn marks_of(t: &mut dyn Terrain, path: &[Pos], moves: &[Mv]) -> Marks {
    let mut marks = Marks::default();
    for i in 0..path.len().saturating_sub(1) {
        let (cur, next) = (path[i], path[i + 1]);
        match moves.get(i + 1).copied().unwrap_or(Mv::Walk) {
            Mv::Bridge => marks.places.push([next[0], next[1] - 1, next[2]]),
            Mv::Pillar => marks.places.push(cur),
            _ => {
                for c in required_free(cur, next) {
                    if t.cell(c) == Cell::Solid {
                        marks.breaks.push(c);
                    }
                }
            }
        }
    }
    marks
}
