// baritone.api.utils: positions, directions and the yaw maths every other part uses. Plain Rust, no
// game types, so it is tested on its own.

/// A block position: x, y, z.
pub type Pos = [i32; 3];

/// The four horizontal neighbours as (dx, dz).
pub const CARDINAL: [[i32; 2]; 4] = [[1, 0], [-1, 0], [0, 1], [0, -1]];

pub fn at(p: Pos, dx: i32, dy: i32, dz: i32) -> Pos {
    [p[0] + dx, p[1] + dy, p[2] + dz]
}

pub fn up(p: Pos, n: i32) -> Pos {
    [p[0], p[1] + n, p[2]]
}

pub fn manhattan(a: Pos, b: Pos) -> i32 {
    (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs()
}

pub fn dist_sq(a: Pos, b: Pos) -> i64 {
    let d = [i64::from(a[0] - b[0]), i64::from(a[1] - b[1]), i64::from(a[2] - b[2])];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

pub fn dist_xz(a: Pos, b: Pos) -> f64 {
    f64::from(a[0] - b[0]).hypot(f64::from(a[2] - b[2]))
}

/// The block a position is in. A player standing on the ground at y = 64.0 is in y = 64.
pub fn feet_of(p: [f64; 3]) -> Pos {
    [p[0].floor() as i32, (p[1] + 0.01).floor() as i32, p[2].floor() as i32]
}

/// The middle of a block, as (x, z).
pub fn centre(n: Pos) -> [f64; 2] {
    [f64::from(n[0]) + 0.5, f64::from(n[2]) + 0.5]
}

/// Yaw for a vector, in the game's convention: 0 faces south (+z), 90 west (-x).
pub fn yaw_to(dx: f64, dz: f64) -> f32 {
    (-dx).atan2(dz).to_degrees() as f32
}

/// Degrees brought into -180..180.
pub fn wrap_degrees(mut d: f32) -> f32 {
    d %= 360.0;
    if d > 180.0 {
        d -= 360.0;
    }
    if d < -180.0 {
        d += 360.0;
    }
    d
}

/// Turn from `from` towards `to` by at most `max` degrees, the short way round.
pub fn approach_yaw(from: f32, to: f32, max: f32) -> f32 {
    from + wrap_degrees(to - from).clamp(-max, max)
}

/// The cardinal direction a yaw faces, as (dx, dz).
pub fn facing_step(yaw: f32) -> (i32, i32) {
    let r = f64::from(yaw).to_radians();
    let (x, z) = (-r.sin(), r.cos());
    if x.abs() > z.abs() { (x.signum() as i32, 0) } else { (0, z.signum() as i32) }
}

/// A direction by name: `north` is -z, `south` +z, `east` +x, `west` -x.
pub fn direction_named(name: &str) -> Option<(i32, i32)> {
    match name.to_ascii_lowercase().as_str() {
        "north" | "n" => Some((0, -1)),
        "south" | "s" => Some((0, 1)),
        "east" | "e" => Some((1, 0)),
        "west" | "w" => Some((-1, 0)),
        _ => None,
    }
}

pub fn direction_label(step: (i32, i32)) -> &'static str {
    match step {
        (0, -1) => "north",
        (0, 1) => "south",
        (1, 0) => "east",
        (-1, 0) => "west",
        _ => "nowhere",
    }
}

/// A direction by name, as a unit vector: down, up, north (-z), south (+z), west (-x), east (+x).
pub fn direction_vector(name: &str) -> Option<[i32; 3]> {
    match name.to_ascii_lowercase().as_str() {
        "down" => Some([0, -1, 0]),
        "up" => Some([0, 1, 0]),
        "north" => Some([0, 0, -1]),
        "south" => Some([0, 0, 1]),
        "west" => Some([-1, 0, 0]),
        "east" => Some([1, 0, 0]),
        _ => None,
    }
}

thread_local! {
    /// `censorCoordinates`, kept where the code that prints a coordinate can reach it.
    static CENSOR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub fn set_censor(on: bool) {
    CENSOR.with(|c| c.set(on));
}

/// `SettingsUtil.maybeCensor`: a coordinate as text, or `<censored>`.
pub fn maybe_censor(coordinate: i32) -> String {
    if CENSOR.with(std::cell::Cell::get) { "<censored>".to_owned() } else { coordinate.to_string() }
}

/// A block position as Baritone prints it.
pub fn pos_label(p: Pos) -> String {
    format!("BetterBlockPos{{x={},y={},z={}}}", maybe_censor(p[0]), maybe_censor(p[1]), maybe_censor(p[2]))
}

#[cfg(test)]
mod censor_tests {
    use super::*;

    #[test]
    fn coordinates_are_hidden_while_the_setting_is_on() {
        set_censor(true);
        assert_eq!(pos_label([1, 2, 3]), "BetterBlockPos{x=<censored>,y=<censored>,z=<censored>}");
        set_censor(false);
        assert_eq!(pos_label([1, 2, 3]), "BetterBlockPos{x=1,y=2,z=3}");
    }
}

/// The column a distance away along a yaw (GoalXZ.fromDirection): 0 faces south, 90 west.
pub fn column_ahead(from: [f64; 3], yaw: f64, distance: f64) -> (i32, i32) {
    let r = yaw.to_radians();
    ((from[0] - r.sin() * distance).floor() as i32, (from[2] + r.cos() * distance).floor() as i32)
}
