// baritone.pathing.movement.Moves and the movements/ package: every move one node can make, and what
// it costs. A move is generated only when its cells are free, so the search never plans through a
// wall; the executor (path.rs) is what carries it out.
use super::baritone_costs::*;
use super::baritone_legacy_helper::{open, safe_to_break, stand, CalcSettings, Cell, Kind, Terrain};
use super::baritone_utils::{at, up, Pos, CARDINAL};

/// How one node is reached from the one before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mv {
    /// Walk, jump up, drop, swim, climb, or dig through.
    Walk,
    /// The floor of the next node is placed from the edge of this one.
    Bridge,
    /// Jump straight up and place the block you stood in under yourself.
    Pillar,
    /// A sprint jump across a gap.
    Parkour,
}

/// Opening a door takes a click and a moment.
const DOOR: f64 = 6.0;
/// A water bucket landing: take it out, aim, place it, pick it up again.
const BUCKET: f64 = 14.0;

fn door_cost<T: Terrain + ?Sized>(t: &mut T, n: Pos) -> f64 {
    if t.cell(n) == Cell::Door || t.cell(up(n, 1)) == Cell::Door { DOOR } else { 0.0 }
}

/// The ticks to walk one block on the flat.
fn walk(s: &CalcSettings) -> f64 {
    if s.allow_sprint { SPRINT_ONE_BLOCK_COST } else { WALK_ONE_BLOCK_COST }
}

/// The way down from `p` onto `to`'s column: a drop of `k` blocks lands on the first standable
/// cell, and every cell it falls through must be free.
fn descend<T: Terrain + ?Sized>(t: &mut T, p: Pos, dx: i32, dz: i32, s: &CalcSettings) -> Option<(Pos, f64)> {
    let column = at(p, dx, 0, dz);
    // The first step off the edge: the player's body must pass at the height of `p`.
    if !t.cell(column).free() || !t.cell(up(column, 1)).free() {
        return None;
    }
    let limit = s.max_fall_into_water.max(s.max_fall).max(s.max_fall_bucket);
    for k in 1..=limit {
        let land = at(p, dx, -k, dz);
        let below = t.cell(up(land, -1));
        let here = t.cell(land);
        if matches!(here, Cell::Lava | Cell::Danger | Cell::Unloaded) || matches!(below, Cell::Lava | Cell::Danger | Cell::Unloaded) {
            return None;
        }
        if !here.free() && here != Cell::Low {
            return None;
        }
        if let Some(kind) = stand(t, land) {
            let ok = match kind {
                Kind::Ground => k <= s.max_fall.max(s.max_fall_bucket),
                Kind::Swim | Kind::Ladder => k <= s.max_fall_into_water,
            };
            if !ok {
                return None;
            }
            let into_water = here == Cell::Water || below == Cell::Water;
            let mut cost = WALK_OFF_BLOCK_COST + fall_cost(k) * if into_water { 0.6 } else { 1.0 } + CENTER_AFTER_FALL_COST;
            if kind == Kind::Ground && k > s.max_fall {
                cost += BUCKET;
            }
            return Some((land, cost));
        }
        if t.cell(up(land, 1)) == Cell::Lava {
            return None;
        }
    }
    None
}

/// Every move from `p`: where it ends and what it costs.
pub fn moves<T: Terrain + ?Sized>(t: &mut T, p: Pos, from: Kind, s: &CalcSettings, out: &mut Vec<(Pos, f64, Mv)>) {
    out.clear();
    const DIAGONAL_STEPS: [[i32; 2]; 4] = [[1, 1], [1, -1], [-1, 1], [-1, -1]];
    let flat = walk(s);

    for [dx, dz] in CARDINAL {
        // Traverse: the same height.
        let n = at(p, dx, 0, dz);
        if let Some(kind) = stand(t, n) {
            let cost = match (from, kind) {
                (_, Kind::Swim) | (Kind::Swim, _) => {
                    if !s.allow_swim {
                        continue;
                    }
                    WALK_ONE_IN_WATER_COST
                }
                (_, Kind::Ladder) => LADDER_UP_ONE_COST,
                _ => flat,
            };
            out.push((n, cost + door_cost(t, n), Mv::Walk));
            continue;
        }
        // Ascend: one up, with room to jump from where we are and to land.
        if t.cell(up(p, 2)).free()
            && let Some(kind) = stand(t, at(p, dx, 1, dz))
            && kind != Kind::Swim
        {
            let n = at(p, dx, 1, dz);
            out.push((n, WALK_ONE_BLOCK_COST + s.jump_penalty + jump_one_block_cost() + door_cost(t, n), Mv::Walk));
            continue;
        }
        // Descend: walk off an edge.
        if let Some((land, cost)) = descend(t, p, dx, dz, s) {
            out.push((land, cost, Mv::Walk));
        }
    }
    // Diagonals: both corners clear, so the body does not clip one.
    for [dx, dz] in DIAGONAL_STEPS {
        let n = at(p, dx, 0, dz);
        let side_a = at(p, dx, 0, 0);
        let side_b = at(p, 0, 0, dz);
        // A closed door is a wall for a body cutting a corner: it is opened head on, or not at all.
        let clear = |t: &mut T, c: Pos| open(t, c) && open(t, up(c, 1));
        if clear(t, side_a) && clear(t, side_b) && clear(t, n) && stand(t, n) == Some(Kind::Ground) && from == Kind::Ground {
            out.push((n, flat * std::f64::consts::SQRT_2, Mv::Walk));
        }
    }
    // Ladders and vines: up and down in the column.
    if from == Kind::Ladder {
        let above = up(p, 1);
        if t.cell(above) == Cell::Climb && t.cell(up(p, 2)).free() {
            out.push((above, LADDER_UP_ONE_COST, Mv::Walk));
        }
        let below = up(p, -1);
        if t.cell(below) == Cell::Climb {
            out.push((below, LADDER_DOWN_ONE_COST, Mv::Walk));
        }
    }
    // Climbing onto the top of a ladder: the cell above the last rung is free, so step onto it.
    if from == Kind::Ladder && t.cell(up(p, 1)) != Cell::Climb {
        for [dx, dz] in CARDINAL {
            let top = at(p, dx, 1, dz);
            if stand(t, top) == Some(Kind::Ground) && t.cell(up(p, 2)).free() {
                out.push((top, WALK_ONE_BLOCK_COST + s.jump_penalty + jump_one_block_cost(), Mv::Walk));
            }
        }
    }
    if s.allow_break {
        dig_moves(t, p, s, out);
    }
    if from == Kind::Ground && t.cell(p) != Cell::Low {
        if s.allow_place {
            place_moves(t, p, s, out);
        }
        if s.allow_parkour {
            parkour_moves(t, p, s, out);
        }
    }
    // Swimming up and down.
    if from == Kind::Swim && s.allow_swim {
        if t.cell(up(p, 1)) == Cell::Water && t.cell(up(p, 2)).free() {
            out.push((up(p, 1), WALK_ONE_IN_WATER_COST, Mv::Walk));
        }
        let below = up(p, -1);
        if t.cell(below) == Cell::Water && t.cell(up(below, -1)) != Cell::Lava {
            out.push((below, WALK_ONE_IN_WATER_COST * 0.7, Mv::Walk));
        }
    }
}

/// The cells a body passes through to get from `p` to the adjacent `n`, the new feet cell first.
/// Empty for a move that is not a single step (a long fall, a diagonal): those are planned free.
pub fn required_free(p: Pos, n: Pos) -> Vec<Pos> {
    let (dx, dy, dz) = (n[0] - p[0], n[1] - p[1], n[2] - p[2]);
    let horizontal = dx.abs() + dz.abs();
    match (horizontal, dy) {
        (1, 0) => vec![n, up(n, 1)],
        (1, 1) => vec![n, up(n, 1), up(p, 2)],
        (1, -1) => vec![n, up(n, 1), up(n, 2)],
        (0, -1) => vec![n],
        _ => Vec::new(),
    }
}

/// The ticks to clear `cells` for a body, or `None` when something in them cannot be cleared. The
/// first cell is the new feet cell, which may be a low floor to walk onto.
fn clearing<T: Terrain + ?Sized>(t: &mut T, cells: &[Pos], s: &CalcSettings) -> Option<f64> {
    let mut total = 0.0;
    for (i, c) in cells.iter().enumerate() {
        match t.cell(*c) {
            Cell::Air | Cell::Climb | Cell::Water | Cell::Door => {}
            Cell::Low if i == 0 => {}
            Cell::Solid => {
                if !safe_to_break(t, *c) {
                    return None;
                }
                total += t.dig_cost(*c)? + s.break_penalty;
            }
            _ => return None,
        }
    }
    Some(total)
}

fn dig_moves<T: Terrain + ?Sized>(t: &mut T, p: Pos, s: &CalcSettings, out: &mut Vec<(Pos, f64, Mv)>) {
    const STEPS: [[i32; 3]; 12] = [
        [1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1],
        [1, 1, 0], [-1, 1, 0], [0, 1, 1], [0, 1, -1],
        [1, -1, 0], [-1, -1, 0], [0, -1, 1], [0, -1, -1],
    ];
    let mut tried: Vec<[i32; 3]> = STEPS.to_vec();
    if s.allow_downward {
        tried.push([0, -1, 0]);
    }
    for [dx, dy, dz] in tried {
        let n = at(p, dx, dy, dz);
        if out.iter().any(|(q, ..)| *q == n) {
            continue;
        }
        let cells = required_free(p, n);
        let Some(dig) = clearing(t, &cells, s) else {
            continue;
        };
        // Nothing to dig is a move the ordinary rules already took or turned down.
        if dig == 0.0 {
            continue;
        }
        // What the player stands on afterwards: solid, never lava or air.
        if !t.cell(up(n, -1)).holds() && t.cell(n) != Cell::Low {
            continue;
        }
        let base = match dy {
            0 => walk(s),
            1 => WALK_ONE_BLOCK_COST + s.jump_penalty + jump_one_block_cost(),
            _ => WALK_OFF_BLOCK_COST + fall_cost(1) + CENTER_AFTER_FALL_COST,
        };
        out.push((n, base + dig, Mv::Walk));
    }
}

/// Bridging over a gap and pillaring up: a block is put where the floor is missing. The planner
/// assumes the block is there once the move is made, so a bridge can run on for as long as the
/// hotbar holds blocks.
fn place_moves<T: Terrain + ?Sized>(t: &mut T, p: Pos, s: &CalcSettings, out: &mut Vec<(Pos, f64, Mv)>) {
    for [dx, dz] in CARDINAL {
        let n = at(p, dx, 0, dz);
        if out.iter().any(|(q, ..)| *q == n) {
            continue;
        }
        if t.cell(n) == Cell::Air && t.cell(up(n, 1)) == Cell::Air && t.cell(up(n, -1)) == Cell::Air {
            out.push((n, SNEAK_ONE_BLOCK_COST + s.place_cost, Mv::Bridge));
        }
    }
    let n = up(p, 1);
    if open(t, up(p, 2)) && t.cell(up(p, 2)) != Cell::Water && !out.iter().any(|(q, ..)| *q == n) {
        out.push((n, jump_one_block_cost() + s.place_cost + s.jump_penalty, Mv::Pillar));
    }
}

/// Sprint jumps over gaps of one to three blocks, level, one up (two blocks of gap at most) or
/// one down. A long jump needs a block to run up from behind.
fn parkour_moves<T: Terrain + ?Sized>(t: &mut T, p: Pos, s: &CalcSettings, out: &mut Vec<(Pos, f64, Mv)>) {
    if !open(t, up(p, 2)) {
        return;
    }
    for [dx, dz] in CARDINAL {
        let run_up = stand(t, at(p, -dx, 0, -dz)) == Some(Kind::Ground);
        for (dy, widest) in [(0, 3), (1, 2), (-1, 3)] {
            for gap in 1..=widest {
                if !run_up && (gap == 3 || (dy == 1 && gap == 2)) {
                    continue;
                }
                let land = at(p, dx * (gap + 1), dy, dz * (gap + 1));
                if stand(t, land) != Some(Kind::Ground) || out.iter().any(|(q, ..)| *q == land) {
                    continue;
                }
                let lift = if dy == 1 { 1 } else { 0 };
                let clear = (1..=gap + 1).all(|k| {
                    let c = at(p, dx * k, 0, dz * k);
                    (0..=1 + lift).all(|h| open(t, up(c, h)))
                });
                if !clear {
                    continue;
                }
                out.push((land, f64::from(gap + 1) * SPRINT_ONE_BLOCK_COST + s.jump_penalty + 2.0 + f64::from(dy.abs()) * 2.0, Mv::Parkour));
            }
        }
    }
}
