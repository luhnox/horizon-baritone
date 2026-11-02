// Baritone's settings page.
fn baritone_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::game::baritone as baritone;
    if mod_id != baritone::ID {
        return;
    }
    buttons.extend(baritone::page_labels(game));
}
