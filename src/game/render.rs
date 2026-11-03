// baritone.utils.PathRenderer: the path, what it breaks and places, the goal, the ore being mined, the
// next stretch of a tunnel and the selections, as lines the game draws over everything. Colors are
// settings (colorCurrentPath, colorGoalBox...).
use super::baritone::Baritone;
use super::baritone_goals::Shape;
use super::baritone_gworld::is_air_at;
use super::baritone_utils::Pos;
use super::Game;
use horizon_client::gizmos::{Gizmos, Line};

fn cube(g: &mut Gizmos, c: Pos, color: u32, width: f32) {
    g.cuboid(
        [f64::from(c[0]) - 0.002, f64::from(c[1]) - 0.002, f64::from(c[2]) - 0.002],
        [f64::from(c[0]) + 1.002, f64::from(c[1]) + 1.002, f64::from(c[2]) + 1.002],
        color,
        width,
    );
}

pub fn lines(b: &Baritone, game: &Game<'_>, out: &mut Vec<Line>) {
    let s = &b.core.settings;
    let mut g = Gizmos::default();
    let at = game.player.as_ref().map(|p| [p.position.x, p.position.y, p.position.z]);
    let pathing = &b.core.pathing;

    if s.flag("renderGoal") {
        // The goal in force, or the one set and not gone to yet.
        let goal = pathing.goal().cloned().or_else(|| b.procs.goal.goal().cloned());
        if let Some(goal) = goal {
            let (good, bad) = (s.color("colorGoalBox"), s.color("colorInvertedGoalBox"));
            for shape in goal.shapes() {
                match shape {
                    Shape::Block(p) => g.cuboid([f64::from(p[0]), f64::from(p[1]), f64::from(p[2])], [f64::from(p[0] + 1), f64::from(p[1] + 2), f64::from(p[2] + 1)], good, 3.0),
                    Shape::Avoid(p) => g.cuboid([f64::from(p[0]), f64::from(p[1]), f64::from(p[2])], [f64::from(p[0] + 1), f64::from(p[1] + 2), f64::from(p[2] + 1)], bad, 3.0),
                    Shape::Column(x, z) if s.flag("renderGoalXZBeacon") => {
                        let y = at.map_or(64.0, |a| a[1]);
                        g.cuboid([f64::from(x), y - 4.0, f64::from(z)], [f64::from(x + 1), y + 80.0, f64::from(z + 1)], good, 3.0);
                    }
                    Shape::Column(..) => {}
                    Shape::Level(y) => {
                        if let Some(a) = at {
                            let (x, z) = (a[0].floor(), a[2].floor());
                            g.cuboid([x - 2.0, f64::from(y), z - 2.0], [x + 3.0, f64::from(y) + 0.02, z + 3.0], good, 3.0);
                        }
                    }
                }
            }
        }
    }

    if s.flag("renderPath") {
        let (current_color, next_color) = (s.color("colorCurrentPath"), s.color("colorNextPath"));
        let line_of = |g: &mut Gizmos, path: &[Pos], color: u32| {
            let p = |n: Pos| [f64::from(n[0]) + 0.5, f64::from(n[1]) + 0.08, f64::from(n[2]) + 0.5];
            for w in path.windows(2) {
                g.line(p(w[0]), p(w[1]), color, 4.0);
            }
        };
        if let Some(ex) = pathing.current.as_ref() {
            let from = ex.position().min(ex.path.positions.len());
            let rest = &ex.path.positions[from..];
            line_of(&mut g, &rest[..rest.len().min(240)], current_color);
        }
        if let Some(next) = pathing.next.as_ref() {
            line_of(&mut g, &next.path.positions[..next.path.positions.len().min(240)], next_color);
        }
        let (break_color, place_color) = (s.color("colorBlocksToBreak"), s.color("colorBlocksToPlace"));
        if let Some(ex) = pathing.current.as_ref() {
            for p in ex.to_break().iter().filter(|p| !is_air_at(game, **p)).take(80) {
                cube(&mut g, *p, break_color, 2.5);
            }
            for p in ex.to_place().iter().filter(|p| is_air_at(game, **p)).take(80) {
                cube(&mut g, *p, place_color, 2.5);
            }
        }
        // What the processes are after.
        let mine_color = s.color("colorBlocksToBreak");
        for p in b.procs.mine.targets().iter().take(12) {
            cube(&mut g, *p, mine_color, 2.5);
        }
        for p in b.procs.build.incorrect().take(200) {
            let breaking = !is_air_at(game, p);
            let color = if breaking { break_color } else { place_color };
            cube(&mut g, p, color, 1.5);
        }
    }

    if s.flag("renderSelectionBoxes") {
        let color = s.color("colorSelection");
        for sel in &b.core.selections.selections {
            g.cuboid(
                [f64::from(sel.min()[0]), f64::from(sel.min()[1]), f64::from(sel.min()[2])],
                [f64::from(sel.max()[0] + 1), f64::from(sel.max()[1] + 1), f64::from(sel.max()[2] + 1)],
                color,
                3.0,
            );
        }
        if let Some(p) = b.core.sel_pos1 {
            cube(&mut g, p, s.color("colorSelectionPos1"), 3.5);
        }
    }
    out.extend(g.lines);
}
