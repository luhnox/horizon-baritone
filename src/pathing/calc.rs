// baritone.pathing.calc.AStarPathFinder: the search. It runs a few thousand nodes a tick, so a long
// path is found over several ticks and the game does not stall, and it can be asked at any time for
// the best path so far.
use super::baritone_goals::GoalRef;
use super::baritone_legacy_helper::{stand, CalcSettings, Kind, Terrain};
use super::baritone_legacy_movement::{moves, Mv};
use super::baritone_utils::{up, Pos};
use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
};

#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    /// From the start to the end, both included.
    pub path: Vec<Pos>,
    /// How each node of the path is reached (the first is `Walk`).
    pub moves: Vec<Mv>,
    /// The path reaches the goal. False: it goes as near as the search could get.
    pub complete: bool,
    pub nodes: usize,
    /// The ticks the path takes, as the search costed it.
    pub cost: f64,
}

struct Entry {
    f: f64,
    g: f64,
    pos: Pos,
}
impl PartialEq for Entry {
    fn eq(&self, o: &Self) -> bool {
        self.f == o.f
    }
}
impl Eq for Entry {}
impl PartialOrd for Entry {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Entry {
    // A BinaryHeap pops the greatest: the smallest f, then the greatest g (deeper is nearer).
    fn cmp(&self, o: &Self) -> Ordering {
        o.f.total_cmp(&self.f).then(self.g.total_cmp(&o.g))
    }
}

pub enum Status {
    Running,
    /// The goal was reached, or the search ran out of room and this is the nearest it got.
    Done(Plan),
    /// It cannot move at all.
    Failed,
}

pub struct AStar {
    goal: GoalRef,
    settings: CalcSettings,
    start: Pos,
    best: HashMap<Pos, (f64, Option<Pos>, Kind, Mv)>,
    open: BinaryHeap<Entry>,
    nearest: (f64, Pos),
    pub nodes: usize,
    next: Vec<(Pos, f64, Mv)>,
}

impl AStar {
    /// None when the player cannot stand at `start` or anywhere just below it (mid-fall, or on a
    /// block the classifier did not like).
    pub fn new<T: Terrain + ?Sized>(t: &mut T, goal: GoalRef, start: Pos, settings: CalcSettings) -> Option<Self> {
        let start_kind = stand(t, start).or_else(|| {
            (1..=settings.max_fall_into_water).find_map(|k| stand(t, up(start, -k)).map(|_| Kind::Ground))
        })?;
        let mut best = HashMap::new();
        best.insert(start, (0.0, None, start_kind, Mv::Walk));
        let mut open = BinaryHeap::new();
        let h = goal.heuristic(start);
        open.push(Entry { f: h, g: 0.0, pos: start });
        Some(Self { goal, settings, start, best, open, nearest: (h, start), nodes: 0, next: Vec::with_capacity(24) })
    }

    pub fn start(&self) -> Pos {
        self.start
    }

    /// Expand up to `budget` nodes.
    pub fn run<T: Terrain + ?Sized>(&mut self, t: &mut T, budget: usize) -> Status {
        let mut spent = 0;
        while let Some(Entry { g, pos, .. }) = self.open.pop() {
            let Some(&(known, _, kind, _)) = self.best.get(&pos) else {
                continue;
            };
            if g > known {
                continue;
            }
            if self.goal.is_in_goal(pos) {
                let (path, moves) = self.trace(pos);
                return Status::Done(Plan { path, moves, complete: true, nodes: self.nodes, cost: g });
            }
            self.nodes += 1;
            if self.nodes > self.settings.max_nodes {
                return self.give_up();
            }
            moves(t, pos, kind, &self.settings, &mut self.next);
            for i in 0..self.next.len() {
                let (to, cost, mv) = self.next[i];
                let g2 = g + cost;
                if self.best.get(&to).is_some_and(|&(old, ..)| old <= g2) {
                    continue;
                }
                let kind2 = stand(t, to).unwrap_or(Kind::Ground);
                self.best.insert(to, (g2, Some(pos), kind2, mv));
                let h = self.goal.heuristic(to);
                // The node nearest the goal, preferring the cheaper way there.
                if h < self.nearest.0 - 1e-9
                    || (h - self.nearest.0).abs() < 1e-9 && g2 < self.best.get(&self.nearest.1).map_or(f64::MAX, |b| b.0)
                {
                    self.nearest = (h, to);
                }
                self.open.push(Entry { f: g2 + h, g: g2, pos: to });
            }
            spent += 1;
            if spent >= budget {
                return Status::Running;
            }
        }
        self.give_up()
    }

    fn give_up(&self) -> Status {
        match self.best_so_far() {
            Some(plan) => Status::Done(plan),
            None => Status::Failed,
        }
    }

    /// The path to the node nearest the goal so far, if the search has got anywhere.
    pub fn best_so_far(&self) -> Option<Plan> {
        if self.nearest.1 == self.start {
            return None;
        }
        let (path, moves) = self.trace(self.nearest.1);
        let cost = self.best.get(&self.nearest.1).map_or(0.0, |b| b.0);
        Some(Plan { path, moves, complete: false, nodes: self.nodes, cost })
    }

    fn trace(&self, end: Pos) -> (Vec<Pos>, Vec<Mv>) {
        let mut path = vec![end];
        let mut moves = vec![self.best.get(&end).map_or(Mv::Walk, |b| b.3)];
        let mut at = end;
        while let Some(&(_, Some(parent), _, _)) = self.best.get(&at) {
            path.push(parent);
            moves.push(self.best.get(&parent).map_or(Mv::Walk, |b| b.3));
            at = parent;
        }
        path.reverse();
        moves.reverse();
        if let Some(first) = moves.first_mut() {
            *first = Mv::Walk;
        }
        (path, moves)
    }
}

/// The whole search at once: the path from `start` to the goal; when it runs out of nodes or room,
/// the path to the node nearest the goal (`complete` false). `None` when it cannot move at all.
pub fn find_path<T: Terrain + ?Sized>(t: &mut T, start: Pos, goal: GoalRef, settings: &CalcSettings) -> Option<Plan> {
    let mut search = AStar::new(t, goal, start, settings.clone())?;
    loop {
        match search.run(t, 4096) {
            Status::Running => {}
            Status::Done(plan) => return Some(plan),
            Status::Failed => return None,
        }
    }
}
