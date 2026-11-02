// baritone.utils.InputOverrideHandler: the keys Baritone holds down, so that releasing them is one
// call and nothing is left pressed when it stops.
use super::{Game, Screen};
use std::cell::Cell;
use std::collections::HashSet;
use winit::keyboard::KeyCode;

thread_local! {
    /// What Baritone holds down: forward, sprint, jump, sneak. The game reads the keys only while it
    /// is playing, so with the chat or a menu open this is added to them on the way into the motion.
    static HELD: Cell<[bool; 4]> = const { Cell::new([false; 4]) };
}

thread_local! {
    /// The screen the tick turned into the playing one, to give back.
    static BORROWED: Cell<Option<Screen>> = const { Cell::new(None) };
}

fn held_keys() -> [(KeyCode, bool); 4] {
    let [forward, sprint, jump, sneak] = HELD.with(Cell::get);
    [(KeyCode::KeyW, forward), (KeyCode::ControlLeft, sprint), (KeyCode::Space, jump), (KeyCode::ShiftLeft, sneak)]
}

/// Just before the motion reads the keys: with the chat or a menu open and Baritone holding keys, the
/// game plays for this one tick, as the Java Baritone walks with any GUI open. The keys go into the
/// game's set only for it.
pub fn through(game: &mut Game<'_>) {
    if game.screen == Screen::Playing || game.screen == Screen::Dead || BORROWED.with(Cell::get).is_some() {
        return;
    }
    let keys = held_keys();
    if keys.iter().all(|(_, on)| !on) {
        return;
    }
    for (key, on) in keys {
        if on {
            game.keys.insert(key);
        }
    }
    BORROWED.with(|b| b.set(Some(game.screen)));
    game.screen = Screen::Playing;
}

/// After the motion has read them: the chat is the chat again, without Baritone's keys.
pub fn back(game: &mut Game<'_>) {
    let Some(screen) = BORROWED.with(Cell::get) else {
        return;
    };
    BORROWED.with(|b| b.set(None));
    for (key, on) in held_keys() {
        if on {
            game.keys.remove(&key);
        }
    }
    if game.screen == Screen::Playing {
        game.screen = screen;
    }
}

#[derive(Default)]
pub struct Input {
    held: HashSet<KeyCode>,
}

impl Input {
    pub fn any_held(&self) -> bool {
        !self.held.is_empty()
    }

    /// Let go of everything held.
    pub fn release(&mut self, game: &mut Game<'_>) {
        for key in self.held.drain() {
            game.keys.remove(&key);
        }
        self.publish();
    }

    fn publish(&self) {
        let has = |k| self.held.contains(&k);
        HELD.with(|h| h.set([has(KeyCode::KeyW), has(KeyCode::ControlLeft), has(KeyCode::Space), has(KeyCode::ShiftLeft)]));
    }

    /// Hold the keys marked on and let go of the others.
    pub fn hold(&mut self, game: &mut Game<'_>, wanted: [(KeyCode, bool); 4]) {
        // With a screen open the keys stay out of the game's own set (the chat would read a held
        // Ctrl as a shortcut); the motion gets them through `apply`.
        let playing = game.screen == Screen::Playing;
        for (key, on) in wanted {
            if on {
                if playing {
                    game.keys.insert(key);
                } else {
                    game.keys.remove(&key);
                }
                self.held.insert(key);
            } else if self.held.remove(&key) {
                game.keys.remove(&key);
            }
        }
        self.publish();
    }

    /// Stand still with these keys and no others.
    pub fn only(&mut self, game: &mut Game<'_>, sneak: bool, forward: bool, jump: bool) {
        self.hold(
            game,
            [(KeyCode::KeyW, forward), (KeyCode::ControlLeft, false), (KeyCode::Space, jump), (KeyCode::ShiftLeft, sneak)],
        );
    }

    /// Walk: forward, sprint, jump and sneak as asked.
    pub fn walk(&mut self, game: &mut Game<'_>, forward: bool, sprint: bool, jump: bool, sneak: bool) {
        self.hold(
            game,
            [(KeyCode::KeyW, forward), (KeyCode::ControlLeft, sprint), (KeyCode::Space, jump), (KeyCode::ShiftLeft, sneak)],
        );
    }
}

/// Whether the player is pressing the movement keys themselves: they take the wheel back.
pub fn player_took_over(game: &Game<'_>) -> bool {
    [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD]
        .iter()
        .any(|k| game.physical_keys.contains(k))
}
