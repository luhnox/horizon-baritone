// baritone.process.BuilderProcess: make the world match a schematic. It looks for the blocks that are
// wrong, lowest layer first, walks to where one of them is in reach, and breaks it or puts the right
// block there. `#tunnel` and `#sel set|walls|sphere|replace...` are builds of the same kind.
//
// A block's properties (which way a stair faces) are not placed: the block is, and what the game
// decides for the click is what it is.
use super::baritone::Core;
use super::baritone_goals::{rc, xz_heuristic, y_heuristic, Goal};
use super::baritone_gworld::{block_name, count_items, exposed, hotbar_slot, loaded, GameTerrain};
use super::baritone_legacy_helper::{Cell, Terrain};
use super::baritone_inventory as inv;
use super::baritone_pathing::{PathingCommand, Process};
use super::baritone_schematic::{BlockSpec, Schematic};
use super::baritone_utils::{dist_sq, up, Pos};
use super::Game;
use std::{
    collections::{BTreeMap, HashMap},
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq)]
enum Need {
    Break,
    Place(BlockSpec),
}

#[derive(Clone, Debug)]
struct Wrong {
    pos: Pos,
    need: Need,
}

/// Standing where a block at `target` can be reached by hand, but not inside it.
struct ReachGoal {
    target: Pos,
}

impl ReachGoal {
    fn eye_dist_sq(&self, p: Pos) -> f64 {
        let eye = [f64::from(p[0]) + 0.5, f64::from(p[1]) + 1.62, f64::from(p[2]) + 0.5];
        let c = [f64::from(self.target[0]) + 0.5, f64::from(self.target[1]) + 0.5, f64::from(self.target[2]) + 0.5];
        (eye[0] - c[0]).powi(2) + (eye[1] - c[1]).powi(2) + (eye[2] - c[2]).powi(2)
    }
}

impl Goal for ReachGoal {
    fn is_in_goal(&self, p: Pos) -> bool {
        p != self.target && up(p, 1) != self.target && self.eye_dist_sq(p) <= 3.8 * 3.8
    }
    fn heuristic(&self, p: Pos) -> f64 {
        let (dx, dz) = (f64::from(p[0] - self.target[0]), f64::from(p[2] - self.target[2]));
        let flat = (dx.hypot(dz) - 3.0).max(0.0);
        xz_heuristic(flat, 0.0) + y_heuristic(self.target[1], p[1])
    }
    fn label(&self) -> String {
        format!("within reach of {}, {}, {}", self.target[0], self.target[1], self.target[2])
    }
    fn render_pos(&self) -> Option<Pos> {
        Some(self.target)
    }
}

/// The item that places a block (most are the same word).
pub fn item_for_block(block: &str) -> String {
    let bare = block.strip_prefix("minecraft:").unwrap_or(block);
    let item = match bare {
        "redstone_wire" => "redstone",
        "wheat" => "wheat_seeds",
        "carrots" => "carrot",
        "potatoes" => "potato",
        "beetroots" => "beetroot_seeds",
        "melon_stem" => "melon_seeds",
        "pumpkin_stem" => "pumpkin_seeds",
        "tripwire" => "string",
        "cocoa" => "cocoa_beans",
        "sweet_berry_bush" => "sweet_berries",
        "bamboo_sapling" => "bamboo",
        "lava" => "lava_bucket",
        "water" => "water_bucket",
        other => other,
    };
    let item = item.replace("_wall_", "_").replace("wall_torch", "torch");
    format!("minecraft:{item}")
}

pub struct BuilderProcess {
    schematic: Option<Rc<dyn Schematic>>,
    name: String,
    origin: Pos,
    active: bool,
    wrong: Vec<Wrong>,
    next_scan: u64,
    blacklist: HashMap<Pos, u64>,
    tries: HashMap<Pos, u32>,
    placed: u32,
    broken: u32,
    missing: BTreeMap<String, u32>,
    told_missing: bool,
    started: u64,
    /// Work the lowest layer first (a building); false: the nearest first (a tunnel).
    layered: bool,
    /// Waiting for materials to be put in the hotbar.
    paused: bool,
}

impl BuilderProcess {
    pub fn new() -> Self {
        Self {
            schematic: None,
            name: String::new(),
            origin: [0, 0, 0],
            active: false,
            wrong: Vec::new(),
            next_scan: 0,
            blacklist: HashMap::new(),
            tries: HashMap::new(),
            placed: 0,
            broken: 0,
            missing: BTreeMap::new(),
            told_missing: false,
            started: 0,
            layered: true,
            paused: false,
        }
    }

    /// Build `schematic` with its corner (its x, y, z all zero) at `origin`.
    pub fn build(&mut self, name: String, schematic: Rc<dyn Schematic>, origin: Pos, layered: bool, tick: u64) {
        *self = Self::new();
        self.layered = layered;
        self.name = name;
        self.schematic = Some(schematic);
        self.origin = origin;
        self.active = true;
        self.started = tick;
    }

    /// Break everything in the box between two corners, both included.
    pub fn clear_area(&mut self, c1: Pos, c2: Pos, tick: u64) {
        let lo = [0, 1, 2].map(|i| c1[i].min(c2[i]));
        let hi = [0, 1, 2].map(|i| c1[i].max(c2[i]));
        let size = [hi[0] - lo[0] + 1, hi[1] - lo[1] + 1, hi[2] - lo[2] + 1];
        let air = super::baritone_schematic::FillSchematic { size, spec: BlockSpec::air() };
        self.build("clear area".into(), Rc::new(air), lo, false, tick);
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn resume(&mut self) {
        self.paused = false;
        self.told_missing = false;
    }

    pub fn wrong_count(&self) -> usize {
        self.wrong.len()
    }

    pub fn placed(&self) -> u32 {
        self.placed
    }

    pub fn broken(&self) -> u32 {
        self.broken
    }

    pub fn missing(&self) -> &BTreeMap<String, u32> {
        &self.missing
    }

    /// The blocks still wrong, for drawing: where, and whether it is to be broken.
    pub fn wrong_positions(&self) -> impl Iterator<Item = (Pos, bool)> + '_ {
        self.wrong.iter().take(120).map(|w| (w.pos, w.need == Need::Break))
    }

    pub fn origin(&self) -> Pos {
        self.origin
    }

    pub fn schematic_size(&self) -> Option<[i32; 3]> {
        self.schematic.as_ref().map(|s| s.size())
    }

    /// Everything that differs, bottom layer first (or top first with layerOrder).
    fn scan(&mut self, core: &Core, game: &Game<'_>) {
        let Some(sch) = self.schematic.clone() else {
            return;
        };
        let [w, h, l] = sch.size();
        let settings = &core.settings;
        let limit = settings.int("incorrectSize").clamp(1, 5000) as usize;
        let ignore = settings.list("buildIgnoreBlocks");
        let ignore_existing = settings.flag("buildIgnoreExisting");
        let ok_if_water = settings.flag("okIfWater");
        let layers = settings.flag("buildInLayers");
        let top_down = settings.flag("layerOrder");
        let mut wrong: Vec<Wrong> = Vec::new();
        let ys: Vec<i32> = if top_down { (0..h).rev().collect() } else { (0..h).collect() };
        for y in ys {
            for z in 0..l {
                for x in 0..w {
                    let rel = [x, y, z];
                    let pos = [self.origin[0] + x, self.origin[1] + y, self.origin[2] + z];
                    if !loaded(game, pos) {
                        continue;
                    }
                    let state = game.world.block(pos[0], pos[1], pos[2]);
                    let facts = game.res.facts().get(state);
                    let current: &str = &facts.name;
                    let Some(spec) = sch.desired(rel, current) else {
                        continue;
                    };
                    let is_air = game.res.facts().is_air(state);
                    if spec.is_air() {
                        if is_air || (ok_if_water && facts.fluid != horizon_client::blocks::Fluid::None) {
                            continue;
                        }
                    } else if current == spec.name {
                        continue;
                    }
                    let bare = current.strip_prefix("minecraft:").unwrap_or(current);
                    if ignore.iter().any(|i| i == bare) || (ignore_existing && !is_air) {
                        continue;
                    }
                    if self.blacklist.get(&pos).is_some_and(|until| game.ticks < *until) {
                        continue;
                    }
                    if facts.hardness < 0.0 && !is_air {
                        continue;
                    }
                    let need = if spec.is_air() {
                        Need::Break
                    } else if is_air || facts.replaceable || facts.fluid != horizon_client::blocks::Fluid::None {
                        Need::Place(spec)
                    } else {
                        Need::Break
                    };
                    wrong.push(Wrong { pos, need });
                }
            }
            if layers && !wrong.is_empty() {
                break;
            }
            if wrong.len() >= limit * 4 {
                break;
            }
        }
        self.wrong = wrong;
    }

    /// Whether a block put at `pos` would have something to be put against.
    fn supported(terrain: &mut dyn Terrain, pos: Pos) -> Vec<([i32; 3], Pos)> {
        // Under it first: the way a player builds up.
        [[0, -1, 0], [-1, 0, 0], [1, 0, 0], [0, 0, -1], [0, 0, 1], [0, 1, 0]]
            .into_iter()
            .filter_map(|d| {
                let s = [pos[0] + d[0], pos[1] + d[1], pos[2] + d[2]];
                (terrain.cell(s) == Cell::Solid).then_some(([-d[0], -d[1], -d[2]], s))
            })
            .collect()
    }
}

impl Process for BuilderProcess {
    fn name(&self) -> &'static str {
        "build"
    }

    fn display(&self) -> String {
        format!("Building {}: {} blocks to go, {} placed, {} broken", self.name, self.wrong.len(), self.placed, self.broken)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn priority(&self) -> f64 {
        0.1
    }

    fn on_tick(&mut self, core: &mut Core, game: &mut Game<'_>, _calc_failed: bool, _safe: bool) -> PathingCommand {
        let tick = game.ticks;
        let Some(feet) = core.player_feet(game) else {
            return PathingCommand::Defer;
        };
        for (pos, _) in std::mem::take(&mut core.pathing.broken) {
            self.broken += 1;
            self.tries.remove(&pos);
        }
        if tick >= self.next_scan {
            self.next_scan = tick + 20;
            self.scan(core, game);
            self.missing.clear();
        }
        // Already right: drop it from the list at once rather than at the next scan.
        let sch = self.schematic.clone();
        if let Some(sch) = &sch {
            let origin = self.origin;
            let mut kept = Vec::with_capacity(self.wrong.len());
            for mut w in std::mem::take(&mut self.wrong) {
                let rel = [w.pos[0] - origin[0], w.pos[1] - origin[1], w.pos[2] - origin[2]];
                let name = block_name(game, w.pos);
                let is_air = game.res.facts().is_air(game.world.block(w.pos[0], w.pos[1], w.pos[2]));
                match (&w.need, sch.desired(rel, &name)) {
                    (Need::Break, Some(spec)) if spec.is_air() => {
                        if is_air {
                            continue;
                        }
                    }
                    (Need::Break, Some(spec)) => {
                        if name == spec.name {
                            continue;
                        }
                        if is_air {
                            w.need = Need::Place(spec);
                        }
                    }
                    (Need::Place(spec), _) => {
                        if name == spec.name {
                            continue;
                        }
                    }
                    _ => continue,
                }
                kept.push(w);
            }
            self.wrong = kept;
        }
        if self.wrong.is_empty() {
            core.say_green(
                game,
                &format!("Finished building {}: {} placed, {} broken.", self.name, self.placed, self.broken),
            );
            self.active = false;
            return PathingCommand::CancelAndSetGoal(rc(super::baritone_goals::GoalBlock(feet)));
        }

        // The wrong block to work on: breaks and placements both, the lowest layer first, nearest first.
        let top_down = core.settings.flag("layerOrder");
        let mut order: Vec<usize> = (0..self.wrong.len()).collect();
        order.sort_by_key(|&i| {
            let w = &self.wrong[i];
            (if !self.layered { 0 } else if top_down { -w.pos[1] } else { w.pos[1] }, dist_sq(w.pos, feet))
        });
        let mut choice: Option<(usize, Vec<([i32; 3], Pos)>, usize, Option<Pos>)> = None;
        let mut buried: Option<(usize, Vec<([i32; 3], Pos)>, usize, Option<Pos>)> = None;
        {
            let g = &*game;
            let mut terrain = GameTerrain::new(g, &mut core.cells, &core.avoid_breaking, true);
            for &i in order.iter().take(60) {
                let w = &self.wrong[i];
                match &w.need {
                    Need::Break => {
                        if exposed(&mut terrain, w.pos, false) {
                            choice = Some((i, Vec::new(), usize::MAX, None));
                            break;
                        }
                        buried.get_or_insert((i, Vec::new(), usize::MAX, None));
                    }
                    Need::Place(spec) => {
                        let item = item_for_block(&spec.name);
                        let Some(slot) = hotbar_slot(g, |n| n == item) else {
                            *self.missing.entry(item).or_insert(0) += 1;
                            continue;
                        };
                        let faces = Self::supported(&mut terrain, w.pos);
                        if !faces.is_empty() {
                            choice = Some((i, faces, slot, None));
                            break;
                        }
                    }
                }
            }
        }
        let Some((index, faces, slot, dig_at)) = choice.or(buried) else {
            // Nothing can be done yet: blocks floating in the air, or materials missing.
            if !self.missing.is_empty() && !self.told_missing {
                self.told_missing = true;
                self.paused = true;
                let list: Vec<String> = self.missing.keys().map(|k| k.trim_start_matches("minecraft:").to_owned()).collect();
                core.say_red(game, &format!("Missing materials in the hotbar: {}. Put them there and I will go on.", list.join(", ")));
            }
            return PathingCommand::Defer;
        };
        self.told_missing = false;
        self.paused = false;
        let target = dig_at.unwrap_or(self.wrong[index].pos);
        let reach = ReachGoal { target };
        // In reach: act.
        if reach.is_in_goal(feet) && core.pathing.dig.is_none() && !core.pathing.paused {
            match &self.wrong[index].need {
                Need::Break => {
                    let n = self.tries.entry(target).or_insert(0);
                    *n += 1;
                    if game.res.facts().is_air(game.world.block(target[0], target[1], target[2])) {
                        self.tries.remove(&target);
                        if dig_at.is_none() {
                            self.wrong.remove(index);
                        }
                    } else if *n > 4 || !core.start_dig(game, target) {
                        self.blacklist.insert(target, tick + 2000);
                        self.tries.remove(&target);
                        let name = block_name(game, target);
                        core.say_red(game, &format!("I cannot break {} at {}, {}, {}: skipping it.", name.trim_start_matches("minecraft:"), target[0], target[1], target[2]));
                    }
                }
                Need::Place(_) => {
                    // Only a click that is made counts: waiting for the game's use delay is not a try.
                    if core.click_ready(game) {
                        inv::select(game, slot);
                        let n = self.tries.entry(target).or_insert(0);
                        if faces.iter().any(|(face, support)| core.click_block(game, *support, *face)) {
                            self.placed += 1;
                            *n += 1;
                        } else {
                            *n += 2;
                        }
                        // Clicked six times and it is still not there, or could not be aimed at.
                        if *n > 10 {
                            self.blacklist.insert(target, tick + 400);
                            self.tries.remove(&target);
                            let why = std::mem::take(&mut core.pathing.aim_error);
                            core.say_red(game, &format!("A block would not go down at {}, {}, {}: skipping it for now. {why}", target[0], target[1], target[2]));
                        }
                    }
                }
            }
        }
        PathingCommand::RevalidateGoalAndPath(rc(reach))
    }

    fn on_lost_control(&mut self, _core: &mut Core, _game: &mut Game<'_>) {}

    fn cancel(&mut self) {
        self.active = false;
    }
}

/// How many of an item the inventory holds, for the build's report.
pub fn have(game: &Game<'_>, item: &str) -> u32 {
    count_items(game, |n| n == item)
}
