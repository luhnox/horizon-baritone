// Tests of the pathfinder ported from Baritone's Java (BlockState, CalculationContext, MovementHelper,
// the eight movements, Moves, AStarPathFinder, Path), on a world held in a map.
use crate::baritone_astar::{AStarPathFinder, Status};
use crate::baritone_block_state::{parse_props, BlockState, Class, FluidKind};
use crate::baritone_block_state_interface::BlockStateInterface;
use crate::baritone_calculation_context::{BareHands, CalculationContext, PlayerFacts};
use crate::baritone_costs::*;
use crate::baritone_goals::{rc, GoalBlock};
use crate::baritone_moves::Moves;
use crate::baritone_settings::Settings;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

fn state(name: &str, classes: &[&str], full: bool, hardness: f32) -> Rc<BlockState> {
    Rc::new(BlockState {
        id: 0,
        name: Arc::from(format!("minecraft:{name}")),
        classes: Class::flags_of(classes.iter().copied()),
        props: parse_props(name),
        fluid: FluidKind::None,
        fluid_amount: 0,
        hardness,
        replaceable: name == "air",
        full_collision: full,
        sturdy_top: full,
        pathfindable: !full,
    })
}

struct World {
    blocks: HashMap<(i32, i32, i32), Rc<BlockState>>,
    air: Rc<BlockState>,
    stone: Rc<BlockState>,
}

impl World {
    fn new() -> Self {
        Self { blocks: HashMap::new(), air: state("air", &["AirBlock"], false, 0.0), stone: state("stone", &[], true, 1.5) }
    }

    fn set(&mut self, x: i32, y: i32, z: i32, s: &Rc<BlockState>) {
        self.blocks.insert((x, y, z), s.clone());
    }

    /// A stone floor at y = 63 (feet at 64) from x0 to x1 and z from -3 to 3.
    fn floor(&mut self, x0: i32, x1: i32) {
        let stone = self.stone.clone();
        for x in x0..=x1 {
            for z in -3..=3 {
                self.set(x, 63, z, &stone);
            }
        }
    }
}

impl BlockStateInterface for World {
    fn get0(&self, x: i32, y: i32, z: i32) -> Rc<BlockState> {
        self.blocks.get(&(x, y, z)).cloned().unwrap_or_else(|| self.air.clone())
    }
    fn is_loaded(&self, _x: i32, _z: i32) -> bool {
        true
    }
    fn min_y(&self) -> i32 {
        0
    }
    fn max_y(&self) -> i32 {
        256
    }
}

fn settings() -> Settings {
    Settings::load(|_| None)
}

fn ctx<'a>(s: &Settings, w: &'a World, tools: &'a BareHands, facts: &PlayerFacts) -> CalculationContext<'a> {
    CalculationContext::new(s, w, tools, facts)
}

#[test]
fn a_walk_on_a_flat_floor_is_a_sprinted_block() {
    let mut w = World::new();
    w.floor(0, 10);
    let s = settings();
    let c = ctx(&s, &w, &BareHands, &PlayerFacts::default());
    let cost = Moves::TraverseEast.cost(&c, 0, 64, 0);
    assert!((cost - WALK_ONE_BLOCK_COST * SPRINT_MULTIPLIER).abs() < 1e-9, "{cost}");
}

#[test]
fn without_food_to_sprint_a_walk_is_a_walked_block() {
    let mut w = World::new();
    w.floor(0, 10);
    let s = settings();
    let facts = PlayerFacts { can_sprint: false, ..PlayerFacts::default() };
    let c = ctx(&s, &w, &BareHands, &facts);
    assert!((Moves::TraverseEast.cost(&c, 0, 64, 0) - WALK_ONE_BLOCK_COST).abs() < 1e-9);
}

#[test]
fn an_ascend_costs_a_jump_and_its_penalty() {
    let mut w = World::new();
    w.floor(0, 10);
    let stone = w.stone.clone();
    w.set(1, 64, 0, &stone);
    let s = settings();
    let c = ctx(&s, &w, &BareHands, &PlayerFacts::default());
    let cost = Moves::AscendEast.cost(&c, 0, 64, 0);
    let expected = jump_one_block_cost().max(WALK_ONE_BLOCK_COST) + s.float("jumpPenalty");
    assert!((cost - expected).abs() < 1e-9, "{cost} {expected}");
}

#[test]
fn a_one_block_drop_is_a_descend() {
    let mut w = World::new();
    w.floor(0, 10);
    let stone = w.stone.clone();
    // A ledge at x = 0 (stone at y 64, so the feet are at 65); the floor at x = 1 is one lower.
    w.set(0, 64, 0, &stone);
    let s = settings();
    let c = ctx(&s, &w, &BareHands, &PlayerFacts::default());
    let mut res = crate::baritone_calculation_context::MutableMoveResult::default();
    Moves::DescendEast.apply(&c, 0, 65, 0, &mut res);
    assert_eq!((res.x, res.y, res.z), (1, 64, 0));
    let expected = WALK_OFF_BLOCK_COST + fall_cost(1).max(CENTER_AFTER_FALL_COST);
    assert!((res.cost - expected).abs() < 1e-9, "{} {}", res.cost, expected);
}

#[test]
fn a_bridge_costs_a_placed_block() {
    let mut w = World::new();
    w.floor(0, 0);
    let stone = w.stone.clone();
    w.set(2, 63, 0, &stone);
    let s = settings();
    let c = ctx(&s, &w, &BareHands, &PlayerFacts::default());
    // x = 1 has no floor: a block goes there, against the floor of x = 2 (a side place, no sneaking).
    let cost = Moves::TraverseEast.cost(&c, 0, 64, 0);
    let expected = WALK_ONE_BLOCK_COST + s.float("blockPlacementPenalty");
    assert!((cost - expected).abs() < 1e-9, "{cost} {expected}");
}

#[test]
fn a_bridge_with_nothing_beside_it_is_a_backplace_and_sneaks() {
    let mut w = World::new();
    w.floor(0, 0);
    let s = settings();
    let c = ctx(&s, &w, &BareHands, &PlayerFacts::default());
    // Nothing at x = 2: the only block to place against is the one stood on.
    let cost = Moves::TraverseEast.cost(&c, 0, 64, 0);
    let expected = SNEAK_ONE_BLOCK_COST + s.float("blockPlacementPenalty");
    assert!((cost - expected).abs() < 1e-9, "{cost} {expected}");
}

#[test]
fn nothing_to_place_means_no_bridge() {
    let mut w = World::new();
    w.floor(0, 0);
    let s = settings();
    let facts = PlayerFacts { has_throwaway: false, ..PlayerFacts::default() };
    let c = ctx(&s, &w, &BareHands, &facts);
    assert!(Moves::TraverseEast.cost(&c, 0, 64, 0) >= COST_INF);
}

#[test]
fn a_star_walks_a_straight_line_across_a_floor() {
    let mut w = World::new();
    w.floor(-2, 20);
    let s = settings();
    let c = ctx(&s, &w, &BareHands, &PlayerFacts::default());
    let mut finder = AStarPathFinder::new([0, 64, 0], [0, 64, 0], rc(GoalBlock([10, 64, 0])), HashMap::new(), 1000, true);
    let path = loop {
        match finder.run(&c, 500) {
            Status::Running => {}
            Status::Done(p) => break p.post_process(&c),
            Status::Exhausted => panic!("no path"),
        }
    };
    assert_eq!(path.dest(), [10, 64, 0]);
    assert_eq!(path.length(), 11);
    let total: f64 = path.movements.iter().map(|m| m.known_cost()).sum();
    assert!((total - 10.0 * WALK_ONE_BLOCK_COST * SPRINT_MULTIPLIER).abs() < 1e-6, "{total}");
}

#[test]
fn a_star_cuts_a_corner_with_a_diagonal() {
    let mut w = World::new();
    w.floor(-2, 20);
    let s = settings();
    let c = ctx(&s, &w, &BareHands, &PlayerFacts::default());
    let mut finder = AStarPathFinder::new([0, 64, 0], [0, 64, 0], rc(GoalBlock([3, 64, 3])), HashMap::new(), 1000, true);
    let path = loop {
        match finder.run(&c, 500) {
            Status::Running => {}
            Status::Done(p) => break p.post_process(&c),
            Status::Exhausted => panic!("no path"),
        }
    };
    assert_eq!(path.length(), 4); // three diagonals
}

#[test]
fn a_star_bridges_a_gap_when_it_has_blocks() {
    let mut w = World::new();
    w.floor(-2, 2);
    w.floor(6, 12);
    let s = settings();
    let c = ctx(&s, &w, &BareHands, &PlayerFacts::default());
    let mut finder = AStarPathFinder::new([0, 64, 0], [0, 64, 0], rc(GoalBlock([8, 64, 0])), HashMap::new(), 1000, true);
    let mut result = None;
    for _ in 0..200 {
        match finder.run(&c, 500) {
            Status::Running => {}
            Status::Done(p) => {
                result = Some(p.post_process(&c));
                break;
            }
            Status::Exhausted => break,
        }
    }
    let path = result.expect("a path over the gap");
    assert_eq!(path.dest(), [8, 64, 0]);
}

#[test]
fn a_star_finds_nothing_across_a_gap_with_no_blocks_and_no_parkour() {
    let mut w = World::new();
    w.floor(-2, 2);
    w.floor(30, 40);
    let s = settings();
    let facts = PlayerFacts { has_throwaway: false, ..PlayerFacts::default() };
    let c = ctx(&s, &w, &BareHands, &facts);
    let mut finder = AStarPathFinder::new([0, 64, 0], [0, 64, 0], rc(GoalBlock([35, 64, 0])), HashMap::new(), 1000, true);
    loop {
        match finder.run(&c, 5000) {
            Status::Running => {}
            Status::Done(_) => panic!("crossed a gap it cannot"),
            Status::Exhausted => break,
        }
    }
}
