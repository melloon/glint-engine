//! Frame animation: showing a sequence of drawings one after another.
//!
//! An animation is only a list of frame numbers and a speed. What a frame
//! number means (which tiles to draw) is up to the game. Time is counted in
//! ticks, so animations run at the same speed on every machine.

pub struct Animation {
    /// Frame numbers to show, in order, repeating.
    pub frames: &'static [u8],
    /// How many ticks each frame stays on screen.
    pub ticks_per_frame: u32,
}

/// Plays one animation at a time and remembers how far through it is.
pub struct Animator {
    animation: &'static Animation,
    ticks: u32,
}

impl Animator {
    pub fn new(animation: &'static Animation) -> Self {
        Self { animation, ticks: 0 }
    }

    /// Switch to an animation. Asking for the one already playing does
    /// nothing, so the game can call this every tick without restarting it.
    pub fn play(&mut self, animation: &'static Animation) {
        if !std::ptr::eq(self.animation, animation) {
            self.animation = animation;
            self.ticks = 0;
        }
    }

    /// Advance by one tick.
    pub fn update(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
    }

    /// The frame number to draw right now.
    pub fn frame(&self) -> u8 {
        let animation = self.animation;
        let index = (self.ticks / animation.ticks_per_frame) as usize % animation.frames.len();
        animation.frames[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static BLINK: Animation = Animation { frames: &[7, 9], ticks_per_frame: 3 };
    static STILL: Animation = Animation { frames: &[4], ticks_per_frame: 1 };

    fn frames(animator: &mut Animator, ticks: u32) -> Vec<u8> {
        (0..ticks)
            .map(|_| {
                let frame = animator.frame();
                animator.update();
                frame
            })
            .collect()
    }

    #[test]
    fn each_frame_lasts_its_ticks_then_the_cycle_repeats() {
        let mut animator = Animator::new(&BLINK);
        assert_eq!(frames(&mut animator, 8), [7, 7, 7, 9, 9, 9, 7, 7]);
    }

    #[test]
    fn playing_the_current_animation_again_does_not_restart_it() {
        let mut animator = Animator::new(&BLINK);
        frames(&mut animator, 4); // now on the second frame
        animator.play(&BLINK);
        assert_eq!(animator.frame(), 9);
    }

    #[test]
    fn switching_animation_starts_from_its_first_frame() {
        let mut animator = Animator::new(&BLINK);
        frames(&mut animator, 4);
        animator.play(&STILL);
        assert_eq!(animator.frame(), 4);
        animator.play(&BLINK);
        assert_eq!(animator.frame(), 7);
    }
}
