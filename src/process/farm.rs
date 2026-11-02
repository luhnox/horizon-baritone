// baritone.process.FarmProcess: harvest the crops that are ripe, plant the seeds again on the empty
// farmland, and pick up what fell. Wheat, carrots, potatoes, beetroots, nether wart and cocoa are
// ripe at their last growth stage; melons and pumpkins always; sugar cane and cactus above the base.
use super::baritone::Core;
use super::baritone_goals::{rc, Goal, GoalGetToBlock, GoalNear, GoalRef};
use super::baritone_gworld::{block_name, block_state_text, dropped_items, is_air_at};
use super::baritone_inventory as inv;
use super::baritone_pathing::{PathingCommand, Process};
use super::baritone_utils::{dist_sq, up, Pos};
use super::Game;
use std::collections::HashSet;

/// (crop block, growth stage that is ripe, seed item that replants it, block the seed goes on)
const CROPS: &[(&str, i32, &str, &str)] = &[
    ("minecraft:wheat", 7, "minecraft:wheat_seeds", "minecraft:farmland"),
    ("minecraft:carrots", 7, "minecraft:carrot", "minecraft:farmland"),
    ("minecraft:potatoes", 7, "minecraft:potato", "minecraft:farmland"),
    ("minecraft:beetroots", 3, "minecraft:beetroot_seeds", "minecraft:farmland"),
    ("minecraft:nether_wart", 3, "minecraft:nether_wart", "minecraft:soul_sand"),
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Job {
    Harvest(Pos),
    /// Plant on the farmland (or soul sand) block at this position, the item in this hotbar slot.
    Plant(Pos, usize),
}

pub struct FarmProcess {
    active: bool,
    center: Pos,
    range: i32,
    job: Option<Job>,
    next_scan: u64,
    loiter_until: u64,
    last_break: Option<Pos>,
    harvested: u32,
    planted: u32,
    jobs: Vec<Job>,
    told_idle: bool,
    tries: u32,
}

impl FarmProcess {
    pub fn new() -> Self {
        Self {
            active: false,
            center: [0, 0, 0],
            range: 40,
            job: None,
            next_scan: 0,
            loiter_until: 0,
            last_break: None,
            harvested: 0,
            planted: 0,
            jobs: Vec::new(),
            told_idle: false,
            tries: 0,
        }
    }

    /// Farm within `range` blocks of `center`; a range of 0 is as far as the world is scanned.
    pub fn farm(&mut self, center: Pos, range: i32) {
        *self = Self::new();
        self.center = center;
        self.range = if range <= 0 { 256 } else { range.clamp(4, 256) };
        self.active = true;
    }
}

/// The growth stage in a state text (`age=7`), if it has one.
fn age_of(state: &str) -> Option<i32> {
    let at = state.find("age=")? + 4;
    state[at..].split(|c: char| !c.is_ascii_digit()).next()?.parse().ok()
}

/// Whether the crop block at `pos` is ready to harvest.
fn ripe(game: &Game<'_>, pos: Pos, name: &str) -> bool {
    match name {
        "minecraft:pumpkin" | "minecraft:melon" => true,
        "minecraft:sugar_cane" | "minecraft:cactus" => block_name(game, up(pos, -1)) == name,
        "minecraft:cocoa" => age_of(&block_state_text(game, pos)) == Some(2),
        _ => CROPS
            .iter()
            .find(|c| c.0 == name)
            .is_some_and(|c| age_of(&block_state_text(game, pos)).is_some_and(|a| a >= c.1)),
    }
}

impl Process for FarmProcess {
    fn name(&self) -> &'static str {
        "farm"
    }

    fn display(&self) -> String {
        format!("Farming: harvested {}, planted {}", self.harvested, self.planted)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn priority(&self) -> f64 {
        2.5
    }

    fn on_tick(&mut self, core: &mut Core, game: &mut Game<'_>, _calc_failed: bool, _safe: bool) -> PathingCommand {
        let tick = game.ticks;
        let Some(feet) = core.player_feet(game) else {
            return PathingCommand::Defer;
        };
        let broken = std::mem::take(&mut core.pathing.broken);
        for (pos, _) in broken {
            self.harvested += 1;
            self.last_break = Some(pos);
            self.loiter_until = tick + 15;
        }
        // Pick up what fell.
        if tick < self.loiter_until {
            if let Some(near) = self.last_break {
                let item = dropped_items(game)
                    .into_iter()
                    .map(|(p, _)| [p[0].floor() as i32, p[1].floor() as i32, p[2].floor() as i32])
                    .filter(|p| dist_sq(*p, near) <= 49 && *p != feet)
                    .min_by_key(|p| dist_sq(*p, feet));
                if let Some(item) = item {
                    return PathingCommand::SetGoalAndPath(rc(GoalNear(item, 0)));
                }
            }
        }
        if tick >= self.next_scan {
            self.next_scan = tick + 20;
            self.jobs = self.scan(core, game);
            self.job = self.jobs.first().copied();
            self.tries = 0;
        }
        // A job that is done (the crop is gone, the seed is in) goes.
        if let Some(job) = self.job {
            let gone = match job {
                Job::Harvest(p) => is_air_at(game, p),
                Job::Plant(base, _) => !is_air_at(game, up(base, 1)),
            };
            if gone {
                if matches!(job, Job::Plant(..)) {
                    self.planted += 1;
                }
                self.jobs.retain(|j| *j != job);
                self.job = self.jobs.first().copied();
                self.tries = 0;
            }
        }
        let Some(job) = self.job else {
            if !self.told_idle {
                self.told_idle = true;
                core.say(game, "Nothing ripe to harvest and nothing to plant: waiting for the crops to grow.");
            }
            return PathingCommand::Defer;
        };
        self.told_idle = false;

        match job {
            Job::Harvest(p) => {
                if GoalGetToBlock(p).is_in_goal(feet) && core.pathing.dig.is_none() && !core.pathing.paused {
                    self.tries += 1;
                    if self.tries > 6 || !core.start_dig(game, p) {
                        self.jobs.retain(|j| *j != job);
                        self.job = self.jobs.first().copied();
                    }
                }
                PathingCommand::RevalidateGoalAndPath(rc(GoalGetToBlock(p)))
            }
            Job::Plant(base, slot) => {
                // Stand within reach of the soil's top face.
                let above = up(base, 1);
                let close = dist_sq(feet, above) <= 9;
                if close {
                    inv::select(game, slot);
                    self.tries += 1;
                    if core.click_block(game, base, [0, 1, 0]) {
                        self.next_scan = tick + 3;
                    }
                    if self.tries > 30 {
                        self.jobs.retain(|j| *j != job);
                        self.job = self.jobs.first().copied();
                        self.tries = 0;
                    }
                }
                let goal: GoalRef = rc(GoalNear(above, 2));
                PathingCommand::RevalidateGoalAndPath(goal)
            }
        }
    }

    fn on_lost_control(&mut self, _core: &mut Core, _game: &mut Game<'_>) {}

    fn cancel(&mut self) {
        self.active = false;
    }
}

impl FarmProcess {
    /// The jobs around the centre, nearest first: ripe crops, then empty soil to plant.
    fn scan(&self, core: &mut Core, game: &Game<'_>) -> Vec<Job> {
        let feet = core.player_feet(game).unwrap_or(self.center);
        let mut names: HashSet<String> = CROPS.iter().map(|c| c.0.to_owned()).collect();
        for extra in ["minecraft:pumpkin", "minecraft:melon", "minecraft:sugar_cane", "minecraft:cactus", "minecraft:cocoa"] {
            names.insert(extra.to_owned());
        }
        let mut jobs: Vec<(i64, Job)> = Vec::new();
        for p in core.cache.locations_of(game, &names, self.center, self.range, Some((self.center[1] - 6, self.center[1] + 8)), 300) {
            let name = block_name(game, p);
            if (name == "minecraft:nether_wart") && !core.settings.flag("replantNetherWart") && !ripe(game, p, &name) {
                continue;
            }
            if ripe(game, p, &name) {
                jobs.push((i64::from(dist_sq(feet, p)), Job::Harvest(p)));
            }
        }
        if core.settings.flag("replantCrops") {
            // Soil with nothing growing, and a seed for it in the hotbar.
            for (_, _, seed, soil) in CROPS {
                if *seed == "minecraft:nether_wart" && !core.settings.flag("replantNetherWart") {
                    continue;
                }
                let Some(slot) = inv::slot_of(game, seed) else {
                    continue;
                };
                let soils: HashSet<String> = [(*soil).to_owned()].into_iter().collect();
                for base in core.cache.locations_of(game, &soils, self.center, self.range, Some((self.center[1] - 6, self.center[1] + 8)), 300) {
                    if is_air_at(game, up(base, 1)) {
                        jobs.push((i64::from(dist_sq(feet, base)), Job::Plant(base, slot)));
                    }
                }
            }
        }
        jobs.sort_by_key(|j| j.0);
        jobs.into_iter().map(|j| j.1).collect()
    }
}
