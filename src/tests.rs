use crate::baritone_legacy_calc::*;
use crate::baritone_costs::*;
use crate::baritone_goals::*;
use crate::baritone_legacy_helper::*;
use crate::baritone_legacy_movement::*;
use crate::baritone_legacy_path::*;
use crate::baritone_utils::*;
use std::collections::HashMap;

/// A box of a world: solid below `floor`, with extra blocks set by hand.
struct World {
    floor: i32,
    set: HashMap<Pos, Cell>,
    reads: usize,
    diggable: bool,
}
impl World {
    fn flat(floor: i32) -> Self {
        Self { floor, set: HashMap::new(), reads: 0, diggable: false }
    }
    fn put(&mut self, p: Pos, c: Cell) -> &mut Self {
        self.set.insert(p, c);
        self
    }
    fn fill(&mut self, a: Pos, b: Pos, c: Cell) -> &mut Self {
        for x in a[0].min(b[0])..=a[0].max(b[0]) {
            for y in a[1].min(b[1])..=a[1].max(b[1]) {
                for z in a[2].min(b[2])..=a[2].max(b[2]) {
                    self.set.insert([x, y, z], c);
                }
            }
        }
        self
    }
}
impl Terrain for World {
    fn cell(&mut self, p: Pos) -> Cell {
        self.reads += 1;
        if let Some(&c) = self.set.get(&p) {
            return c;
        }
        if p[1] < self.floor { Cell::Solid } else { Cell::Air }
    }
    fn dig_cost(&mut self, p: Pos) -> Option<f64> {
        (self.diggable && self.cell(p) == Cell::Solid && p[1] >= self.floor).then_some(12.0)
    }
}

fn block(p: Pos) -> GoalRef {
    rc(GoalBlock(p))
}

fn plan(w: &mut World, from: Pos, goal: GoalRef) -> Plan {
    find_path(w, from, goal, &CalcSettings::default()).expect("a path")
}

fn snap(pos: [f64; 3], tick: u64) -> Snapshot {
    Snapshot { pos, on_ground: true, in_water: false, climbing: false, blocked: false, tick }
}

fn build() -> CalcSettings {
    CalcSettings { allow_place: true, ..CalcSettings::default() }
}

fn limited(n: usize) -> CalcSettings {
    CalcSettings { max_nodes: n, ..CalcSettings::default() }
}

// ---- costs and goals

#[test]
fn costs_match_baritones() {
    assert!((WALK_ONE_BLOCK_COST - 4.6328).abs() < 1e-3);
    assert!((SPRINT_ONE_BLOCK_COST - 3.5638).abs() < 1e-3);
    assert!(fall_cost(1) < fall_cost(2) && fall_cost(2) < fall_cost(20));
    assert!(fall_cost(0) == 0.0);
    // A jump up one block is dearer than a step, cheaper than a long fall.
    assert!(jump_one_block_cost() > 2.0 && jump_one_block_cost() < 5.0, "{}", jump_one_block_cost());
}

#[test]
fn goal_shapes() {
    assert!(GoalNear([5, 64, 5], 2).is_in_goal([4, 64, 4]));
    assert!(!GoalNear([5, 64, 5], 1).is_in_goal([3, 64, 5]));
    assert!(GoalXZ(3, 4).is_in_goal([3, 70, 4]));
    assert!(GoalYLevel(70).is_in_goal([9, 70, 9]));
    let any = GoalComposite(vec![block([10, 64, 0]), block([0, 64, 10])]);
    assert!(any.is_in_goal([10, 64, 0]) && any.is_in_goal([0, 64, 10]) && !any.is_in_goal([5, 64, 5]));
    assert!(any.heuristic([9, 64, 0]) < any.heuristic([5, 64, 5]));
}

#[test]
fn get_to_block_is_next_to_it_or_in_its_column() {
    let g = GoalGetToBlock([5, 64, 5]);
    assert!(g.is_in_goal([4, 64, 5]) && g.is_in_goal([5, 63, 5]) && g.is_in_goal([5, 62, 5]) && g.is_in_goal([5, 65, 5]));
    assert!(!g.is_in_goal([4, 64, 4]) && !g.is_in_goal([3, 64, 5]));
    assert!(g.heuristic([0, 64, 0]) > g.heuristic([4, 64, 5]));
}

#[test]
fn two_blocks_is_the_block_and_the_one_under_it() {
    let g = GoalTwoBlocks([2, 64, 2]);
    assert!(g.is_in_goal([2, 64, 2]) && g.is_in_goal([2, 63, 2]) && !g.is_in_goal([2, 65, 2]) && !g.is_in_goal([3, 64, 2]));
}

#[test]
fn inverted_run_away_axis_and_strict_direction() {
    let near = block([0, 64, 0]);
    let away = GoalInverted(near);
    assert!(!away.is_in_goal([50, 64, 0]));
    assert!(away.heuristic([1, 64, 0]) > away.heuristic([30, 64, 0]), "nearer the thing is worse");
    let run = GoalRunAway { from: vec![[0, 64, 0]], distance: 10, maintain_y: None };
    assert!(!run.is_in_goal([5, 64, 0]) && run.is_in_goal([10, 64, 0]) && run.is_in_goal([-8, 64, 8]));
    let axis = GoalAxis(120);
    assert!(axis.is_in_goal([0, 120, 9]) && axis.is_in_goal([7, 120, 7]) && axis.is_in_goal([-7, 120, 7]) && !axis.is_in_goal([3, 120, 9]) && !axis.is_in_goal([0, 64, 9]));
    let east = GoalStrictDirection { origin: [0, 64, 0], dx: 1, dz: 0 };
    assert!(east.heuristic([10, 64, 0]) < east.heuristic([0, 64, 0]) && east.heuristic([10, 64, 3]) > east.heuristic([10, 64, 0]));
}

#[test]
fn directions_and_yaw() {
    assert_eq!(facing_step(0.0), (0, 1));
    assert_eq!(facing_step(90.0), (-1, 0));
    assert_eq!(facing_step(180.0), (0, -1));
    assert_eq!(facing_step(-90.0), (1, 0));
    assert_eq!(direction_named("North"), Some((0, -1)));
    assert!((yaw_to(1.0, 0.0) + 90.0).abs() < 1e-3);
    assert!((approach_yaw(170.0, -170.0, 40.0) - 190.0).abs() < 1e-3, "through 180, not back round");
    assert!((approach_yaw(0.0, 90.0, 30.0) - 30.0).abs() < 1e-3);
}

// ---- search

#[test]
fn walks_straight_across_flat_ground() {
    let mut w = World::flat(64);
    let p = plan(&mut w, [0, 64, 0], block([8, 64, 0]));
    assert!(p.complete);
    assert_eq!(p.path.first(), Some(&[0, 64, 0]));
    assert_eq!(p.path.last(), Some(&[8, 64, 0]));
    assert_eq!(p.path.len(), 9, "eight steps, no detour");
}

#[test]
fn cuts_corners_on_the_diagonal() {
    let mut w = World::flat(64);
    let p = plan(&mut w, [0, 64, 0], block([5, 64, 5]));
    assert!(p.complete);
    assert_eq!(p.path.len(), 6, "five diagonal steps");
}

#[test]
fn goes_round_a_wall() {
    let mut w = World::flat(64);
    w.fill([4, 64, -3], [4, 66, 3], Cell::Solid);
    let p = plan(&mut w, [0, 64, 0], block([8, 64, 0]));
    assert!(p.complete);
    assert!(p.path.iter().all(|n| !(n[0] == 4 && n[2].abs() <= 3)), "never through the wall");
}

#[test]
fn jumps_up_a_one_block_step_but_not_two() {
    let mut w = World::flat(64);
    w.put([3, 64, 0], Cell::Solid);
    let p = plan(&mut w, [0, 64, 0], block([3, 65, 0]));
    assert!(p.complete);
    assert_eq!(p.path.last(), Some(&[3, 65, 0]));
    let mut w = World::flat(64);
    w.fill([3, 64, -9], [3, 65, 9], Cell::Solid);
    let p = find_path(&mut w, [0, 64, 0], block([5, 66, 0]), &CalcSettings::default()).unwrap();
    assert!(!p.complete, "a two block wall that is nine wide has no way over");
}

#[test]
fn needs_headroom_to_jump() {
    let mut w = World::flat(64);
    w.put([2, 64, 0], Cell::Solid);
    // A ceiling over every block a jump up could start from.
    w.fill([0, 66, -3], [4, 66, 3], Cell::Solid);
    let p = find_path(&mut w, [0, 64, 0], block([2, 65, 0]), &CalcSettings::default()).unwrap();
    assert!(!p.complete, "no jump under a low ceiling: {:?}", p.path);
    let mut w = World::flat(64);
    w.put([2, 64, 0], Cell::Solid);
    let p = find_path(&mut w, [0, 64, 0], block([2, 65, 0]), &CalcSettings::default()).unwrap();
    assert!(p.complete);
}

#[test]
fn drops_up_to_three_blocks_and_no_further() {
    // A platform three blocks high: stand on it at 63, the ground is at 60.
    let mut w = World::flat(60);
    w.fill([-2, 60, -2], [2, 62, 2], Cell::Solid);
    let p = plan(&mut w, [0, 63, 0], block([6, 60, 0]));
    assert!(p.complete, "a three block drop is allowed: {:?}", p.path);
    // Five high: no way down but the drop, and five is too many.
    let mut w = World::flat(60);
    w.fill([-2, 60, -2], [2, 64, 2], Cell::Solid);
    let p = find_path(&mut w, [0, 65, 0], block([6, 60, 0]), &CalcSettings::default()).unwrap();
    assert!(!p.complete, "a five block drop is not");
}

#[test]
fn will_not_walk_into_lava_or_fire() {
    let mut w = World::flat(64);
    w.fill([2, 63, -400], [2, 63, 400], Cell::Lava);
    let p = find_path(&mut w, [0, 64, 0], block([4, 64, 0]), &limited(3000)).unwrap();
    assert!(!p.complete);
    assert!(p.path.iter().all(|n| n[0] < 2), "stops at the lava");
    let mut w = World::flat(64);
    w.fill([2, 64, -400], [2, 65, 400], Cell::Danger);
    let p = find_path(&mut w, [0, 64, 0], block([4, 64, 0]), &limited(3000)).unwrap();
    assert!(!p.complete, "a wall of fire is a wall");
}

#[test]
fn swims_through_water() {
    let mut w = World::flat(60);
    // A lake five wide, its surface at the ground's level.
    w.fill([2, 59, -300], [6, 60, 300], Cell::Water);
    let p = plan(&mut w, [0, 60, 0], block([8, 60, 0]));
    assert!(p.complete, "across the lake: {:?}", p.path);
    let dry = CalcSettings { allow_swim: false, max_nodes: 3000, ..CalcSettings::default() };
    let none = find_path(&mut w, [0, 60, 0], block([8, 60, 0]), &dry).map(|p| p.complete);
    assert_ne!(none, Some(true), "no swimming, no way across a lake that long");
}

#[test]
fn climbs_a_ladder() {
    let mut w = World::flat(64);
    // A tower: a wall at x=3 with a ladder on its west face, ten high.
    w.fill([3, 64, 0], [3, 74, 0], Cell::Solid);
    w.fill([2, 64, 0], [2, 74, 0], Cell::Climb);
    let p = plan(&mut w, [0, 64, 0], block([3, 75, 0]));
    assert!(p.complete, "up the ladder and onto the top: {:?}", p.path);
    assert!(p.path.contains(&[2, 70, 0]));
}

#[test]
fn falls_into_deep_water_safely() {
    let mut w = World::flat(40);
    w.fill([0, 40, 0], [0, 50, 0], Cell::Water);
    w.fill([0, 64, 0], [0, 64, 0], Cell::Air);
    w.put([1, 63, 0], Cell::Solid);
    let p = plan(&mut w, [1, 64, 0], block([0, 45, 0]));
    assert!(p.complete);
}

#[test]
fn heads_for_the_nearest_reachable_point_when_the_goal_is_walled_in() {
    let mut w = World::flat(64);
    w.fill([9, 64, -400], [9, 90, 400], Cell::Solid);
    let p = find_path(&mut w, [0, 64, 0], block([14, 64, 0]), &limited(4000)).unwrap();
    assert!(!p.complete);
    let end = *p.path.last().unwrap();
    assert!(end[0] >= 7 && end[0] < 9, "it walks up to the wall: {end:?}");
}

#[test]
fn a_hundred_block_walk_stays_inside_its_budget() {
    let mut w = World::flat(64);
    let p = find_path(&mut w, [0, 64, 0], block([100, 64, 60]), &CalcSettings::default()).unwrap();
    assert!(p.complete);
    assert!(p.nodes < 20_000, "{} nodes", p.nodes);
}

#[test]
fn the_search_can_be_spread_over_ticks() {
    let mut w = World::flat(64);
    let mut search = AStar::new(&mut w, block([60, 64, 40]), [0, 64, 0], CalcSettings::default()).unwrap();
    let mut runs = 0;
    let plan = loop {
        runs += 1;
        match search.run(&mut w, 200) {
            Status::Running => {
                assert!(runs < 500, "it never ends");
                // Asked in the middle, it can already say where it would go.
                if runs == 3 {
                    assert!(search.best_so_far().is_some());
                }
            }
            Status::Done(p) => break p,
            Status::Failed => panic!("failed"),
        }
    };
    assert!(plan.complete && runs > 3, "{runs} runs");
}

#[test]
fn tunnels_through_a_wall_when_allowed_and_not_otherwise() {
    let wall = |w: &mut World| {
        w.fill([3, 64, -400], [5, 70, 400], Cell::Solid);
    };
    let mut w = World::flat(64);
    wall(&mut w);
    let p = find_path(&mut w, [0, 64, 0], block([8, 64, 0]), &limited(3000)).unwrap();
    assert!(!p.complete, "no digging, no way through a wall 800 long");
    let mut w = World::flat(64);
    wall(&mut w);
    w.diggable = true;
    let open = CalcSettings { allow_break: true, ..CalcSettings::default() };
    let p = find_path(&mut w, [0, 64, 0], block([8, 64, 0]), &open).unwrap();
    assert!(p.complete, "straight through: {:?}", p.path);
    assert!(p.path.contains(&[4, 64, 0]));
}

#[test]
fn will_not_break_a_block_that_holds_back_lava_or_water() {
    let mut w = World::flat(64);
    w.fill([3, 64, -400], [3, 66, 400], Cell::Solid);
    w.put([4, 64, 0], Cell::Lava);
    w.diggable = true;
    let open = CalcSettings { allow_break: true, max_nodes: 3000, ..CalcSettings::default() };
    let p = find_path(&mut w, [0, 64, 0], block([8, 64, 0]), &open).unwrap();
    assert!(!p.path.contains(&[3, 64, 0]), "no hole in the wall with lava behind it");
}

#[test]
fn digs_up_a_slope_and_down_a_shaft() {
    let mut w = World::flat(64);
    w.fill([2, 64, -400], [10, 68, 400], Cell::Solid);
    w.diggable = true;
    let open = CalcSettings { allow_break: true, ..CalcSettings::default() };
    let p = find_path(&mut w, [0, 64, 0], block([6, 69, 0]), &open).unwrap();
    assert!(p.complete, "up through the hill, or over: {:?}", p.path);
    // A shaft straight down through the floor.
    let mut w = World::flat(40);
    w.fill([0, 40, 0], [0, 45, 0], Cell::Solid);
    w.fill([-2, 46, -2], [2, 63, 2], Cell::Solid);
    w.diggable = true;
    let p = find_path(&mut w, [0, 64, 0], block([0, 46, 0]), &open).unwrap();
    assert!(p.complete, "down a shaft: {:?}", p.path);
}

#[test]
fn walks_over_slabs_and_stairs_without_jumping() {
    let mut w = World::flat(64);
    w.fill([2, 64, 0], [5, 64, 0], Cell::Low);
    let p = plan(&mut w, [0, 64, 0], block([8, 64, 0]));
    assert!(p.complete);
    let mut w = World::flat(64);
    w.put([0, 64, 0], Cell::Low);
    let p = plan(&mut w, [0, 64, 0], block([6, 64, 0]));
    assert!(p.complete, "from a slab: {:?}", p.path);
    let mut w = World::flat(64);
    w.fill([0, 64, 0], [3, 64, 0], Cell::Low);
    w.fill([0, 66, 0], [3, 66, 0], Cell::Solid);
    assert!(stand(&mut w, [2, 64, 0]).is_none());
}

// ---- placing, parkour, doors, the bucket

#[test]
fn bridges_a_gap_that_is_too_wide_to_go_round() {
    let mut w = World::flat(64);
    // A trench a hundred blocks long, three wide, with the goal on the far side.
    w.fill([2, 40, -50], [4, 63, 50], Cell::Air);
    let plain = find_path(&mut w, [0, 64, 0], block([7, 64, 0]), &limited(2000));
    assert!(!plain.is_some_and(|p| p.complete), "no blocks, no way across");
    let p = find_path(&mut w, [0, 64, 0], block([7, 64, 0]), &build()).unwrap();
    assert!(p.complete, "bridged: {:?}", p.path);
    assert!(p.moves.contains(&Mv::Bridge));
    assert_eq!(p.moves.len(), p.path.len());
}

#[test]
fn pillars_up_a_wall_nothing_else_climbs() {
    let mut w = World::flat(64);
    w.fill([2, 64, -400], [2, 74, 400], Cell::Solid);
    let p = find_path(&mut w, [0, 64, 0], block([0, 70, 0]), &build()).unwrap();
    assert!(p.complete, "straight up: {:?}", p.path);
    assert_eq!(p.moves.iter().filter(|m| **m == Mv::Pillar).count(), 6);
    let p = find_path(&mut w, [0, 64, 0], block([0, 70, 0]), &limited(3000));
    assert!(!p.is_some_and(|p| p.complete), "without blocks it cannot");
}

#[test]
fn will_not_pillar_into_a_ceiling() {
    let mut w = World::flat(64);
    w.fill([-3, 66, -3], [3, 66, 3], Cell::Solid);
    let p = find_path(&mut w, [0, 64, 0], block([0, 68, 0]), &CalcSettings { max_nodes: 3000, ..build() }).unwrap();
    assert!(!p.path.iter().any(|n| n[0].abs() <= 3 && n[2].abs() <= 3 && n[1] == 66), "never inside the plate: {:?}", p.path);
}

#[test]
fn jumps_gaps_when_allowed() {
    let parkour = CalcSettings { allow_parkour: true, max_nodes: 4000, ..CalcSettings::default() };
    for gap in 1..=3 {
        let mut w = World::flat(64);
        w.fill([1, 40, -400], [gap, 63, 400], Cell::Air);
        let p = find_path(&mut w, [0, 64, 0], block([gap + 2, 64, 0]), &parkour).unwrap();
        assert!(p.complete, "a gap of {gap}: {:?}", p.path);
        assert!(p.moves.contains(&Mv::Parkour));
        let none = find_path(&mut w, [0, 64, 0], block([gap + 2, 64, 0]), &limited(4000));
        assert!(!none.is_some_and(|p| p.complete), "parkour is off by default");
    }
    // Four is too many.
    let mut w = World::flat(64);
    w.fill([1, 40, -400], [4, 63, 400], Cell::Air);
    let p = find_path(&mut w, [0, 64, 0], block([6, 64, 0]), &parkour);
    assert!(!p.is_some_and(|p| p.complete));
}

#[test]
fn a_long_jump_needs_a_run_up() {
    let parkour = CalcSettings { allow_parkour: true, max_nodes: 600, ..CalcSettings::default() };
    let mut w = World::flat(64);
    w.fill([-50, 40, -400], [-1, 63, 400], Cell::Air);
    w.fill([1, 40, -400], [3, 63, 400], Cell::Air);
    let p = find_path(&mut w, [0, 64, 0], block([5, 64, 0]), &parkour);
    assert!(!p.clone().is_some_and(|p| p.complete), "no run-up: {:?}", p.map(|p| (p.path, p.moves)));
}

#[test]
fn plans_through_a_door_and_the_follower_opens_it() {
    let mut w = World::flat(64);
    w.fill([2, 64, -400], [2, 66, 400], Cell::Solid);
    w.put([2, 64, 0], Cell::Door).put([2, 65, 0], Cell::Door);
    let p = plan(&mut w, [0, 64, 0], block([5, 64, 0]));
    assert!(p.complete);
    assert!(p.path.contains(&[2, 64, 0]), "through the door: {:?}", p.path);
    let mut f = PathExecutor::new(vec![[0, 64, 0], [1, 64, 0], [2, 64, 0]], 0);
    assert!(matches!(f.step(&snap([0.5, 64.0, 0.5], 1), &mut w), Step::Go(_)));
    f.index = 1;
    assert_eq!(f.step(&snap([1.5, 64.0, 0.5], 2), &mut w), Step::Interact([2, 64, 0]));
    w.put([2, 64, 0], Cell::Air).put([2, 65, 0], Cell::Air);
    assert!(matches!(f.step(&snap([1.5, 64.0, 0.5], 3), &mut w), Step::Go(_)));
}

#[test]
fn a_door_is_not_cut_across_on_the_diagonal() {
    let mut w = World::flat(64);
    w.put([1, 64, 0], Cell::Door).put([1, 65, 0], Cell::Door);
    let mut out = Vec::new();
    moves(&mut w, [0, 64, 0], Kind::Ground, &CalcSettings::default(), &mut out);
    assert!(!out.iter().any(|m| m.0 == [1, 64, 1]), "no corner cut past a closed door: {out:?}");
    assert!(out.iter().any(|m| m.0 == [1, 64, 0]), "head on is fine");
}

#[test]
fn drops_further_with_a_water_bucket() {
    let mut w = World::flat(50);
    w.fill([-2, 50, -2], [2, 63, 2], Cell::Solid);
    let plain = find_path(&mut w, [0, 64, 0], block([6, 50, 0]), &limited(3000));
    assert!(!plain.is_some_and(|p| p.complete), "a fourteen block drop is too far");
    let bucket = CalcSettings { max_fall_bucket: 20, ..CalcSettings::default() };
    let p = find_path(&mut w, [0, 64, 0], block([6, 50, 0]), &bucket).unwrap();
    assert!(p.complete, "with a bucket: {:?}", p.path);
}

// ---- the executor

#[test]
fn faces_the_next_node_and_walks() {
    let mut f = PathExecutor::new(vec![[0, 64, 0], [1, 64, 0], [2, 64, 0]], 0);
    let Step::Go(c) = f.step(&snap([0.5, 64.0, 0.5], 1), &mut World::flat(64)) else { panic!("go") };
    assert!(c.forward && !c.jump);
    assert!((c.yaw + 90.0).abs() < 0.01, "yaw {}", c.yaw);
    let Step::Go(c) = f.step(&snap([0.5, 64.0, 0.5], 2), &mut World::flat(64)) else { panic!("go") };
    assert!(c.sprint, "a straight run is sprinted");
}

#[test]
fn jumps_at_the_foot_of_a_step_and_arrives() {
    let mut f = PathExecutor::new(vec![[0, 64, 0], [1, 65, 0]], 0);
    let Step::Go(c) = f.step(&snap([0.5, 64.0, 0.5], 1), &mut World::flat(64)) else { panic!("go") };
    assert!(c.jump, "the node is a block up");
    assert!(!c.sprint, "no sprint into a step");
    assert_eq!(f.step(&snap([1.5, 65.0, 0.5], 9), &mut World::flat(64)), Step::Arrived);
}

#[test]
fn replans_when_knocked_off_or_stuck() {
    let mut f = PathExecutor::new(vec![[0, 64, 0], [1, 64, 0], [2, 64, 0], [3, 64, 0]], 0);
    assert_eq!(f.step(&snap([0.5, 64.0, 8.5], 1), &mut World::flat(64)), Step::Replan("off the path"));
    let mut f = PathExecutor::new(vec![[0, 64, 0], [1, 64, 0], [2, 64, 0]], 0);
    let mut last = Step::Arrived;
    for t in 1..=80 {
        last = f.step(&snap([0.5, 64.0, 0.5], t), &mut World::flat(64));
    }
    assert_eq!(last, Step::Replan("not getting anywhere"));
}

#[test]
fn does_not_replan_in_the_middle_of_a_fall() {
    let mut f = PathExecutor::new(vec![[0, 70, 0], [1, 66, 0], [2, 66, 0]], 0);
    let mut s = snap([1.2, 68.0, 0.5], 4);
    s.on_ground = false;
    assert!(matches!(f.step(&s, &mut World::flat(60)), Step::Go(_)));
}

#[test]
fn the_follower_asks_for_the_block_in_the_way_and_then_walks() {
    let mut w = World::flat(64);
    w.put([1, 64, 0], Cell::Solid);
    w.put([1, 65, 0], Cell::Solid);
    let mut f = PathExecutor::new(vec![[0, 64, 0], [1, 64, 0], [2, 64, 0]], 0);
    assert_eq!(f.step(&snap([0.5, 64.0, 0.5], 1), &mut w), Step::Dig([1, 64, 0]));
    w.put([1, 64, 0], Cell::Air);
    assert_eq!(f.step(&snap([0.5, 64.0, 0.5], 2), &mut w), Step::Dig([1, 65, 0]));
    w.put([1, 65, 0], Cell::Air);
    assert!(matches!(f.step(&snap([0.5, 64.0, 0.5], 3), &mut w), Step::Go(_)));
    let mut w = World::flat(64);
    w.put([1, 64, 0], Cell::Lava);
    let mut f = PathExecutor::new(vec![[0, 64, 0], [1, 64, 0]], 0);
    assert_eq!(f.step(&snap([0.5, 64.0, 0.5], 1), &mut w), Step::Replan("blocked"));
}

#[test]
fn the_follower_places_a_bridge_block_and_then_walks_onto_it() {
    let mut w = World::flat(64);
    w.fill([1, 40, -5], [1, 63, 5], Cell::Air);
    let mut f = PathExecutor::with_moves(vec![[0, 64, 0], [1, 64, 0], [2, 64, 0]], vec![Mv::Walk, Mv::Bridge, Mv::Walk], 0);
    assert_eq!(f.step(&snap([0.5, 64.0, 0.5], 1), &mut w), Step::Place { pos: [1, 63, 0], against: [0, 63, 0], face: [1, 0, 0] });
    w.put([1, 63, 0], Cell::Solid);
    assert!(matches!(f.step(&snap([0.5, 64.0, 0.5], 2), &mut w), Step::Go(_)));
}

#[test]
fn standing_over_the_edge_is_not_being_on_the_bridge() {
    // Sneaking out past the edge puts the feet in the cell the floor will go in: the block is still to
    // be placed, against the floor the player came from, not against the one that is not there.
    let mut w = World::flat(64);
    w.fill([1, 40, -5], [3, 63, 5], Cell::Air);
    let mut f = PathExecutor::with_moves(
        vec![[0, 64, 0], [1, 64, 0], [2, 64, 0], [3, 64, 0]],
        vec![Mv::Walk, Mv::Bridge, Mv::Bridge, Mv::Bridge],
        0,
    );
    let over = snap([1.29, 64.0, 0.5], 3);
    assert_eq!(f.step(&over, &mut w), Step::Place { pos: [1, 63, 0], against: [0, 63, 0], face: [1, 0, 0] });
    assert_eq!(f.index, 0, "still at the first node");
    w.put([1, 63, 0], Cell::Solid);
    // Now that the floor is there the node is reached, and the next block goes against it.
    let on = snap([1.5, 64.0, 0.5], 6);
    assert_eq!(f.step(&on, &mut w), Step::Place { pos: [2, 63, 0], against: [1, 63, 0], face: [1, 0, 0] });
    assert_eq!(f.index, 1);
}

#[test]
fn the_follower_pillars_until_the_block_is_there() {
    let mut w = World::flat(64);
    let mut f = PathExecutor::with_moves(vec![[0, 64, 0], [0, 65, 0], [1, 65, 0]], vec![Mv::Walk, Mv::Pillar, Mv::Walk], 0);
    assert_eq!(f.step(&snap([0.5, 64.0, 0.5], 1), &mut w), Step::Pillar { pos: [0, 64, 0], against: [0, 63, 0] });
    let mut up = snap([0.5, 65.1, 0.5], 2);
    up.on_ground = false;
    assert_eq!(f.step(&up, &mut w), Step::Pillar { pos: [0, 64, 0], against: [0, 63, 0] });
    assert_eq!(f.index, 0, "the node is not reached without the block");
    w.put([0, 64, 0], Cell::Solid);
    let mut on = snap([0.5, 65.0, 0.5], 4);
    on.on_ground = true;
    assert!(matches!(f.step(&on, &mut w), Step::Go(_)));
    assert_eq!(f.index, 1);
}

#[test]
fn the_follower_runs_up_and_jumps_at_the_far_edge() {
    let mut w = World::flat(64);
    w.fill([1, 40, -5], [3, 63, 5], Cell::Air);
    let mut f = PathExecutor::with_moves(vec![[0, 64, 0], [4, 64, 0]], vec![Mv::Walk, Mv::Parkour], 0);
    let Step::Go(c) = f.step(&snap([0.5, 64.0, 0.5], 1), &mut w) else { panic!("go") };
    assert!(c.forward && !c.sprint && !c.jump);
    assert!((c.yaw - 90.0).abs() < 1.0, "backwards is west: {}", c.yaw);
    let Step::Go(c) = f.step(&snap([-0.75, 64.0, 0.5], 5), &mut w) else { panic!("go") };
    assert!(c.sprint && !c.jump);
    let Step::Go(c) = f.step(&snap([0.85, 64.0, 0.5], 9), &mut w) else { panic!("go") };
    assert!(c.sprint && c.jump);
    let mut air = snap([1.6, 64.8, 0.5], 11);
    air.on_ground = false;
    let Step::Go(c) = f.step(&air, &mut w) else { panic!("go") };
    assert!(c.sprint && !c.jump);
}

// ---- settings, selection, schematics, arguments

mod extras {
    use crate::baritone_argument::*;
    use crate::baritone_schematic::*;
    use crate::baritone_selection::*;
    use crate::baritone_settings::*;
    use crate::baritone_utils::*;
    use crate::baritone_waypoints::{Tag, Waypoint, Waypoints};

    #[test]
    fn settings_have_defaults_parse_and_reset() {
        let mut s = Settings::load(|_| None);
        assert!(s.flag("allowBreak") && !s.flag("allowParkour") && s.int("maxFallHeightNoWater") == 3);
        assert_eq!(s.text("prefix"), "#");
        assert!(s.set("allowParkour", "on").is_ok() && s.flag("allowParkour"));
        assert!(s.set("allowParkour", "maybe").is_err());
        assert!(s.set("nope", "1").is_err());
        let (d, v) = s.set("colorGoalBox", "#00ff00").unwrap();
        assert_eq!((d.name, v.as_str()), ("colorGoalBox", "00FF00"));
        assert_eq!(s.color("colorGoalBox"), 0xFF00FF00);
        s.set("buildIgnoreBlocks", "minecraft:Dirt, stone").unwrap();
        assert_eq!(s.list("buildIgnoreBlocks"), vec!["dirt", "stone"]);
        let d = find("allowparkour").unwrap();
        assert!(s.modified(d));
        s.reset(d);
        assert!(!s.modified(d));
        // A stored value that no longer parses falls back to the default.
        let s = Settings::load(|n| (n == "maxFallHeightNoWater").then(|| "lots".to_owned()));
        assert_eq!(s.int("maxFallHeightNoWater"), 3);
    }

    #[test]
    fn every_setting_default_is_valid_and_names_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for d in DEFS {
            assert!(seen.insert(d.name), "{} twice", d.name);
            assert_eq!(parse(d.kind, d.default).as_deref(), Some(d.default), "{}", d.name);
        }
    }

    #[test]
    fn selections_expand_contract_and_shift_like_baritones() {
        let s = Selection::new([0, 64, 0], [4, 66, 2]);
        assert_eq!(s.size(), [5, 3, 3]);
        assert_eq!(s.volume(), 45);
        assert!(s.contains([2, 65, 1]) && !s.contains([5, 65, 1]));
        assert_eq!(s.expand([1, 0, 0], 2).max(), [6, 66, 2]);
        assert_eq!(s.expand([-1, 0, 0], 2).min(), [-2, 64, 0]);
        assert_eq!(s.expand([0, -1, 0], 1).min(), [0, 63, 0]);
        assert_eq!(s.contract([1, 0, 0], 1).min(), [1, 64, 0]);
        assert_eq!(s.contract([-1, 0, 0], 1).max(), [3, 66, 2]);
        assert_eq!(s.shift([0, 0, 1], 3).min(), [0, 64, 3]);
        let r = Selection::new([4, 66, 2], [0, 64, 0]);
        assert_eq!(r.expand([1, 0, 0], 2).max(), [6, 66, 2]);
        let mut m = SelectionManager::default();
        m.add([0, 64, 0], [4, 66, 2]);
        m.add([10, 64, 0], [11, 64, 0]);
        assert_eq!(m.last().unwrap().size(), [2, 1, 1]);
        assert_eq!(m.remove_last().unwrap().pos1, [10, 64, 0]);
        assert_eq!(m.remove_all().len(), 1);
        assert!(m.selections.is_empty());
        assert_eq!(TransformTarget::by_name("n").unwrap().indexes(3), vec![2]);
        assert_eq!(TransformTarget::by_name("oldest").unwrap().indexes(3), vec![0]);
        assert_eq!(TransformTarget::All.indexes(2), vec![0, 1]);
        assert!(TransformTarget::Newest.indexes(0).is_empty());
    }

    #[test]
    fn schematic_shapes() {
        let stone = BlockSpec::named("stone");
        let fill = FillSchematic { size: [3, 2, 3], spec: stone.clone() };
        assert_eq!(fill.desired([1, 1, 1], "minecraft:air"), Some(stone.clone()));
        let walls = ShapeSchematic { size: [5, 3, 5], shape: Shape::Walls, spec: stone.clone() };
        assert!(walls.desired([0, 1, 2], "").is_some() && walls.desired([2, 1, 2], "").is_none());
        let shell = ShapeSchematic { size: [5, 5, 5], shape: Shape::Shell, spec: stone.clone() };
        assert!(shell.desired([2, 0, 2], "").is_some() && shell.desired([2, 2, 2], "").is_none());
        let ball = ShapeSchematic { size: [9, 9, 9], shape: Shape::Sphere { hollow: false }, spec: stone.clone() };
        let shellball = ShapeSchematic { size: [9, 9, 9], shape: Shape::Sphere { hollow: true }, spec: stone.clone() };
        assert!(ball.desired([4, 4, 4], "").is_some() && shellball.desired([4, 4, 4], "").is_none());
        assert!(ball.desired([0, 0, 0], "").is_none(), "the corner is outside");
        assert!(shellball.desired([4, 0, 4], "").is_some() || shellball.desired([4, 1, 4], "").is_some());
        let tube = ShapeSchematic { size: [7, 4, 7], shape: Shape::Cylinder { hollow: true, axis: 1 }, spec: stone.clone() };
        assert!(tube.desired([3, 2, 3], "").is_none() && tube.desired([0, 2, 3], "").is_some());
        let swap = ReplaceSchematic { size: [2, 2, 2], from: vec!["dirt".into(), "grass_block".into()], to: BlockSpec::named("stone") };
        assert!(swap.desired([0, 0, 0], "minecraft:dirt").is_some() && swap.desired([0, 0, 0], "minecraft:sand").is_none());
    }

    #[test]
    fn block_specs_parse_and_match() {
        let b = BlockSpec::parse("oak_stairs[facing=north,half=bottom]");
        assert_eq!(b.name, "minecraft:oak_stairs");
        assert!(b.matches("minecraft:oak_stairs", "minecraft:oak_stairs[facing=north,half=bottom,shape=straight]"));
        assert!(!b.matches("minecraft:oak_stairs", "minecraft:oak_stairs[facing=south,half=bottom]"));
        assert!(BlockSpec::air().matches("minecraft:cave_air", ""));
    }

    #[test]
    fn rotating_a_static_schematic_turns_it_and_its_facing() {
        let stairs = BlockSpec::parse("oak_stairs[facing=north]");
        let s = StaticSchematic {
            size: [2, 1, 1],
            blocks: vec![Some(BlockSpec::named("stone")), Some(stairs)],
            name: "t".into(),
        };
        let r = rotate_quarter(&s);
        assert_eq!(r.size, [1, 1, 2]);
        assert_eq!(r.blocks[0].as_ref().unwrap().name, "minecraft:stone");
        assert_eq!(r.blocks[1].as_ref().unwrap().props, vec![("facing".to_owned(), "east".to_owned())]);
    }

    #[test]
    fn arguments() {
        let mut a = Args::parse("goto 10 ~ ~-5 foo");
        assert_eq!(a.string("command").unwrap(), "goto");
        assert!(a.looks_like_coordinate());
        assert_eq!(a.coord(0.0, "x").unwrap(), 10);
        assert_eq!(a.coord(64.7, "y").unwrap(), 64);
        assert_eq!(a.coord(100.2, "z").unwrap(), 95);
        assert!(a.int("n").is_err());
        assert_eq!(a.try_one_of(&["foo", "bar"]).as_deref(), Some("foo"));
        assert!(a.done().is_ok());
        assert_eq!(a.string("more").unwrap_err().0, "Missing more.");
        let mut b = Args::parse("3 stone dirt");
        assert_eq!(b.try_int(), Some(3));
        assert_eq!(b.try_int(), None);
        assert_eq!(b.rest(), vec!["stone", "dirt"]);
        assert_eq!(b.one_of(&["Stone"], "block").unwrap(), "Stone");
        assert!(b.done().is_err());
    }

    #[test]
    fn waypoints_save_find_and_round_trip() {

        let mut w = Waypoints::default();
        let at = |tag: Tag, name: &str, x, created| Waypoint { tag, name: name.into(), pos: [x, 64, 0], dimension: "overworld".into(), created };
        w.add(at(Tag::Home, "base", 10, 1));
        w.add(at(Tag::Home, "", 20, 2));
        w.add(at(Tag::User, "mine", 5, 3));
        assert_eq!(w.in_dimension("overworld").len(), 3);
        assert!(w.in_dimension("nether").is_empty());
        assert_eq!(w.lookup("home", "overworld").len(), 2, "a tag picks every waypoint with it");
        assert_eq!(w.lookup("MINE", "overworld").len(), 1, "anything else is a name");
        assert_eq!(Tag::by_name("H"), Some(Tag::Home));
        assert_eq!(Tag::by_name("nothing"), None);
        let again = Waypoints::from_text(&w.to_text());
        assert_eq!(again.all, w.all);
        let gone = w.all[0].clone();
        w.remove(&gone);
        assert_eq!(w.all.len(), 2);
    }

    #[test]
    fn composite_schematics_lay_their_parts_in_one_box() {
        use std::rc::Rc;
        let stone: Rc<dyn Schematic> = Rc::new(FillSchematic { size: [2, 2, 2], spec: BlockSpec::named("stone") });
        let dirt: Rc<dyn Schematic> = Rc::new(FillSchematic { size: [2, 2, 2], spec: BlockSpec::named("dirt") });
        let both = CompositeSchematic { size: [5, 2, 2], parts: vec![([0, 0, 0], stone), ([3, 0, 0], dirt)], name: "two".into() };
        assert_eq!(both.desired([1, 1, 1], "").unwrap().name, "minecraft:stone");
        assert_eq!(both.desired([4, 0, 0], "").unwrap().name, "minecraft:dirt");
        assert!(both.desired([2, 0, 0], "").is_none(), "the gap between them is left alone");
    }

    #[test]
    fn goal_shapes_for_drawing() {
        use crate::baritone_goals::*;
        let a = rc(GoalBlock([1, 64, 2]));
        assert_eq!(a.shapes(), vec![Shape::Block([1, 64, 2])]);
        assert_eq!(GoalXZ(3, 4).shapes(), vec![Shape::Column(3, 4)]);
        assert_eq!(GoalXZ(3, 4).column(), Some((3, 4)));
        assert_eq!(GoalYLevel(70).shapes(), vec![Shape::Level(70)]);
        let inverted = GoalInverted(a.clone());
        assert_eq!(inverted.shapes(), vec![Shape::Avoid([1, 64, 2])]);
        assert_eq!(GoalComposite(vec![a, rc(GoalXZ(0, 0))]).shapes().len(), 2);
    }

    #[test]
    fn pages_of_eight() {
        let p = Paginator::new(20);
        assert_eq!(p.max_page(), 3);
        assert!(p.valid_page(3) && !p.valid_page(4) && !p.valid_page(0));
        assert_eq!(p.range(1), (0..8, 0));
        assert_eq!(p.range(3), (16..20, 4), "four entries, four empty lines");
        assert_eq!(Paginator::new(0).max_page(), 1);
        assert_eq!(Paginator::new(0).range(1), (0..0, 8));
        assert_eq!(p.page_from(&mut Args::parse("")).unwrap(), 1);
        assert_eq!(p.page_from(&mut Args::parse("2")).unwrap(), 2);
        assert!(p.page_from(&mut Args::parse("9")).is_err());
        assert!(p.page_from(&mut Args::parse("1 2")).is_err());
    }

    #[test]
    fn argument_requirements() {
        let a = Args::parse("a b");
        assert!(a.require_max(2).is_ok() && a.require_max(1).is_err());
        assert!(a.require_min(2).is_ok() && a.require_min(3).is_err());
        assert!(a.require_exactly(2).is_ok() && a.require_exactly(1).is_err());
        assert!(a.has_exactly(2));
    }

    #[test]
    fn goals_print_the_way_baritone_does() {
        use crate::baritone_goals::*;
        assert_eq!(GoalBlock([1, 2, 3]).label(), "GoalBlock{x=1,y=2,z=3}");
        assert_eq!(GoalXZ(4, 5).label(), "GoalXZ{x=4,z=5}");
        assert_eq!(GoalYLevel(70).label(), "GoalYLevel{level=70}");
        assert_eq!(GoalNear([1, 2, 3], 4).label(), "GoalNear{x=1,y=2,z=3,rangeSq=16}");
        assert_eq!(GoalInverted(rc(GoalXZ(0, 0))).label(), "GoalInverted{GoalXZ{x=0,z=0}}");
        assert_eq!(pos_label([1, 2, 3]), "BetterBlockPos{x=1,y=2,z=3}");
        assert_eq!(direction_vector("EAST"), Some([1, 0, 0]));
        assert_eq!(column_ahead([0.5, 64.0, 0.5], 0.0, 10.0), (0, 10));
        assert_eq!(column_ahead([0.5, 64.0, 0.5], -90.0, 10.0), (10, 0));
    }
}
