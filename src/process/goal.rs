// baritone.process.CustomGoalProcess: go to a goal the user (or a command) set.
use super::baritone::Core;
use super::baritone_goals::GoalRef;
use super::baritone_pathing::{PathingCommand, Process};
use super::Game;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    None,
    /// A goal is set, nothing asked to go.
    GoalSet,
    PathRequested,
    Executing,
}

pub struct CustomGoalProcess {
    goal: Option<GoalRef>,
    /// The last goal set, even after it was reached or cleared (what #elytra flies to).
    recent: Option<GoalRef>,
    state: State,
}

impl CustomGoalProcess {
    pub fn new() -> Self {
        Self { goal: None, recent: None, state: State::None }
    }

    pub fn goal(&self) -> Option<&GoalRef> {
        self.goal.as_ref()
    }

    pub fn most_recent_goal(&self) -> Option<&GoalRef> {
        self.recent.as_ref()
    }

    /// Set the goal without going to it.
    pub fn set_goal(&mut self, goal: GoalRef) {
        self.recent = Some(goal.clone());
        self.goal = Some(goal);
        if self.state == State::None {
            self.state = State::GoalSet;
        } else if self.state == State::Executing {
            self.state = State::PathRequested;
        }
    }

    pub fn set_goal_and_path(&mut self, goal: GoalRef) {
        self.recent = Some(goal.clone());
        self.goal = Some(goal);
        self.state = State::PathRequested;
    }

    /// Go to the goal that is set. False when there is none.
    pub fn path(&mut self) -> bool {
        if self.goal.is_none() {
            return false;
        }
        self.state = State::PathRequested;
        true
    }

    /// Forget the goal. False when there was none.
    pub fn clear_goal(&mut self) -> bool {
        let had = self.goal.is_some();
        self.goal = None;
        self.state = State::None;
        had
    }
}

impl Process for CustomGoalProcess {
    fn name(&self) -> &'static str {
        "custom goal"
    }

    fn display(&self) -> String {
        match &self.goal {
            Some(g) => format!("Going to {}", g.label()),
            None => "No goal".into(),
        }
    }

    fn is_active(&self) -> bool {
        matches!(self.state, State::PathRequested | State::Executing)
    }

    fn priority(&self) -> f64 {
        3.0
    }

    fn on_tick(&mut self, core: &mut Core, game: &mut Game<'_>, calc_failed: bool, _safe_to_cancel: bool) -> PathingCommand {
        let Some(goal) = self.goal.clone() else {
            self.state = State::None;
            return PathingCommand::Defer;
        };
        match self.state {
            State::None | State::GoalSet => PathingCommand::Defer,
            State::PathRequested => {
                self.state = State::Executing;
                PathingCommand::ForceRevalidateGoalAndPath(goal)
            }
            State::Executing => {
                if calc_failed {
                    core.say_red(game, "There is no way to go from here.");
                    self.state = State::GoalSet;
                    return PathingCommand::CancelAndSetGoal(goal);
                }
                let arrived = core.player_feet(game).is_some_and(|f| goal.is_in_goal(f));
                if arrived {
                    core.say_green(game, &format!("Arrived at {}.", goal.label()));
                    self.goal = None;
                    self.state = State::None;
                    return PathingCommand::CancelAndSetGoal(goal);
                }
                PathingCommand::SetGoalAndPath(goal)
            }
        }
    }

    fn on_lost_control(&mut self, _core: &mut Core, _game: &mut Game<'_>) {
        self.state = if self.goal.is_some() { State::GoalSet } else { State::None };
    }

    fn cancel(&mut self) {
        self.state = if self.goal.is_some() { State::GoalSet } else { State::None };
    }
}
