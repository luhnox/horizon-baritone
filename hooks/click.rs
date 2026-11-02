// A click on Baritone's settings page.
fn baritone_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::baritone as baritone;
    if mod_id != baritone::ID {
        return;
    }
    baritone::page_click(game, button);
}
