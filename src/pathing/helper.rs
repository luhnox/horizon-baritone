// baritone.pathing.movement.MovementHelper and CalculationContext: what a block means for walking,
// whether a player can stand in a cell, and the settings a search runs with.
use super::baritone_utils::{at, up, Pos};

/// What one block means for walking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// Nothing in the way: air, grass, flowers, a torch.
    Air,
    /// A full cube: something to stand on, something to go around.
    Solid,
    Water,
    Lava,
    /// A ladder, a vine, scaffolding: free to occupy, and the player can climb in it.
    Climb,
    /// Hurts or holds the player: fire, cactus, magma, berries, cobweb, powder snow.
    Danger,
    /// A low floor to stand IN rather than on: a bottom slab, a stair, a carpet, a snow layer. The
    /// player walks onto it without jumping, and its cell is the one its feet are in.
    Low,
    /// A shape that is neither a full cube nor a low floor (a fence, a chest, a pane): neither
    /// passable nor safe to stand on.
    Partial,
    /// A closed door or fence gate that opens with a click: free to plan through, opened on the way.
    Door,
    /// Outside the loaded world.
    Unloaded,
}

impl Cell {
    pub fn free(self) -> bool {
        matches!(self, Cell::Air | Cell::Water | Cell::Climb | Cell::Door)
    }
    pub fn holds(self) -> bool {
        matches!(self, Cell::Solid)
    }
}

/// The world as a search sees it.
pub trait Terrain {
    fn cell(&mut self, p: Pos) -> Cell;
    /// The ticks it takes to break the block at `p` with the best tool in hand, or `None` when it
    /// cannot or should not be broken (bedrock, no tool for it, a chest). Plain terrain digs nothing.
    fn dig_cost(&mut self, _p: Pos) -> Option<f64> {
        None
    }
}

/// baritone's CalculationContext: what a search may do and what it costs.
#[derive(Clone, Debug)]
pub struct CalcSettings {
    pub max_nodes: usize,
    /// How far the player may drop onto solid ground. Water lets it fall further.
    pub max_fall: i32,
    pub max_fall_into_water: i32,
    /// The longest drop a water bucket takes (0: no bucket in the hotbar, or not allowed).
    pub max_fall_bucket: i32,
    pub allow_swim: bool,
    pub allow_sprint: bool,
    /// Tunnel through blocks that are in the way.
    pub allow_break: bool,
    /// Ticks added to every block broken: a path through open air wins over a short dig.
    pub break_penalty: f64,
    /// Put blocks down: bridge over a gap, pillar up. Only with something to place.
    pub allow_place: bool,
    /// Ticks added to every block placed.
    pub place_cost: f64,
    /// Sprint jumps over gaps of one to three blocks.
    pub allow_parkour: bool,
    /// Ticks added to every jump.
    pub jump_penalty: f64,
    /// Dig straight down.
    pub allow_downward: bool,
}

impl Default for CalcSettings {
    fn default() -> Self {
        Self {
            max_nodes: 40_000,
            max_fall: 3,
            max_fall_into_water: 20,
            max_fall_bucket: 0,
            allow_swim: true,
            allow_sprint: true,
            allow_break: false,
            break_penalty: 2.0,
            allow_place: false,
            place_cost: 20.0,
            allow_parkour: false,
            jump_penalty: 2.0,
            allow_downward: true,
        }
    }
}

/// How a player is in a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Ground,
    Swim,
    Ladder,
}

/// Free, and not a door that has to be opened first.
pub fn open<T: Terrain + ?Sized>(t: &mut T, c: Pos) -> bool {
    t.cell(c).free() && t.cell(c) != Cell::Door
}

/// Whether a player can occupy `p`, and how.
pub fn stand<T: Terrain + ?Sized>(t: &mut T, p: Pos) -> Option<Kind> {
    let feet = t.cell(p);
    let head = t.cell(up(p, 1));
    // On a low floor the feet are half a block up, so the head needs the cell above its own.
    if feet == Cell::Low {
        return (head.free() && t.cell(up(p, 2)).free()).then_some(Kind::Ground);
    }
    if !feet.free() || !head.free() {
        return None;
    }
    match feet {
        Cell::Water => Some(Kind::Swim),
        Cell::Climb => Some(Kind::Ladder),
        _ if head == Cell::Water && t.cell(at(p, 0, -1, 0)) == Cell::Water => Some(Kind::Swim),
        _ if t.cell(at(p, 0, -1, 0)).holds() => Some(Kind::Ground),
        // Over a ladder or in a vine: standing in it climbs it, standing above it falls.
        _ => None,
    }
}

/// Whether breaking the block at `c` is safe: nothing it holds back (lava, water) can come in.
pub fn safe_to_break<T: Terrain + ?Sized>(t: &mut T, c: Pos) -> bool {
    [[0, 1, 0], [0, -1, 0], [1, 0, 0], [-1, 0, 0], [0, 0, 1], [0, 0, -1]]
        .iter()
        .all(|d| !matches!(t.cell(at(c, d[0], d[1], d[2])), Cell::Lava | Cell::Water))
}
