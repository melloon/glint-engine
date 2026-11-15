//! The camera: decides which part of a wide level is on screen.
//!
//! It holds one number, the scroll position, which is the level pixel shown
//! at the left edge of the screen. The background draws from that offset and
//! sprites subtract it from their level positions.

use crate::video::SCREEN_W;

/// The camera only moves when the thing it follows leaves this band of the
/// screen. Small back-and-forth steps inside the band do not scroll at all,
/// which keeps the picture steady.
const BAND_LEFT: i32 = 96;
const BAND_RIGHT: i32 = 160;

pub struct Camera {
    pub x: i32,
}

impl Camera {
    pub fn new() -> Self {
        Self { x: 0 }
    }

    /// Scroll just enough to bring `target_x` (a level position) back inside
    /// the band, without ever showing anything beyond the ends of the level.
    pub fn follow(&mut self, target_x: i32, level_width: i32) {
        let on_screen = target_x - self.x;
        if on_screen > BAND_RIGHT {
            self.x = target_x - BAND_RIGHT;
        } else if on_screen < BAND_LEFT {
            self.x = target_x - BAND_LEFT;
        }
        self.x = self.x.clamp(0, level_width - SCREEN_W as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEVEL: i32 = 768; // three screens

    #[test]
    fn stays_put_while_the_target_is_inside_the_band() {
        let mut camera = Camera { x: 100 };
        camera.follow(100 + 96, LEVEL);
        camera.follow(100 + 160, LEVEL);
        assert_eq!(camera.x, 100);
    }

    #[test]
    fn scrolls_just_enough_when_the_target_leaves_the_band() {
        let mut camera = Camera { x: 100 };
        camera.follow(100 + 165, LEVEL); // 5 px past the right of the band
        assert_eq!(camera.x, 105);
        camera.follow(105 + 90, LEVEL); // 6 px past the left of the band
        assert_eq!(camera.x, 99);
    }

    #[test]
    fn never_shows_past_the_ends_of_the_level() {
        let mut camera = Camera::new();
        camera.follow(10, LEVEL);
        assert_eq!(camera.x, 0);
        camera.follow(760, LEVEL);
        assert_eq!(camera.x, LEVEL - 256);
    }
}
