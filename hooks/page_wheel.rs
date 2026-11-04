// The wheel on Baritone's page in Options > Mods.
fn baritone_page_wheel(game: &mut crate::game::Game<'_>, amount: f32) -> core::ops::ControlFlow<Result<()>> {
    if !crate::game::baritone::page_open(game) {
        return core::ops::ControlFlow::Continue(());
    }
    crate::game::baritone::page_wheel(game, amount);
    core::ops::ControlFlow::Break(Ok(()))
}
