// baritone.api.pathing.goals: where a path is going. A goal says when the search is done and how far
// from done a position is. The heuristic must not be more than the real cost left, or the first
// path found is not the cheapest one.
use super::baritone_costs::{fall_cost, jump_one_block_cost, SPRINT_ONE_BLOCK_COST};
use super::baritone_utils::Pos;
use std::rc::Rc;

pub trait Goal {
    fn is_in_goal(&self, p: Pos) -> bool;
    /// An estimate of the ticks from `p` to the goal.
    fn heuristic(&self, p: Pos) -> f64;
    /// A short description for chat.
    fn label(&self) -> String;
    /// The block drawn for this goal, if it has one.
    fn render_pos(&self) -> Option<Pos> {
        None
    }
    /// The column this goal is in, if it has one (#elytra flies to it).
    fn column(&self) -> Option<(i32, i32)> {
        self.render_pos().map(|p| (p[0], p[2]))
    }
    /// The goal this one turns around, if it is an inversion.
    fn inverted_origin(&self) -> Option<GoalRef> {
        None
    }
    /// What to draw for it.
    fn shapes(&self) -> Vec<Shape> {
        if let Some(p) = self.render_pos() {
            vec![Shape::Block(p)]
        } else if let Some(c) = self.column() {
            vec![Shape::Column(c.0, c.1)]
        } else {
            Vec::new()
        }
    }
}

/// A goal as it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    Block(Pos),
    /// A tall column over a place with no height.
    Column(i32, i32),
    /// A flat square at a height.
    Level(i32),
    /// Somewhere to keep away from.
    Avoid(Pos),
}

pub type GoalRef = Rc<dyn Goal>;

/// Baritone's costHeuristic: the cost of a sprinted block, a little over the cheapest real one.
const COST_HEURISTIC: f64 = 3.563;

/// The cost of covering (x, z) blocks on the flat: diagonals first, then straight.
pub fn xz_heuristic(dx: f64, dz: f64) -> f64 {
    let (x, z) = (dx.abs(), dz.abs());
    let straight = (x - z).abs();
    let diagonal = x.min(z) * std::f64::consts::SQRT_2;
    (diagonal + straight) * COST_HEURISTIC
}

/// The cost of changing height from `current` to `goal`.
pub fn y_heuristic(goal: i32, current: i32) -> f64 {
    if current > goal {
        // Falling is cheap: half of what two blocks of it cost for each block.
        fall_cost(2) / 2.0 * f64::from(current - goal)
    } else if current < goal {
        f64::from(goal - current) * jump_one_block_cost().max(SPRINT_ONE_BLOCK_COST)
    } else {
        0.0
    }
}

fn block_heuristic(dx: i32, dy: i32, dz: i32, goal_y: i32, y: i32) -> f64 {
    let _ = dy;
    xz_heuristic(f64::from(dx), f64::from(dz)) + y_heuristic(goal_y, y)
}

/// One block.
pub struct GoalBlock(pub Pos);

impl Goal for GoalBlock {
    fn is_in_goal(&self, p: Pos) -> bool {
        p == self.0
    }
    fn heuristic(&self, p: Pos) -> f64 {
        block_heuristic(p[0] - self.0[0], p[1] - self.0[1], p[2] - self.0[2], self.0[1], p[1])
    }
    fn label(&self) -> String {
        format!("GoalBlock{{x={},y={},z={}}}", self.0[0], self.0[1], self.0[2])
    }
    fn render_pos(&self) -> Option<Pos> {
        Some(self.0)
    }
}

/// A column, any height.
pub struct GoalXZ(pub i32, pub i32);

impl Goal for GoalXZ {
    fn is_in_goal(&self, p: Pos) -> bool {
        p[0] == self.0 && p[2] == self.1
    }
    fn heuristic(&self, p: Pos) -> f64 {
        xz_heuristic(f64::from(p[0] - self.0), f64::from(p[2] - self.1))
    }
    fn label(&self) -> String {
        format!("GoalXZ{{x={},z={}}}", self.0, self.1)
    }
    fn column(&self) -> Option<(i32, i32)> {
        Some((self.0, self.1))
    }
}

/// A height, anywhere.
pub struct GoalYLevel(pub i32);

impl Goal for GoalYLevel {
    fn is_in_goal(&self, p: Pos) -> bool {
        p[1] == self.0
    }
    fn heuristic(&self, p: Pos) -> f64 {
        y_heuristic(self.0, p[1])
    }
    fn label(&self) -> String {
        format!("GoalYLevel{{level={}}}", self.0)
    }
    fn shapes(&self) -> Vec<Shape> {
        vec![Shape::Level(self.0)]
    }
}

/// Within a sphere of `range` blocks around a position.
pub struct GoalNear(pub Pos, pub i32);

impl Goal for GoalNear {
    fn is_in_goal(&self, p: Pos) -> bool {
        let d = [p[0] - self.0[0], p[1] - self.0[1], p[2] - self.0[2]];
        d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= self.1 * self.1
    }
    fn heuristic(&self, p: Pos) -> f64 {
        block_heuristic(p[0] - self.0[0], p[1] - self.0[1], p[2] - self.0[2], self.0[1], p[1])
    }
    fn label(&self) -> String {
        format!("GoalNear{{x={},y={},z={},rangeSq={}}}", self.0[0], self.0[1], self.0[2], self.1 * self.1)
    }
    fn render_pos(&self) -> Option<Pos> {
        Some(self.0)
    }
}

/// Next to a block, or in its column: where you stand to use or break it.
pub struct GoalGetToBlock(pub Pos);

impl Goal for GoalGetToBlock {
    fn is_in_goal(&self, p: Pos) -> bool {
        let (dx, dy, dz) = (p[0] - self.0[0], p[1] - self.0[1], p[2] - self.0[2]);
        dx.abs() + (if dy < 0 { dy + 1 } else { dy }).abs() + dz.abs() <= 1
    }
    fn heuristic(&self, p: Pos) -> f64 {
        let dy = p[1] - self.0[1];
        let shifted = if dy < 0 { dy + 1 } else { dy };
        block_heuristic(p[0] - self.0[0], shifted, p[2] - self.0[2], self.0[1], p[1])
    }
    fn label(&self) -> String {
        format!("GoalGetToBlock{{x={},y={},z={}}}", self.0[0], self.0[1], self.0[2])
    }
    fn render_pos(&self) -> Option<Pos> {
        Some(self.0)
    }
}

/// A two high column: stand at the block or the one under it, to break something above your head.
pub struct GoalTwoBlocks(pub Pos);

impl Goal for GoalTwoBlocks {
    fn is_in_goal(&self, p: Pos) -> bool {
        p[0] == self.0[0] && p[2] == self.0[2] && (p[1] == self.0[1] || p[1] == self.0[1] - 1)
    }
    fn heuristic(&self, p: Pos) -> f64 {
        let dy = p[1] - self.0[1];
        let shifted = if p[1] < self.0[1] { dy + 1 } else { dy };
        block_heuristic(p[0] - self.0[0], shifted, p[2] - self.0[2], self.0[1], p[1])
    }
    fn label(&self) -> String {
        format!("GoalTwoBlocks{{x={},y={},z={}}}", self.0[0], self.0[1], self.0[2])
    }
    fn render_pos(&self) -> Option<Pos> {
        Some(self.0)
    }
}

/// MineProcess.GoalThreeBlocks: the block, the one under it, or the one two under it.
pub struct GoalThreeBlocks(pub Pos);

impl Goal for GoalThreeBlocks {
    fn is_in_goal(&self, p: Pos) -> bool {
        p[0] == self.0[0] && p[2] == self.0[2] && (self.0[1] - 2..=self.0[1]).contains(&p[1])
    }
    fn heuristic(&self, p: Pos) -> f64 {
        GoalTwoBlocks(self.0).heuristic(p)
    }
    fn label(&self) -> String {
        format!("GoalThreeBlocks{{x={},y={},z={}}}", self.0[0], self.0[1], self.0[2])
    }
    fn render_pos(&self) -> Option<Pos> {
        Some(self.0)
    }
}

/// Any of several goals: done at the first, as near as the nearest.
pub struct GoalComposite(pub Vec<GoalRef>);

impl Goal for GoalComposite {
    fn is_in_goal(&self, p: Pos) -> bool {
        self.0.iter().any(|g| g.is_in_goal(p))
    }
    fn heuristic(&self, p: Pos) -> f64 {
        self.0.iter().map(|g| g.heuristic(p)).fold(f64::INFINITY, f64::min)
    }
    fn label(&self) -> String {
        format!("GoalComposite{{[{}]}}", self.0.iter().map(|g| g.label()).collect::<Vec<_>>().join(", "))
    }
    fn render_pos(&self) -> Option<Pos> {
        self.0.first().and_then(|g| g.render_pos())
    }
    fn column(&self) -> Option<(i32, i32)> {
        self.0.first().and_then(|g| g.column())
    }
    fn shapes(&self) -> Vec<Shape> {
        self.0.iter().take(40).flat_map(|g| g.shapes()).collect()
    }
}

/// Anywhere but the goal: the heuristic climbs as the position nears it.
pub struct GoalInverted(pub GoalRef);

impl Goal for GoalInverted {
    fn is_in_goal(&self, _p: Pos) -> bool {
        false
    }
    fn heuristic(&self, p: Pos) -> f64 {
        -self.0.heuristic(p)
    }
    fn label(&self) -> String {
        format!("GoalInverted{{{}}}", self.0.label())
    }
    fn inverted_origin(&self) -> Option<GoalRef> {
        Some(self.0.clone())
    }
    fn shapes(&self) -> Vec<Shape> {
        self.0
            .shapes()
            .into_iter()
            .map(|s| match s {
                Shape::Block(p) => Shape::Avoid(p),
                other => other,
            })
            .collect()
    }
}

/// Away from some positions: done once at least `distance` from every one of them.
pub struct GoalRunAway {
    pub from: Vec<Pos>,
    pub distance: i32,
    pub maintain_y: Option<i32>,
}

impl Goal for GoalRunAway {
    fn is_in_goal(&self, p: Pos) -> bool {
        if let Some(y) = self.maintain_y {
            if p[1] != y {
                return false;
            }
        }
        let want = i64::from(self.distance) * i64::from(self.distance);
        self.from.iter().all(|f| {
            let (dx, dz) = (i64::from(p[0] - f[0]), i64::from(p[2] - f[2]));
            dx * dx + dz * dz >= want
        })
    }
    fn heuristic(&self, p: Pos) -> f64 {
        let mut best = f64::INFINITY;
        for f in &self.from {
            best = best.min(-xz_heuristic(f64::from(p[0] - f[0]), f64::from(p[2] - f[2])));
        }
        if !best.is_finite() {
            best = 0.0;
        }
        if let Some(y) = self.maintain_y {
            best += y_heuristic(y, p[1]);
        }
        best
    }
    fn label(&self) -> String {
        format!("GoalRunAwayFrom{{from=[{}],distance={}}}", self.from.iter().map(|f| format!("BetterBlockPos{{x={},y={},z={}}}", f[0], f[1], f[2])).collect::<Vec<_>>().join(", "), self.distance)
    }
}

/// Keep going in one direction (dx, dz) from an origin, in a straight line.
pub struct GoalStrictDirection {
    pub origin: Pos,
    pub dx: i32,
    pub dz: i32,
}

impl Goal for GoalStrictDirection {
    fn is_in_goal(&self, _p: Pos) -> bool {
        false
    }
    fn heuristic(&self, p: Pos) -> f64 {
        let (dx, dz) = (p[0] - self.origin[0], p[2] - self.origin[2]);
        let along = dx * self.dx + dz * self.dz;
        let across = (dx * self.dz).abs() + (dz * self.dx).abs();
        let vertical = (p[1] - self.origin[1]).abs();
        -f64::from(along) * 100.0 + f64::from(across) * 1000.0 + f64::from(vertical) * 1000.0
    }
    fn label(&self) -> String {
        format!("GoalStrictDirection{{x={},y={},z={},dx={},dz={}}}", self.origin[0], self.origin[1], self.origin[2], self.dx, self.dz)
    }
}

/// The highways of the Nether: a height, and the axes and diagonals through the origin.
pub struct GoalAxis(pub i32);

impl Goal for GoalAxis {
    fn is_in_goal(&self, p: Pos) -> bool {
        p[1] == self.0 && (p[0] == 0 || p[2] == 0 || p[0].abs() == p[2].abs())
    }
    fn heuristic(&self, p: Pos) -> f64 {
        let (x, z) = (p[0].abs(), p[2].abs());
        let to_axis = x.min(z);
        let to_diagonal = (x - z).abs();
        f64::from(to_axis.min(to_diagonal)) * COST_HEURISTIC + y_heuristic(self.0, p[1])
    }
    fn label(&self) -> String {
        "GoalAxis".to_owned()
    }
}

/// A goal in a shared pointer: what the searches and the processes hold.
pub fn rc<G: Goal + 'static>(goal: G) -> GoalRef {
    Rc::new(goal)
}
