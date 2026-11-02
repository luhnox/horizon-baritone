// baritone.pathing.path.PathExecutor and the movements' updateState: what to hold, tick by tick, to
// walk a path. It only reads the world and the player (a `Snapshot`) and answers with the controls
// to hold or the one thing to do next, so everything here is tested on its own.
use super::baritone_legacy_helper::{Cell, Terrain};
use super::baritone_legacy_movement::{required_free, Mv};
use super::baritone_utils::{centre, feet_of, up, yaw_to, Pos};

/// What the game says about the player, once a tick.
#[derive(Clone, Copy, Debug)]
pub struct Snapshot {
    pub pos: [f64; 3],
    pub on_ground: bool,
    pub in_water: bool,
    /// Inside a ladder or a vine block.
    pub climbing: bool,
    /// Pushing against something sideways.
    pub blocked: bool,
    pub tick: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Controls {
    pub forward: bool,
    pub sprint: bool,
    pub jump: bool,
    pub sneak: bool,
    /// The direction to face, degrees; 0 faces south, 90 west.
    pub yaw: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    Go(Controls),
    Arrived,
    /// The path no longer fits what is happening: plan again from here.
    Replan(&'static str),
    /// A block is in the way: break it, then call `step` again.
    Dig(Pos),
    /// Put a block at `pos` by clicking the `face` of `against`, standing on the edge of a gap
    /// (sneaking, so as not to fall in), and call `step` again once it is there.
    Place { pos: Pos, against: Pos, face: [i32; 3] },
    /// Jump, and place a block at `pos` (the cell the feet were in) against the top of `against`.
    Pillar { pos: Pos, against: Pos },
    /// A closed door or gate is in the way: open it with a click.
    Interact(Pos),
}

pub struct PathExecutor {
    pub path: Vec<Pos>,
    pub moves: Vec<Mv>,
    pub index: usize,
    progress_tick: u64,
    best_distance: f64,
    airborne_since: Option<u64>,
    /// A long jump: the run-up has been taken.
    run_started: bool,
    run_index: usize,
}

impl PathExecutor {
    pub fn new(path: Vec<Pos>, tick: u64) -> Self {
        let moves = vec![Mv::Walk; path.len()];
        Self::with_moves(path, moves, tick)
    }

    pub fn with_moves(path: Vec<Pos>, moves: Vec<Mv>, tick: u64) -> Self {
        Self {
            path,
            moves,
            index: 0,
            progress_tick: tick,
            best_distance: f64::INFINITY,
            airborne_since: None,
            run_started: false,
            run_index: usize::MAX,
        }
    }

    pub fn last(&self) -> Option<Pos> {
        self.path.last().copied()
    }

    /// The nodes still to walk, for drawing.
    pub fn remaining(&self) -> &[Pos] {
        &self.path[self.index.min(self.path.len())..]
    }

    /// The move that reaches node `i`.
    fn mv(&self, i: usize) -> Mv {
        self.moves.get(i).copied().unwrap_or(Mv::Walk)
    }

    /// Node `i` is reached by placing a block and the block is not there yet: being at its height
    /// in mid-jump, or standing over the edge where its floor will be, is not being there.
    fn pending(&self, i: usize, t: &mut dyn Terrain) -> bool {
        if i == 0 {
            return false;
        }
        match self.mv(i) {
            Mv::Pillar => !t.cell(self.path[i - 1]).holds(),
            Mv::Bridge => !t.cell(up(self.path[i], -1)).holds(),
            _ => false,
        }
    }

    pub fn step(&mut self, s: &Snapshot, t: &mut dyn Terrain) -> Step {
        let here = feet_of(s.pos);

        // Where on the path are we? Further along counts; so do the next two nodes.
        let window_end = (self.index + 3).min(self.path.len());
        let found = (self.index..window_end)
            .rev()
            .find(|&i| self.path[i] == here && !self.pending(i, t));
        if let Some(i) = found {
            if i > self.index {
                self.index = i;
                self.progress_tick = s.tick;
                self.best_distance = f64::INFINITY;
            }
        }

        let grounded = s.on_ground || s.in_water || s.climbing;
        match grounded {
            true => self.airborne_since = None,
            false => {
                self.airborne_since.get_or_insert(s.tick);
            }
        }

        let Some(&cur) = self.path.get(self.index) else {
            return Step::Arrived;
        };
        if self.index + 1 >= self.path.len() {
            // On the last node, or close enough standing on it.
            let c = centre(cur);
            let d = (s.pos[0] - c[0]).hypot(s.pos[2] - c[1]);
            if here == cur || (d < 0.6 && (s.pos[1] - f64::from(cur[1])).abs() < 1.2) {
                return Step::Arrived;
            }
        }
        let next = self.path.get(self.index + 1).copied().unwrap_or(cur);
        let mv = self.mv(self.index + 1);

        // Off the path: on solid footing, but not at a node of this stretch.
        if grounded && !self.path[self.index..window_end].contains(&here) {
            // Backing up for a long jump is a few steps off the node.
            let radius = if mv == Mv::Parkour { 1.8 } else { 1.3 };
            let near = self.path[self.index..window_end].iter().any(|n| {
                let c = centre(*n);
                (s.pos[0] - c[0]).hypot(s.pos[2] - c[1]) < radius && (s.pos[1] - f64::from(n[1])).abs() < 1.6
            });
            if !near {
                return Step::Replan("off the path");
            }
        }
        if self.airborne_since.is_some_and(|t| s.tick.saturating_sub(t) > 80) {
            return Step::Replan("in the air too long");
        }

        // Putting a block down: bridge from the edge, or pillar from a jump.
        if next != cur {
            match mv {
                Mv::Bridge if here == cur || here == next => {
                    let floor = up(next, -1);
                    if !t.cell(floor).holds() {
                        self.progress_tick = s.tick;
                        let face = [next[0] - cur[0], 0, next[2] - cur[2]];
                        return Step::Place { pos: floor, against: up(cur, -1), face };
                    }
                }
                Mv::Pillar if (here == cur || here == next) && !t.cell(cur).holds() => {
                    self.progress_tick = s.tick;
                    return Step::Pillar { pos: cur, against: up(cur, -1) };
                }
                _ => {}
            }
        }

        // Something in the way of the next step: open it or break it from here, if we are at this node.
        if here == cur && next != cur {
            let mut cells = required_free(cur, next);
            // Going down, the cells are cleared from the top: the block under the new feet cell is
            // only in sight once the ones above it are gone.
            if next[1] < cur[1] {
                cells.reverse();
            }
            let first = if next[1] < cur[1] { cells.len().saturating_sub(1) } else { 0 };
            for (i, c) in cells.into_iter().enumerate() {
                match t.cell(c) {
                    Cell::Air | Cell::Climb | Cell::Water => {}
                    Cell::Low if i == first => {}
                    Cell::Door => {
                        self.progress_tick = s.tick;
                        return Step::Interact(c);
                    }
                    Cell::Solid => {
                        self.progress_tick = s.tick;
                        return Step::Dig(c);
                    }
                    _ => return Step::Replan("blocked"),
                }
            }
        }

        if mv == Mv::Parkour && next != cur {
            return self.parkour(s, cur, next);
        }

        let target = centre(next);
        let dx = target[0] - s.pos[0];
        let dz = target[1] - s.pos[2];
        let distance = dx.hypot(dz);

        // Progress: nearer the node, or on to the next one.
        if distance < self.best_distance - 0.02 {
            self.best_distance = distance;
            self.progress_tick = s.tick;
        }
        if s.tick.saturating_sub(self.progress_tick) > 60 && grounded {
            return Step::Replan("not getting anywhere");
        }

        let dy = next[1] - cur[1];
        let mut c = Controls {
            forward: true,
            yaw: if distance > 0.05 { yaw_to(dx, dz) } else { 0.0 },
            ..Controls::default()
        };
        if distance <= 0.05 {
            c.forward = false;
        }

        if s.climbing {
            // Hold forward into the ladder and climb with jump; going down needs nothing.
            c.jump = dy > 0 || next == cur;
            c.forward = dy >= 0 || distance > 0.3;
            if dy > 0 && distance < 0.35 {
                c.forward = false;
            }
        } else if s.in_water {
            c.jump = dy >= 0;
            c.sprint = false;
        } else {
            // Step up: jump when we reach the edge of the block we are leaving.
            if dy > 0 && s.on_ground && distance < 1.4 && mv != Mv::Pillar {
                c.jump = true;
            }
            // Pushing against something while standing: a jump clears an unseen lip.
            if s.blocked && s.on_ground {
                c.jump = true;
            }
            // Going down: walk off, no jumping.
            if dy < 0 && s.on_ground {
                c.jump = false;
            }
            // Sprint down a straight run, not into a turn, a step up or a drop.
            let straight = self
                .path
                .get(self.index + 2)
                .is_none_or(|after| after[1] == next[1] && (after[0] - next[0] == next[0] - cur[0]) && (after[2] - next[2] == next[2] - cur[2]))
                && self.mv(self.index + 2) == Mv::Walk;
            c.sprint = s.on_ground && dy <= 0 && straight && distance > 0.8 && mv == Mv::Walk;
        }
        Step::Go(c)
    }

    /// A sprint jump from `cur` to `next`, a straight line along one axis. A long one backs up one
    /// block first for the run, then runs, then jumps from the far edge of the block.
    fn parkour(&mut self, s: &Snapshot, cur: Pos, next: Pos) -> Step {
        let (dx, dz) = ((next[0] - cur[0]).signum(), (next[2] - cur[2]).signum());
        let gap = (next[0] - cur[0]).abs().max((next[2] - cur[2]).abs()) - 1;
        let rise = next[1] - cur[1];
        let c = centre(cur);
        let off = (s.pos[0] - c[0]) * f64::from(dx) + (s.pos[2] - c[1]) * f64::from(dz);
        let need_run = gap >= 3 || (rise > 0 && gap >= 2);
        if self.run_index != self.index {
            self.run_index = self.index;
            self.run_started = false;
            self.progress_tick = s.tick;
        }
        let toward = centre(next);
        let yaw = yaw_to(toward[0] - s.pos[0], toward[1] - s.pos[2]);
        if !s.on_ground {
            // In the air: keep going the same way.
            return Step::Go(Controls { forward: true, sprint: true, yaw, ..Controls::default() });
        }
        if s.tick.saturating_sub(self.progress_tick) > 100 {
            return Step::Replan("could not make the jump");
        }
        if need_run && !self.run_started {
            if off > -1.2 {
                let behind = [f64::from(cur[0] - dx) + 0.5, f64::from(cur[2] - dz) + 0.5];
                let yaw_back = yaw_to(behind[0] - s.pos[0], behind[1] - s.pos[2]);
                return Step::Go(Controls { forward: true, yaw: yaw_back, ..Controls::default() });
            }
            self.run_started = true;
        }
        let jump = off >= 0.3;
        if jump {
            self.run_started = true;
        }
        Step::Go(Controls { forward: true, sprint: true, jump, yaw, ..Controls::default() })
    }
}
