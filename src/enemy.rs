//! A simple enemy: it walks back and forth, turning around at walls and at
//! the edges of whatever it is standing on.
//!
//! Like the player, this is game code. It reuses the engine's `Body` for
//! movement and collision and the `Animator` for its walk cycle.

use crate::animation::{Animation, Animator};
use crate::physics::{Body, Terrain};
use crate::video::SCREEN_H;

const SPEED: i32 = 0x80; // half a pixel per tick
const GRAVITY: i32 = 0x70;
const MAX_FALL: i32 = 0x480;

// The collision box is a little smaller than the 16x16 drawing, which makes
// near misses go the player's way.
const WIDTH: i32 = 12;
const HEIGHT: i32 = 12;
const SPRITE_OFFSET_X: i32 = 2;
const SPRITE_OFFSET_Y: i32 = 4;

/// How long the flattened enemy stays on screen after being stomped.
const SQUASH_TICKS: u32 = 20;

static CRAWL: Animation = Animation { frames: &[0, 1], ticks_per_frame: 10 };

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EnemyState {
    Walking,
    /// Stomped, with the number of ticks since. Harmless, about to vanish.
    Squashed(u32),
    Gone,
}

pub struct Enemy {
    pub body: Body,
    pub state: EnemyState,
    /// -1 when walking left, 1 when walking right.
    direction: i32,
    animator: Animator,
}

impl Enemy {
    /// Create an enemy with its 16x16 drawing at pixel (x, y), walking left.
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            body: Body::new(x + SPRITE_OFFSET_X, y + SPRITE_OFFSET_Y, WIDTH, HEIGHT),
            state: EnemyState::Walking,
            direction: -1,
            animator: Animator::new(&CRAWL),
        }
    }

    /// Where to draw the 16x16 sprite.
    pub fn sprite_x(&self) -> i32 {
        self.body.left() - SPRITE_OFFSET_X
    }
    pub fn sprite_y(&self) -> i32 {
        self.body.top() - SPRITE_OFFSET_Y
    }

    /// Which of the two walking poses to draw: 0 or 1.
    pub fn frame(&self) -> u8 {
        self.animator.frame()
    }

    /// Can it still hurt the hero (or be stomped)?
    pub fn is_dangerous(&self) -> bool {
        self.state == EnemyState::Walking
    }

    pub fn squash(&mut self) {
        self.state = EnemyState::Squashed(0);
    }

    /// One tick of patrolling.
    pub fn update(&mut self, terrain: &Terrain) {
        match self.state {
            EnemyState::Walking => {}
            EnemyState::Squashed(ticks) => {
                self.state = if ticks >= SQUASH_TICKS {
                    EnemyState::Gone
                } else {
                    EnemyState::Squashed(ticks + 1)
                };
                return;
            }
            EnemyState::Gone => return,
        }

        let body = &mut self.body;

        // Ledges: look at the spot just past the leading foot, one pixel
        // below floor level. If there is nothing solid there, the next step
        // would be off the edge, so turn around instead.
        if body.on_ground {
            let ahead = if self.direction > 0 { body.left() + body.w } else { body.left() - 1 };
            if !terrain.solid_at(ahead, body.top() + body.h) {
                self.direction = -self.direction;
            }
        }

        body.vx = self.direction * SPEED;
        body.vy = (body.vy + GRAVITY).min(MAX_FALL);
        body.step(terrain);

        // Walls: the engine zeroes the speed of a body it stops.
        if body.vx == 0 {
            self.direction = -self.direction;
        }
        // Anything that does fall off the bottom of the level is removed.
        if body.top() > SCREEN_H as i32 {
            self.state = EnemyState::Gone;
        }

        self.animator.update();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::background::Background;

    /// Floor from y=160 down, plus a floating platform at x=80..128, y=120.
    fn world() -> (Background, [bool; 256]) {
        let mut bg = Background::with_width(32);
        bg.fill(0, 20, 32, 10, 1, 0);
        bg.fill(10, 15, 6, 1, 1, 0);
        let mut solid = [false; 256];
        solid[1] = true;
        (bg, solid)
    }

    #[test]
    fn turns_around_at_a_wall() {
        let (bg, solid) = world();
        let terrain = Terrain { background: &bg, solid: &solid };
        let mut enemy = Enemy::new(20, 140); // walks left, toward the edge of the map
        let mut leftmost = i32::MAX;
        for _ in 0..200 {
            enemy.update(&terrain);
            leftmost = leftmost.min(enemy.body.left());
        }
        assert_eq!(leftmost, 0); // reached the wall, went no further
        assert!(enemy.body.left() > 20); // and is now well on its way back
    }

    #[test]
    fn turns_around_at_ledges_and_stays_on_its_platform() {
        let (bg, solid) = world();
        let terrain = Terrain { background: &bg, solid: &solid };
        let mut enemy = Enemy::new(100, 100);
        for _ in 0..10 {
            enemy.update(&terrain); // settle onto the platform
        }
        let (mut leftmost, mut rightmost) = (i32::MAX, i32::MIN);
        for _ in 0..600 {
            enemy.update(&terrain);
            assert!(enemy.body.on_ground);
            leftmost = leftmost.min(enemy.body.left());
            rightmost = rightmost.max(enemy.body.left() + WIDTH);
        }
        assert_eq!((leftmost, rightmost), (80, 128)); // walked the full width, both ways
    }

    #[test]
    fn squashed_enemy_is_harmless_then_disappears() {
        let (bg, solid) = world();
        let terrain = Terrain { background: &bg, solid: &solid };
        let mut enemy = Enemy::new(100, 140);
        enemy.update(&terrain);
        assert!(enemy.is_dangerous());
        enemy.squash();
        let x = enemy.body.left();
        enemy.update(&terrain);
        assert!(!enemy.is_dangerous());
        assert_eq!(enemy.body.left(), x); // it has stopped walking
        for _ in 0..SQUASH_TICKS {
            enemy.update(&terrain);
        }
        assert_eq!(enemy.state, EnemyState::Gone);
    }
}
