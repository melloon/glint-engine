//! The controller: a d-pad and four buttons, eight in all, so the whole
//! state fits in one byte with one bit per button.
//!
//! The game never looks at the keyboard. Once per tick the engine reads the
//! host keyboard, packs it into a controller byte, and the game asks the
//! controller questions like "is Right held?" or "was A just pressed?".

use macroquad::prelude::{KeyCode, is_key_down};

/// One bit per button, in the classic order.
#[derive(Clone, Copy)]
pub enum Button {
    A = 0x01,
    B = 0x02,
    Select = 0x04,
    Start = 0x08,
    Up = 0x10,
    Down = 0x20,
    Left = 0x40,
    Right = 0x80,
}

pub struct Controller {
    /// Buttons down on this tick.
    now: u8,
    /// Buttons that were down on the previous tick.
    before: u8,
}

impl Controller {
    pub fn new() -> Self {
        Self { now: 0, before: 0 }
    }

    /// Latch a new reading. Call exactly once per tick, before the game
    /// updates, so "just pressed" is true for exactly one tick.
    pub fn update(&mut self, raw: u8) {
        self.before = self.now;
        self.now = remove_opposites(raw);
    }

    /// True for as long as the button is down. Use for movement.
    pub fn held(&self, button: Button) -> bool {
        self.now & button as u8 != 0
    }

    /// True only on the tick the button goes down. Use for one-shot actions
    /// such as jumping or pausing.
    pub fn pressed(&self, button: Button) -> bool {
        self.now & !self.before & button as u8 != 0
    }
}

/// A real d-pad is a rocker: it cannot press Left and Right (or Up and Down)
/// together. A keyboard can, so cancel those pairs out. Game code can then
/// rely on never seeing both.
fn remove_opposites(mut raw: u8) -> u8 {
    for pair in [
        Button::Left as u8 | Button::Right as u8,
        Button::Up as u8 | Button::Down as u8,
    ] {
        if raw & pair == pair {
            raw &= !pair;
        }
    }
    raw
}

/// Read the host keyboard into a controller byte. This is the only part of
/// input that touches macroquad.
///
/// It deliberately reads whether keys are down *right now*, not macroquad's
/// per-frame "was pressed" events: frames and ticks do not line up one to one,
/// so a per-frame event can land on a frame that runs no tick and be lost.
pub fn read_keyboard() -> u8 {
    const KEYS: [(KeyCode, Button); 16] = [
        (KeyCode::Right, Button::Right),
        (KeyCode::D, Button::Right),
        (KeyCode::Left, Button::Left),
        (KeyCode::A, Button::Left),
        (KeyCode::Up, Button::Up),
        (KeyCode::W, Button::Up),
        (KeyCode::Down, Button::Down),
        (KeyCode::S, Button::Down),
        (KeyCode::X, Button::A),
        (KeyCode::K, Button::A),
        (KeyCode::Space, Button::A),
        (KeyCode::Z, Button::B),
        (KeyCode::J, Button::B),
        (KeyCode::Enter, Button::Start),
        (KeyCode::RightShift, Button::Select),
        (KeyCode::Tab, Button::Select),
    ];
    let mut raw = 0;
    for (key, button) in KEYS {
        if is_key_down(key) {
            raw |= button as u8;
        }
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_lasts_but_pressed_is_one_tick() {
        let mut pad = Controller::new();
        pad.update(Button::A as u8);
        assert!(pad.held(Button::A));
        assert!(pad.pressed(Button::A));

        pad.update(Button::A as u8); // still down on the next tick
        assert!(pad.held(Button::A));
        assert!(!pad.pressed(Button::A));

        pad.update(0); // let go
        assert!(!pad.held(Button::A));
    }

    #[test]
    fn buttons_are_independent() {
        let mut pad = Controller::new();
        pad.update(Button::Right as u8 | Button::B as u8);
        assert!(pad.held(Button::Right));
        assert!(pad.held(Button::B));
        assert!(!pad.held(Button::Left));
        assert!(!pad.held(Button::A));
    }

    #[test]
    fn opposite_directions_cancel() {
        let mut pad = Controller::new();
        pad.update(Button::Left as u8 | Button::Right as u8 | Button::Up as u8);
        assert!(!pad.held(Button::Left));
        assert!(!pad.held(Button::Right));
        assert!(pad.held(Button::Up)); // unaffected
    }
}
