//! Physical gamepads (DualSense, Xbox pads and so on), read through gilrs.
//!
//! A gamepad is just a second way of filling in the same controller byte the
//! keyboard produces, so nothing in the game changes. This is the only file
//! that touches gilrs.

use gilrs::{Axis, Button as PadButton, EventType, Gilrs};

use crate::input::Button;

/// How far the left stick must be pushed before it counts as a d-pad press.
const STICK_THRESHOLD: f32 = 0.5;

/// Which gamepad button stands in for which controller button. gilrs names
/// face buttons by position: South is Cross on a DualSense, West is Square.
const BUTTONS: [(PadButton, Button); 8] = [
    (PadButton::South, Button::A),
    (PadButton::West, Button::B),
    (PadButton::Start, Button::Start), // Options
    (PadButton::Select, Button::Select), // Create
    (PadButton::DPadUp, Button::Up),
    (PadButton::DPadDown, Button::Down),
    (PadButton::DPadLeft, Button::Left),
    (PadButton::DPadRight, Button::Right),
];

pub struct Gamepads {
    /// `None` if gamepad support could not start; the keyboard still works.
    gilrs: Option<Gilrs>,
}

impl Gamepads {
    pub fn new() -> Self {
        let gilrs = match Gilrs::new() {
            Ok(gilrs) => Some(gilrs),
            Err(error) => {
                eprintln!("gamepad support unavailable: {error}");
                None
            }
        };
        Self { gilrs }
    }

    /// Read every connected gamepad into one controller byte.
    pub fn read(&mut self) -> u8 {
        let Some(gilrs) = &mut self.gilrs else {
            return 0;
        };

        // gilrs only refreshes its picture of each pad as events are taken
        // off its queue, so drain the queue before looking.
        while let Some(event) = gilrs.next_event() {
            match event.event {
                EventType::Connected => {
                    eprintln!("gamepad connected: {}", gilrs.gamepad(event.id).name());
                }
                EventType::Disconnected => eprintln!("gamepad disconnected"),
                _ => {}
            }
        }

        let mut raw = 0;
        for (_id, pad) in gilrs.gamepads() {
            for (pad_button, button) in BUTTONS {
                if pad.is_pressed(pad_button) {
                    raw |= button as u8;
                }
            }
            raw |= stick_to_dpad(pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
        }
        raw
    }
}

/// Treat an analog stick as a d-pad. The console only knows on and off, so
/// the stick counts as pressed once it is pushed past the threshold.
/// gilrs reports right and up as positive.
fn stick_to_dpad(x: f32, y: f32) -> u8 {
    let mut raw = 0;
    if x > STICK_THRESHOLD {
        raw |= Button::Right as u8;
    }
    if x < -STICK_THRESHOLD {
        raw |= Button::Left as u8;
    }
    if y > STICK_THRESHOLD {
        raw |= Button::Up as u8;
    }
    if y < -STICK_THRESHOLD {
        raw |= Button::Down as u8;
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_at_rest_or_barely_moved_presses_nothing() {
        assert_eq!(stick_to_dpad(0.0, 0.0), 0);
        assert_eq!(stick_to_dpad(0.3, -0.3), 0);
    }

    #[test]
    fn pushed_stick_presses_the_matching_directions() {
        assert_eq!(stick_to_dpad(1.0, 0.0), Button::Right as u8);
        assert_eq!(stick_to_dpad(-1.0, 0.0), Button::Left as u8);
        assert_eq!(stick_to_dpad(0.0, 1.0), Button::Up as u8);
        assert_eq!(stick_to_dpad(0.0, -1.0), Button::Down as u8);
        assert_eq!(stick_to_dpad(-0.8, 0.8), Button::Left as u8 | Button::Up as u8);
    }
}
