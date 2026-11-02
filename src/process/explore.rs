// baritone.process.ExploreProcess: head for the nearest part of the world that has not been loaded
// yet, nearest to where exploring began, and keep going outwards. A chunk counts as explored once it
// has been loaded, so what a filter file lists (chunks already explored in an earlier world) is
// skipped too.
use super::baritone::Core;
use super::baritone_goals::{rc, GoalComposite, GoalRef, GoalXZ};
use super::baritone_pathing::{PathingCommand, Process};
use super::Game;
use std::collections::HashSet;

pub struct ExploreProcess {
    origin: Option<(i32, i32)>,
    filter: HashSet<(i32, i32)>,
    invert_filter: bool,
    active: bool,
    next_goal: u64,
    goal: Option<GoalRef>,
}

impl ExploreProcess {
    pub fn new() -> Self {
        Self { origin: None, filter: HashSet::new(), invert_filter: false, active: false, next_goal: 0, goal: None }
    }

    pub fn explore(&mut self, x: i32, z: i32) {
        self.origin = Some((x, z));
        self.active = true;
        self.goal = None;
        self.next_goal = 0;
    }

    /// Chunks (x, z) already explored, from a file. `invert`: only these are worth exploring.
    pub fn set_filter(&mut self, chunks: HashSet<(i32, i32)>, invert: bool) {
        self.filter = chunks;
        self.invert_filter = invert;
    }

    pub fn filter_len(&self) -> usize {
        self.filter.len()
    }

    fn skipped(&self, c: (i32, i32)) -> bool {
        if self.filter.is_empty() {
            return false;
        }
        self.filter.contains(&c) != self.invert_filter
    }
}

/// Read a filter file: chunk coordinates `[{"x":0,"z":0},...]`. The error is what #explorefilter says.
pub fn read_filter(path: &std::path::Path) -> Result<HashSet<(i32, i32)>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound { "File not found".to_owned() } else { format!("Cannot read the file: {e}") }
    })?;
    let t = text.trim();
    if !(t.starts_with('[') && t.ends_with(']')) {
        return Err("Invalid JSON syntax".into());
    }
    Ok(chunk_pairs(t))
}

/// The pairs of whole numbers in a text, in order: `[[1,2],[3,4]]` or `{"x":1,"z":2}` lines both read.
pub fn chunk_pairs(text: &str) -> HashSet<(i32, i32)> {
    let mut numbers: Vec<i32> = Vec::new();
    let mut current = String::new();
    let flush = |current: &mut String, numbers: &mut Vec<i32>| {
        if let Ok(n) = current.parse::<i32>() {
            numbers.push(n);
        }
        current.clear();
    };
    for c in text.chars() {
        if c.is_ascii_digit() || (c == '-' && current.is_empty()) {
            current.push(c);
        } else {
            flush(&mut current, &mut numbers);
        }
    }
    flush(&mut current, &mut numbers);
    numbers.chunks_exact(2).map(|p| (p[0], p[1])).collect()
}

impl Process for ExploreProcess {
    fn name(&self) -> &'static str {
        "explore"
    }

    fn display(&self) -> String {
        match self.origin {
            Some((x, z)) => format!("Exploring around {x}, {z}"),
            None => "Not exploring".into(),
        }
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn priority(&self) -> f64 {
        0.2
    }

    fn on_tick(&mut self, core: &mut Core, game: &mut Game<'_>, calc_failed: bool, _safe: bool) -> PathingCommand {
        let Some(origin) = self.origin else {
            return PathingCommand::Defer;
        };
        let tick = game.ticks;
        core.cache.note_chunks(game);
        if self.goal.is_none() || tick >= self.next_goal || calc_failed {
            self.next_goal = tick + 40;
            let explored = &core.cache.explored;
            // Unexplored chunks next to explored ones: where the edge of the known world is.
            let mut frontier: HashSet<(i32, i32)> = HashSet::new();
            for &(cx, cz) in explored.iter() {
                for dx in -1..=1 {
                    for dz in -1..=1 {
                        let c = (cx + dx, cz + dz);
                        if !explored.contains(&c) && !self.skipped(c) {
                            frontier.insert(c);
                        }
                    }
                }
            }
            if frontier.is_empty() {
                core.say(game, "There is nothing left to explore from here.");
                self.active = false;
                return PathingCommand::Defer;
            }
            let (ox, oz) = (origin.0 >> 4, origin.1 >> 4);
            let mut sorted: Vec<(i32, i32)> = frontier.into_iter().collect();
            sorted.sort_by_key(|c| (i64::from(c.0 - ox).pow(2) + i64::from(c.1 - oz).pow(2), c.0, c.1));
            let n = core.settings.int("exploreChunkSetMinimumSize").clamp(1, 64) as usize;
            let goals: Vec<GoalRef> = sorted.into_iter().take(n).map(|(cx, cz)| rc(GoalXZ(cx * 16 + 8, cz * 16 + 8))).collect();
            self.goal = Some(rc(GoalComposite(goals)));
        }
        match &self.goal {
            Some(g) => PathingCommand::RevalidateGoalAndPath(g.clone()),
            None => PathingCommand::Defer,
        }
    }

    fn on_lost_control(&mut self, _core: &mut Core, _game: &mut Game<'_>) {}

    fn cancel(&mut self) {
        self.active = false;
    }
}
