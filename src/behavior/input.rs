// baritone.utils.InputOverrideHandler: the inputs Baritone holds down (`setInputForceState`), applied to the
// game once a tick, so that letting go of all of them is one call and nothing is left pressed when it
// stops. The movement keys go into the game's key set; the two clicks hold the attack button and use the
// held item. With a menu or the chat open the game ignores its key set, so the keys are also published for
// the hook that lets Baritone keep walking with a screen open (see `through`).
use super::baritone_movement_state::Input as In;
use super::{Game, Screen};
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use winit::keyboard::KeyCode;

thread_local! {
    /// What Baritone holds down: forward, back, left, right, sprint, jump, sneak. The game reads the keys
    /// only while it is playing, so with the chat or a menu open this is put into its key set for one tick.
    static HELD: Cell<[bool; 7]> = const { Cell::new([false; 7]) };
    /// The screen the tick turned into the playing one, to give back.
    static BORROWED: Cell<Option<Screen>> = const { Cell::new(None) };
}

const KEYS: [(In, KeyCode); 7] = [
    (In::MoveForward, KeyCode::KeyW),
    (In::MoveBack, KeyCode::KeyS),
    (In::MoveLeft, KeyCode::KeyA),
    (In::MoveRight, KeyCode::KeyD),
    (In::Sprint, KeyCode::ControlLeft),
    (In::Jump, KeyCode::Space),
    (In::Sneak, KeyCode::ShiftLeft),
];

fn held_keys() -> Vec<KeyCode> {
    let bits = HELD.with(Cell::get);
    KEYS.iter().zip(bits).filter(|(_, on)| *on).map(|((_, k), _)| *k).collect()
}

/// Just before the motion reads the keys: with the chat or a menu open and Baritone holding keys, the
/// game plays for this one tick, as the Java Baritone walks with any GUI open. The keys go into the
/// game's set only for it.
pub fn through(game: &mut Game<'_>) {
    if game.screen == Screen::Playing || game.screen == Screen::Dead || BORROWED.with(Cell::get).is_some() {
        return;
    }
    let keys = held_keys();
    if keys.is_empty() {
        return;
    }
    for key in keys {
        game.keys.insert(key);
    }
    BORROWED.with(|b| b.set(Some(game.screen)));
    game.screen = Screen::Playing;
}

/// After the motion has read them: the screen is the screen again, without Baritone's keys.
pub fn back(game: &mut Game<'_>) {
    let Some(screen) = BORROWED.with(Cell::get) else {
        return;
    };
    BORROWED.with(|b| b.set(None));
    for key in held_keys() {
        game.keys.remove(&key);
    }
    if game.screen == Screen::Playing {
        game.screen = screen;
    }
}

#[derive(Default)]
pub struct Input {
    forced: HashMap<In, bool>,
    /// The keys put into the game's set, so they can be taken out again.
    down: HashSet<KeyCode>,
    attacking: bool,
    place_cooldown: i32,
}

impl Input {
    pub fn any_held(&self) -> bool {
        self.forced.values().any(|v| *v)
    }

    // ---- InputOverrideHandler

    pub fn is_input_forced_down(&self, input: In) -> bool {
        self.forced.get(&input).copied().unwrap_or(false)
    }

    pub fn set_input_force_state(&mut self, input: In, forced: bool) {
        self.forced.insert(input, forced);
    }

    /// `clearAllKeys`: nothing forced any more (applied at the next `apply`).
    pub fn clear_all_keys(&mut self) {
        self.forced.clear();
    }

    /// Put what is forced into the game: once a tick, after everything has set its inputs.
    pub fn apply(&mut self, game: &mut Game<'_>) {
        let playing = game.screen == Screen::Playing;
        let mut bits = [false; 7];
        for (i, (input, key)) in KEYS.iter().enumerate() {
            let on = self.is_input_forced_down(*input);
            bits[i] = on;
            if on && playing {
                game.keys.insert(*key);
                self.down.insert(*key);
            } else if self.down.remove(key) {
                game.keys.remove(key);
            }
        }
        HELD.with(|h| h.set(bits));
        let attack = self.is_input_forced_down(In::ClickLeft);
        if attack != self.attacking {
            game.bar_attack(attack);
            self.attacking = attack;
        } else if !attack && game.bar_attacking() {
            game.bar_attack(false);
        }
        if self.place_cooldown > 0 {
            self.place_cooldown -= 1;
        }
        if self.is_input_forced_down(In::ClickRight) && self.place_cooldown <= 0 {
            game.bar_use();
            self.place_cooldown = 4; // BlockPlaceHelper's rightClickSpeed
        }
    }

    /// Let go of everything held, at once.
    pub fn release(&mut self, game: &mut Game<'_>) {
        self.clear_all_keys();
        self.apply(game);
    }

    // ---- the older helpers the processes use

    /// Hold the keys marked on and let go of the others (W, Ctrl, Space, Shift only).
    pub fn hold(&mut self, game: &mut Game<'_>, wanted: [(KeyCode, bool); 4]) {
        for (key, on) in wanted {
            if let Some((input, _)) = KEYS.iter().find(|(_, k)| *k == key) {
                self.forced.insert(*input, on);
            }
        }
        self.apply(game);
    }

    /// Stand still with these keys and no others.
    pub fn only(&mut self, game: &mut Game<'_>, sneak: bool, forward: bool, jump: bool) {
        self.hold(game, [(KeyCode::KeyW, forward), (KeyCode::ControlLeft, false), (KeyCode::Space, jump), (KeyCode::ShiftLeft, sneak)]);
    }

    /// Walk: forward, sprint, jump and sneak as asked.
    pub fn walk(&mut self, game: &mut Game<'_>, forward: bool, sprint: bool, jump: bool, sneak: bool) {
        self.hold(game, [(KeyCode::KeyW, forward), (KeyCode::ControlLeft, sprint), (KeyCode::Space, jump), (KeyCode::ShiftLeft, sneak)]);
    }
}

/// Whether the player is pressing the movement keys themselves: they take the wheel back.
pub fn player_took_over(game: &Game<'_>) -> bool {
    [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD].iter().any(|k| game.physical_keys.contains(k))
}
