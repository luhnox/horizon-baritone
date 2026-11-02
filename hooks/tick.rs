// Once a tick: walk the path if there is one. Turned off in Options > Mods: let go and stop.
fn baritone_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::baritone as baritone;
    if game.native_on(baritone::ID) {
        baritone::tick(game);
    } else {
        baritone::stop(game);
    }
}
