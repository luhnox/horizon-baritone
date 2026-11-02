// baritone.process.MineProcess: keep a list of the places the wanted blocks are, head for the nearest as
// a GoalThreeBlocks (into the block, under it, or two under it), break what is above the head from
// there, and when none is known dig away at legitMineYLevel from where the mining began. With legitMine
// only blocks a player could see are added, from a cube of radius 10 around the player.
// Where this differs from the Java: a quantity counts the inventory's items of what the blocks drop,
// from a table of the common ores instead of loot tables; and the drop of a broken block is found by
// scanning the ground, not by remembering where it fell.
use super::baritone::Core;
use super::baritone_goals::{rc, Goal, GoalBlock, GoalComposite, GoalRef, GoalRunAway, GoalThreeBlocks, GoalTwoBlocks};
use super::baritone_gworld::{bare, block_name, count_items, dropped_items, exposed, loaded, GameTerrain};
use super::baritone_pathing::{PathingCommand, Process};
use super::baritone_utils::{dist_sq, Pos};
use super::Game;
use std::collections::HashSet;

pub struct MineProcess {
    names: HashSet<String>,
    label: String,
    /// What the blocks drop, for counting a quantity.
    drops: HashSet<String>,
    desired: usize,
    active: bool,
    known: Vec<Pos>,
    items: Vec<Pos>,
    blacklist: HashSet<Pos>,
    branch_point: Option<Pos>,
    next_scan: u64,
    last_break: Option<Pos>,
    loiter_until: u64,
}

/// The item a block gives when it is broken with a good pickaxe.
fn drop_of(block: &str) -> String {
    let b = bare(block);
    let b = b.strip_prefix("deepslate_").filter(|r| r.ends_with("_ore")).unwrap_or(b);
    match b {
        "iron_ore" => "raw_iron",
        "gold_ore" => "raw_gold",
        "copper_ore" => "raw_copper",
        "diamond_ore" => "diamond",
        "coal_ore" => "coal",
        "emerald_ore" => "emerald",
        "lapis_ore" => "lapis_lazuli",
        "redstone_ore" => "redstone",
        "nether_quartz_ore" => "quartz",
        "nether_gold_ore" => "gold_nugget",
        "stone" => "cobblestone",
        "deepslate" => "cobbled_deepslate",
        "grass_block" | "podzol" | "mycelium" | "dirt_path" | "farmland" | "rooted_dirt" => "dirt",
        other => other,
    }
    .to_owned()
}

/// Walks away from a point forever: no position is ever the goal, so the path always goes on.
struct Branch(GoalRunAway);

impl Goal for Branch {
    fn is_in_goal(&self, _p: Pos) -> bool {
        false
    }
    fn heuristic(&self, p: Pos) -> f64 {
        self.0.heuristic(p)
    }
    fn label(&self) -> String {
        self.0.label()
    }
}

impl MineProcess {
    pub fn new() -> Self {
        Self {
            names: HashSet::new(),
            label: String::new(),
            drops: HashSet::new(),
            desired: 0,
            active: false,
            known: Vec::new(),
            items: Vec::new(),
            blacklist: HashSet::new(),
            branch_point: None,
            next_scan: 0,
            last_break: None,
            loiter_until: 0,
        }
    }

    pub fn mine(&mut self, quantity: usize, names: HashSet<String>, label: String) {
        *self = Self::new();
        self.drops = names.iter().map(|n| format!("minecraft:{}", drop_of(n))).collect();
        self.desired = quantity;
        self.names = names;
        self.label = label;
        self.active = true;
    }

    /// Never mine this block again (#blacklist): the nearest one known.
    pub fn blacklist_nearest(&mut self, from: Pos) -> Option<Pos> {
        let p = self.known.iter().copied().min_by_key(|t| dist_sq(*t, from))?;
        self.blacklist.insert(p);
        self.known.retain(|t| *t != p);
        Some(p)
    }

    pub fn mined(&self) -> usize {
        0
    }

    /// The ore it is after, nearest first, for drawing.
    pub fn targets(&self) -> &[Pos] {
        &self.known
    }

    fn plausible(core: &Core, game: &Game<'_>, p: Pos) -> bool {
        let name = block_name(game, p);
        let facts = game.res.facts().get(game.world.block(p[0], p[1], p[2]));
        facts.hardness >= 0.0 && !core.avoid_breaking.iter().any(|a| a == bare(&name))
    }

    /// MineProcess.prune: forget what is gone, implausible, out of the height limits or blacklisted,
    /// and keep the nearest few.
    fn prune(&mut self, core: &mut Core, game: &mut Game<'_>, feet: Pos) {
        let (lo, hi) = (core.settings.int("minYLevelWhileMining"), core.settings.int("maxYLevelWhileMining"));
        let exposed_only = core.settings.flag("allowOnlyExposedOres");
        let max = core.settings.int("mineMaxOreLocationsCount").clamp(1, 512) as usize;
        let mut seen = HashSet::new();
        let mut keep: Vec<Pos> = Vec::new();
        for p in std::mem::take(&mut self.known) {
            if !seen.insert(p) || self.blacklist.contains(&p) || p[1] < lo || p[1] > hi {
                continue;
            }
            if loaded(game, p) && (!self.names.contains(&block_name(game, p)) || !Self::plausible(core, game, p)) {
                continue;
            }
            keep.push(p);
        }
        if exposed_only {
            let g = &*game;
            let mut terrain = GameTerrain::new(g, &mut core.cells, &core.avoid_breaking, true);
            keep.retain(|p| exposed(&mut terrain, *p, false));
        }
        keep.sort_by_key(|p| dist_sq(*p, feet));
        keep.truncate(max);
        self.known = keep;
    }

    /// MineProcess.addNearby: what a player standing here could see, in a cube of radius 10.
    fn add_nearby(&mut self, core: &mut Core, game: &mut Game<'_>, feet: Pos) {
        let diagonals = core.settings.flag("legitMineIncludeDiagonals");
        let found = core.cache.locations_of(game, &self.names, feet, 10, Some((feet[1] - 10, feet[1] + 10)), 200);
        let g = &*game;
        let mut terrain = GameTerrain::new(g, &mut core.cells, &core.avoid_breaking, true);
        for p in found {
            if !self.known.contains(&p) && exposed(&mut terrain, p, diagonals) {
                self.known.push(p);
            }
        }
    }

    /// MineProcess.coalesce: the goal that gets the player to breaking `p`.
    fn coalesce(core: &Core, game: &Game<'_>, p: Pos) -> GoalRef {
        let above = [p[0], p[1] + 1, p[2]];
        let falling = matches!(bare(&block_name(game, above)), "sand" | "red_sand" | "gravel" | "anvil" | "chipped_anvil" | "damaged_anvil" | "dragon_egg");
        if core.settings.flag("forceInternalMining") {
            if falling { rc(GoalTwoBlocks(p)) } else { rc(GoalThreeBlocks(p)) }
        } else if falling {
            rc(GoalTwoBlocks([p[0], p[1] - 1, p[2]]))
        } else {
            rc(GoalTwoBlocks(p))
        }
    }
}

impl Process for MineProcess {
    fn name(&self) -> &'static str {
        "mine"
    }

    fn display(&self) -> String {
        format!("Mine {}", self.label)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn priority(&self) -> f64 {
        0.5
    }

    fn on_tick(&mut self, core: &mut Core, game: &mut Game<'_>, calc_failed: bool, _safe: bool) -> PathingCommand {
        let tick = game.ticks;
        let Some(feet) = core.player_feet(game) else {
            return PathingCommand::Defer;
        };
        let broken = std::mem::take(&mut core.pathing.broken);
        for (pos, name) in broken {
            if self.names.contains(&name) {
                self.last_break = Some(pos);
                let ms = core.settings.int("mineDropLoiterDurationMSThanksLouca").max(0) as u64;
                self.loiter_until = tick + ms / 50 + 1;
                self.known.retain(|t| *t != pos);
            }
        }
        if self.desired > 0 {
            let have = count_items(game, |n| self.drops.contains(n)) as usize;
            if have >= self.desired {
                core.say(game, &format!("Have {have} valid items"));
                self.active = false;
                return PathingCommand::CancelAndSetGoal(rc(GoalBlock(feet)));
            }
        }
        if calc_failed {
            if !self.known.is_empty() && core.settings.flag("blacklistClosestOnFailure") {
                core.say(game, &format!("Unable to find any path to {}, blacklisting presumably unreachable closest instance...", self.label));
                if let Some(p) = self.known.iter().copied().min_by_key(|t| dist_sq(*t, feet)) {
                    self.blacklist.insert(p);
                }
                let gone: Vec<Pos> = self.blacklist.iter().copied().collect();
                self.known.retain(|k| !gone.contains(k));
            } else {
                core.say(game, &format!("Unable to find any path to {}, canceling mine", self.label));
                self.active = false;
                return PathingCommand::CancelAndSetGoal(rc(GoalBlock(feet)));
            }
        }

        let legit = core.settings.flag("legitMine");
        let interval = core.settings.int("mineGoalUpdateInterval").max(1) as u64;
        if !legit && (tick >= self.next_scan || self.known.is_empty()) {
            self.next_scan = tick + interval;
            let mut found = core.cache.locations_of(game, &self.names, feet, 64, None, 200);
            found.retain(|p| !self.blacklist.contains(p));
            self.known = found;
            self.prune(core, game, feet);
            if self.known.is_empty() && !core.settings.flag("exploreForBlocks") {
                core.say(game, &format!("No locations for {} known, cancelling", self.label));
                self.active = false;
                return PathingCommand::CancelAndSetGoal(rc(GoalBlock(feet)));
            }
        }
        if legit {
            self.add_nearby(core, game, feet);
            self.prune(core, game, feet);
        }

        // What the broken block dropped, for a moment after.
        self.items.clear();
        if core.settings.flag("mineScanDroppedItems") && tick < self.loiter_until {
            if let Some(near) = self.last_break {
                self.items = dropped_items(game)
                    .into_iter()
                    .filter(|(_, n)| self.drops.contains(n.as_str()))
                    .map(|(p, _)| [p[0].floor() as i32, p[1].floor() as i32, p[2].floor() as i32])
                    .filter(|p| dist_sq(*p, near) <= 100)
                    .collect();
            }
        }

        // Shaft mining: an ore known in this column, level with the head or above it, is broken from here.
        if !core.pathing.paused && core.pathing.dig.is_none() && game.player.as_ref().is_some_and(|p| p.ground.on_ground) {
            let shaft = self
                .known
                .iter()
                .copied()
                .filter(|p| p[0] == feet[0] && p[2] == feet[2] && p[1] > feet[1])
                .filter(|p| loaded(game, *p) && self.names.contains(&block_name(game, *p)))
                .min_by_key(|p| dist_sq(*p, feet));
            if let Some(p) = shaft {
                if core.start_dig(game, p) {
                    return PathingCommand::RequestPause;
                }
            }
        }

        // updateGoal
        if !self.known.is_empty() || !self.items.is_empty() {
            let mut goals: Vec<GoalRef> = self.known.iter().map(|p| Self::coalesce(core, game, *p)).collect();
            goals.extend(self.items.iter().map(|p| rc(GoalBlock(*p))));
            let goal = rc(GoalComposite(goals));
            return if legit { PathingCommand::ForceRevalidateGoalAndPath(goal) } else { PathingCommand::RevalidateGoalAndPath(goal) };
        }
        if !legit && !core.settings.flag("exploreForBlocks") {
            self.active = false;
            return PathingCommand::CancelAndSetGoal(rc(GoalBlock(feet)));
        }
        let from = *self.branch_point.get_or_insert(feet);
        let goal = rc(Branch(GoalRunAway { from: vec![from], distance: 1, maintain_y: Some(core.settings.int("legitMineYLevel")) }));
        PathingCommand::RevalidateGoalAndPath(goal)
    }

    fn on_lost_control(&mut self, _core: &mut Core, _game: &mut Game<'_>) {}

    fn cancel(&mut self) {
        self.active = false;
    }
}
