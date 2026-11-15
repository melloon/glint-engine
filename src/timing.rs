//! Fixed-timestep clock.
//!
//! 8-bit consoles ran game logic exactly once per video frame (60 times a
//! second on NTSC machines). We reproduce that: the game always advances in
//! identical 1/60 s ticks, no matter how fast or slow the host machine draws.

/// Logic updates per second.
pub const TICK_RATE: f32 = 60.0;
/// Length of one logic update, in seconds.
pub const TICK: f32 = 1.0 / TICK_RATE;

/// Longest real frame we will try to catch up on. Without this cap, one long
/// stall (window drag, breakpoint) would make the game run hundreds of ticks
/// in a burst.
const MAX_FRAME_TIME: f32 = 0.25;

pub struct FixedTimestep {
    /// Real time that has passed but has not yet been turned into ticks.
    accumulator: f32,
}

impl FixedTimestep {
    pub fn new() -> Self {
        Self { accumulator: 0.0 }
    }

    /// Add the real time the last frame took and return how many logic ticks
    /// the game should run to catch up.
    pub fn advance(&mut self, frame_time: f32) -> u32 {
        self.accumulator += frame_time.min(MAX_FRAME_TIME);
        let mut ticks = 0;
        while self.accumulator >= TICK {
            self.accumulator -= TICK;
            ticks += 1;
        }
        ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_frame_runs_several_ticks() {
        let mut clock = FixedTimestep::new();
        assert_eq!(clock.advance(0.051), 3);
    }

    #[test]
    fn leftover_time_carries_to_next_frame() {
        let mut clock = FixedTimestep::new();
        assert_eq!(clock.advance(0.010), 0);
        assert_eq!(clock.advance(0.010), 1);
    }

    #[test]
    fn long_stall_is_capped() {
        let mut clock = FixedTimestep::new();
        assert!(clock.advance(10.0) <= 15);
    }
}
