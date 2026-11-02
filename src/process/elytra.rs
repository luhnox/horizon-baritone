// baritone.process.ElytraProcess: fly to a place on elytra. Take off, steer to the goal, fire a rocket
// when the speed drops, keep above the highest thing ahead, glide down on the approach, and walk the
// last stretch. It flies a straight line at a safe height rather than searching a route, so it is for
// open country, not for threading a canyon.
use super::baritone::Core;
use super::baritone_goals::{rc, GoalXZ};
use super::baritone_gworld::{column_top, player_pos};
use super::baritone_inventory as inv;
use super::baritone_look as look;
use super::baritone_pathing::{PathingCommand, Process};
use super::baritone_utils::yaw_to;
use super::Game;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Fly,
    /// Down, and close: walk the rest.
    Walk,
}

/// How many blocks above the highest thing ahead to fly.
const CRUISE_CLEARANCE: f64 = 6.0;

pub struct ElytraProcess {
    goal: Option<[i32; 2]>,
    phase: Phase,
    flew: bool,
    last_boost: u64,
    started: u64,
    attempts: u32,
}

impl ElytraProcess {
    pub fn new() -> Self {
        Self { goal: None, phase: Phase::Fly, flew: false, last_boost: 0, started: 0, attempts: 0 }
    }

    /// Start again towards the same goal. False when there is none.
    pub fn restart(&mut self, tick: u64) -> bool {
        match self.goal {
            Some([x, z]) => {
                self.fly_to(x, z, tick);
                true
            }
            None => false,
        }
    }

    pub fn fly_to(&mut self, x: i32, z: i32, tick: u64) {
        *self = Self { goal: Some([x, z]), started: tick, ..Self::new() };
    }
}

impl Process for ElytraProcess {
    fn name(&self) -> &'static str {
        "elytra"
    }

    fn display(&self) -> String {
        match self.goal {
            Some([x, z]) => format!("Flying to x {x}, z {z}"),
            None => "Not flying".into(),
        }
    }

    fn is_active(&self) -> bool {
        self.goal.is_some()
    }

    fn priority(&self) -> f64 {
        2.8
    }

    fn on_tick(&mut self, core: &mut Core, game: &mut Game<'_>, _calc_failed: bool, _safe: bool) -> PathingCommand {
        let Some(goal) = self.goal else {
            return PathingCommand::Defer;
        };
        let tick = game.ticks;
        let Some(pos) = player_pos(game) else {
            return PathingCommand::Defer;
        };
        let (vel, flying, on_ground) = match game.player.as_ref() {
            Some(p) => ([p.velocity.x, p.velocity.y, p.velocity.z], p.fall_flying, p.ground.on_ground),
            None => return PathingCommand::Defer,
        };
        let to = [f64::from(goal[0]) + 0.5 - pos[0], f64::from(goal[1]) + 0.5 - pos[2]];
        let dist = to[0].hypot(to[1]);
        let heading = yaw_to(to[0], to[1]);

        if self.phase == Phase::Walk {
            let feet = core.player_feet(game).unwrap_or([0, 0, 0]);
            let g = rc(GoalXZ(goal[0], goal[1]));
            if g.is_in_goal(feet) {
                core.say_green(game, "Landed at the goal.");
                self.goal = None;
            }
            return PathingCommand::SetGoalAndPath(g);
        }

        if flying {
            self.flew = true;
            core.input.only(game, false, false, false);
            let speed = vel[0].hypot(vel[2]);
            let (x, y, z) = (pos[0].floor() as i32, pos[1].floor() as i32, pos[2].floor() as i32);
            let above_ground = column_top(game, x, z, y).map_or(200.0, |top| pos[1] - f64::from(top + 1));
            // The highest thing along the way for the next 48 blocks: stay clear of it.
            let r = f64::from(heading).to_radians();
            let (dx, dz) = (-r.sin(), r.cos());
            let mut highest: Option<i32> = None;
            for k in (6..=48).step_by(6) {
                let (sx, sz) = ((pos[0] + dx * f64::from(k)).floor() as i32, (pos[2] + dz * f64::from(k)).floor() as i32);
                if let Some(top) = column_top(game, sx, sz, y + 40) {
                    highest = Some(highest.map_or(top, |h| h.max(top)));
                }
            }
            let clearance = CRUISE_CLEARANCE;
            let want = highest.map(|h| f64::from(h) + clearance);
            let pitch = if dist < 60.0 {
                // Coming in: a gentle descent, and level off over the ground.
                if above_ground > 14.0 { 28.0 } else if above_ground > 5.0 { 10.0 } else { -14.0 }
            } else if want.is_some_and(|w| pos[1] < w) || above_ground < 8.0 {
                -42.0
            } else if want.is_some_and(|w| pos[1] > w + 14.0) {
                14.0
            } else {
                -4.0
            };
            look::turn_to(game, heading, 5.0);
            look::pitch_to(game, pitch as f32, 6.0);
            // A rocket when the speed drops, never while a block is close enough for the click to land on it.
            let slow = core.settings.float("elytraFireworkSpeed");
            // Looking up, a click cannot land on a block, so a rocket at take-off is fine too.
            if dist >= 60.0 && speed < slow && tick.saturating_sub(self.last_boost) > 25 && (above_ground > 8.0 || game.pitch < -15.0) {
                if let Some(slot) = inv::firework_slot(game) {
                    inv::select(game, slot);
                    game.bar_use();
                    self.last_boost = tick;
                }
            }
            return PathingCommand::Defer;
        }
        if on_ground {
            if self.flew {
                // Down. Close enough: walk the rest. Otherwise it may go again if there are rockets.
                self.flew = false;
                self.attempts += 1;
                core.input.release(game);
                if dist < 4.0 {
                    core.say_green(game, "Landed at the goal.");
                    self.goal = None;
                } else if dist < 120.0 {
                    core.say(game, "Landed. Walking the rest of the way.");
                    self.phase = Phase::Walk;
                } else if inv::firework_slot(game).is_some() && self.attempts < 4 {
                    // Take off again.
                } else {
                    core.say_red(game, &format!("Landed {} blocks short of the goal.", dist as i64));
                    self.goal = None;
                }
                return PathingCommand::Defer;
            }
            if tick.saturating_sub(self.started) > 20 * 20 {
                core.say_red(game, "Stopped: I could not take off.");
                self.goal = None;
                return PathingCommand::Defer;
            }
            // Take off: jump, already looking up for the climb.
            look::turn_to(game, heading, 45.0);
            look::pitch_to(game, -45.0, 25.0);
            core.input.only(game, false, false, true);
            return PathingCommand::Defer;
        }
        // In the air and not gliding yet: a fresh press of jump while falling opens the wings.
        if tick.saturating_sub(self.started) > 20 * 30 {
            core.say_red(game, "Stopped: the elytra would not open.");
            self.goal = None;
            return PathingCommand::Defer;
        }
        let press = vel[1] < 0.0 && tick % 2 == 0;
        look::turn_to(game, heading, 45.0);
        look::pitch_to(game, -45.0, 25.0);
        core.input.only(game, false, false, press);
        PathingCommand::Defer
    }

    fn on_lost_control(&mut self, _core: &mut Core, _game: &mut Game<'_>) {}

    fn cancel(&mut self) {
        self.goal = None;
    }
}
