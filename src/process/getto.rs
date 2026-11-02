// baritone.process.GetToBlockProcess: walk to the nearest block of a kind, to stand next to it.
use super::baritone::Core;
use super::baritone_goals::{rc, Goal, GoalComposite, GoalGetToBlock, GoalRef};
use super::baritone_pathing::{PathingCommand, Process};
use super::baritone_utils::Pos;
use super::Game;
use std::collections::HashSet;

pub struct GetToBlockProcess {
    names: HashSet<String>,
    label: String,
    active: bool,
    known: Vec<Pos>,
    next_scan: u64,
    /// Blocks that cannot be reached, never tried again (#blacklist).
    blacklist: HashSet<Pos>,
}

impl GetToBlockProcess {
    pub fn new() -> Self {
        Self { names: HashSet::new(), label: String::new(), active: false, known: Vec::new(), next_scan: 0, blacklist: HashSet::new() }
    }

    pub fn get_to_block(&mut self, names: HashSet<String>, label: String) {
        self.names = names;
        self.label = label;
        self.known.clear();
        self.blacklist.clear();
        self.next_scan = 0;
        self.active = true;
    }

    pub fn running(&self) -> bool {
        self.active
    }

    /// Give up on the nearest known block. False when none is known.
    pub fn blacklist_closest(&mut self, from: Pos) -> bool {
        let nearest = self.known.iter().copied().min_by_key(|p| super::baritone_utils::dist_sq(*p, from));
        match nearest {
            Some(p) => {
                self.blacklist.insert(p);
                self.known.retain(|k| *k != p);
                true
            }
            None => false,
        }
    }
}

impl Process for GetToBlockProcess {
    fn name(&self) -> &'static str {
        "get to block"
    }

    fn display(&self) -> String {
        format!("Getting to {}", self.label)
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn priority(&self) -> f64 {
        2.0
    }

    fn on_tick(&mut self, core: &mut Core, game: &mut Game<'_>, calc_failed: bool, _safe: bool) -> PathingCommand {
        let tick = game.ticks;
        let Some(feet) = core.player_feet(game) else {
            return PathingCommand::Defer;
        };
        if tick >= self.next_scan || self.known.is_empty() {
            self.next_scan = tick + 20;
            let found = core.cache.locations_of(game, &self.names, feet, 128, None, 50);
            self.known = found.into_iter().filter(|p| !self.blacklist.contains(p)).collect();
        }
        if self.known.is_empty() {
            core.say_red(game, &format!("I could not find {} in the loaded world around you.", self.label));
            self.active = false;
            return PathingCommand::Defer;
        }
        if self.known.iter().any(|p| GoalGetToBlock(*p).is_in_goal(feet)) {
            core.say_green(game, &format!("Arrived next to {}.", self.label));
            self.active = false;
            return PathingCommand::CancelAndSetGoal(rc(GoalGetToBlock(feet)));
        }
        if calc_failed {
            // The nearest one cannot be reached: try the others.
            self.known.remove(0);
            if self.known.is_empty() {
                core.say_red(game, &format!("There is no way to any {}.", self.label));
                self.active = false;
                return PathingCommand::Defer;
            }
        }
        let goals: Vec<GoalRef> = self.known.iter().map(|p| rc(GoalGetToBlock(*p))).collect();
        PathingCommand::RevalidateGoalAndPath(rc(GoalComposite(goals)))
    }

    fn on_lost_control(&mut self, _core: &mut Core, _game: &mut Game<'_>) {
        self.active = false;
    }

    fn cancel(&mut self) {
        self.active = false;
    }
}
