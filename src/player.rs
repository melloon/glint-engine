//! The player character: turns controller input into movement.
//!
//! This is game code, not engine code. The engine's `Body` knows how to move
//! and collide; everything about how the hero *feels* is decided here, mostly
//! by the handful of numbers below.

use crate::animation::{Animation, Animator};
use crate::input::{Button, Controller};
use crate::physics::{Body, Terrain};

// The hero's poses. `art.rs` stores the drawings in this order.
pub const FRAME_STAND: u8 = 0;
pub const FRAME_STRIDE: u8 = 1;
pub const FRAME_PASS: u8 = 2;
pub const FRAME_AIR: u8 = 3;

// Walking and running share the same two drawings; running just swaps them
// faster. These are statics, not consts, so each has one fixed identity the
// animator can compare against.
static STAND: Animation = Animation { frames: &[FRAME_STAND], ticks_per_frame: 1 };
static WALK: Animation = Animation { frames: &[FRAME_STRIDE, FRAME_PASS], ticks_per_frame: 8 };
static RUN: Animation = Animation { frames: &[FRAME_STRIDE, FRAME_PASS], ticks_per_frame: 5 };
static AIR: Animation = Animation { frames: &[FRAME_AIR], ticks_per_frame: 1 };

// All speeds are in sub-pixels per tick: 0x100 (256) is one pixel per tick.
const WALK_SPEED: i32 = 0x180; // 1.5 px/tick
const RUN_SPEED: i32 = 0x280; // 2.5 px/tick, while B is held
const ACCEL: i32 = 0x18; // speeding up
const SKID: i32 = 0x60; // braking while pushing the other way
const FRICTION: i32 = 0x40; // slowing down with no direction held
const RUN_FRICTION: i32 = 0x28; // ...while still faster than a walk: a longer slide

const JUMP_SPEED: i32 = 0x480; // 4.5 px/tick upward at take-off
const GRAVITY: i32 = 0x70; // normal pull
const GRAVITY_HELD: i32 = 0x28; // weaker pull while rising with A held
const MAX_FALL: i32 = 0x480;
const BOUNCE_SPEED: i32 = 0x300; // the hop after stomping an enemy

// The collision box is narrower than the 16x16 drawing, so the hero's arms
// can overhang a ledge or brush a wall without it counting.
const WIDTH: i32 = 10;
const HEIGHT: i32 = 16;
const SPRITE_OFFSET_X: i32 = 3;

pub struct Player {
    pub body: Body,
    pub facing_left: bool,
    /// True on the tick a jump starts, so the game can play a sound.
    pub jumped: bool,
    animator: Animator,
}

impl Player {
    /// Create the player with its 16x16 drawing at pixel (x, y).
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            body: Body::new(x + SPRITE_OFFSET_X, y, WIDTH, HEIGHT),
            facing_left: false,
            jumped: false,
            animator: Animator::new(&AIR),
        }
    }

    /// Which pose to draw: one of the FRAME_ numbers.
    pub fn frame(&self) -> u8 {
        self.animator.frame()
    }

    /// Where to draw the 16x16 sprite.
    pub fn sprite_x(&self) -> i32 {
        self.body.left() - SPRITE_OFFSET_X
    }
    pub fn sprite_y(&self) -> i32 {
        self.body.top()
    }

    /// Spring off something just stomped: a small hop, or a full jump if A
    /// is being held.
    pub fn bounce(&mut self, high: bool) {
        self.body.vy = -(if high { JUMP_SPEED } else { BOUNCE_SPEED });
    }

    /// One tick: read the controller, adjust speed, then move and collide.
    pub fn update(&mut self, pad: &Controller, terrain: &Terrain) {
        let body = &mut self.body;

        // Running: the d-pad sets the speed we are heading for, and the
        // actual speed chases it a little each tick. That gradual change is
        // what makes movement feel weighty instead of robotic.
        let top_speed = if pad.held(Button::B) { RUN_SPEED } else { WALK_SPEED };
        let target = if pad.held(Button::Left) {
            self.facing_left = true;
            -top_speed
        } else if pad.held(Button::Right) {
            self.facing_left = false;
            top_speed
        } else {
            0
        };
        let rate = if target == 0 {
            // Coming out of a run carries more momentum than a walk does.
            if body.vx.abs() > WALK_SPEED { RUN_FRICTION } else { FRICTION }
        } else if (target > 0) != (body.vx > 0) && body.vx != 0 {
            SKID
        } else {
            ACCEL
        };
        body.vx = approach(body.vx, target, rate);

        // Jumping: only from the ground, and only on the tick A goes down.
        self.jumped = pad.pressed(Button::A) && body.on_ground;
        if self.jumped {
            body.vy = -JUMP_SPEED;
        }

        // Gravity is weaker while rising with A still held. Letting go early
        // brings back full gravity, so a tap is a hop and a hold is a leap.
        let gravity = if body.vy < 0 && pad.held(Button::A) { GRAVITY_HELD } else { GRAVITY };
        body.vy = (body.vy + gravity).min(MAX_FALL);

        body.step(terrain);

        // Pick the animation from what the body is doing. "Moving" includes
        // holding a direction, so pushing against a wall still walks in place
        // rather than flickering between poses.
        let animation = if !body.on_ground {
            &AIR
        } else if target == 0 && body.vx == 0 {
            &STAND
        } else if body.vx.abs() > WALK_SPEED {
            &RUN
        } else {
            &WALK
        };
        self.animator.update();
        self.animator.play(animation);
    }
}

/// Move `value` toward `target` by at most `step`, without overshooting.
fn approach(value: i32, target: i32, step: i32) -> i32 {
    if value < target {
        (value + step).min(target)
    } else {
        (value - step).max(target)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::background::Background;

    const NONE: u8 = 0;
    const A: u8 = Button::A as u8;
    const B: u8 = Button::B as u8;
    const RIGHT: u8 = Button::Right as u8;
    const LEFT: u8 = Button::Left as u8;

    /// A flat floor across the whole screen, top surface at y=160.
    struct World {
        bg: Background,
        solid: [bool; 256],
        pad: Controller,
        player: Player,
    }

    impl World {
        fn new() -> Self {
            let mut bg = Background::with_width(32);
            bg.fill(0, 20, 32, 10, 1, 0);
            let mut solid = [false; 256];
            solid[1] = true;
            let mut world = Self { bg, solid, pad: Controller::new(), player: Player::new(16, 144) };
            world.hold(NONE, 2); // settle onto the floor
            world
        }

        /// Hold the given buttons for a number of ticks; returns the highest
        /// point (smallest y) the player reached during them.
        fn hold(&mut self, raw: u8, ticks: u32) -> i32 {
            let mut highest = self.player.body.top();
            for _ in 0..ticks {
                self.pad.update(raw);
                let terrain = Terrain { background: &self.bg, solid: &self.solid };
                self.player.update(&self.pad, &terrain);
                highest = highest.min(self.player.body.top());
            }
            highest
        }
    }

    #[test]
    fn stands_still_on_the_floor() {
        let mut world = World::new();
        world.hold(NONE, 60);
        assert!(world.player.body.on_ground);
        assert_eq!((world.player.sprite_x(), world.player.sprite_y()), (16, 144));
    }

    #[test]
    fn holding_jump_goes_higher_than_tapping() {
        let mut tap = World::new();
        let tap_top = tap.hold(A, 1).min(tap.hold(NONE, 90));
        let mut held = World::new();
        let held_top = held.hold(A, 90);

        let (tap_height, held_height) = (144 - tap_top, 144 - held_top);
        assert!(tap_height >= 16, "a tap should clear two tiles, got {tap_height}");
        assert!(held_height >= 2 * tap_height, "hold {held_height} vs tap {tap_height}");
        assert!(tap.player.body.on_ground && held.player.body.on_ground); // both landed
    }

    #[test]
    fn cannot_jump_again_in_mid_air() {
        let mut world = World::new();
        world.hold(A, 1);
        world.hold(NONE, 10); // now well off the ground
        assert!(!world.player.body.on_ground);
        let before = world.player.body.vy;
        world.hold(A, 1);
        assert!(world.player.body.vy > before); // still slowing, no new jump
    }

    #[test]
    fn speed_builds_up_to_a_cap_and_b_raises_it() {
        let mut world = World::new();
        world.hold(RIGHT, 5);
        assert!(world.player.body.vx < WALK_SPEED); // not instant
        world.hold(RIGHT, 40);
        assert_eq!(world.player.body.vx, WALK_SPEED);
        world.hold(RIGHT | B, 40);
        assert_eq!(world.player.body.vx, RUN_SPEED);
    }

    #[test]
    fn pose_follows_what_the_hero_is_doing() {
        let mut world = World::new();
        world.hold(NONE, 5);
        assert_eq!(world.player.frame(), FRAME_STAND);

        // Walking alternates the two leg drawings, 8 ticks each.
        let mut seen = Vec::new();
        for _ in 0..4 {
            world.hold(RIGHT, 8);
            seen.push(world.player.frame());
        }
        assert_eq!(seen, [FRAME_STRIDE, FRAME_PASS, FRAME_STRIDE, FRAME_PASS]);

        world.hold(A, 3);
        assert_eq!(world.player.frame(), FRAME_AIR);
        world.hold(NONE, 90); // land and stop
        assert_eq!(world.player.frame(), FRAME_STAND);
    }

    #[test]
    fn a_run_slides_further_than_a_walk_before_stopping() {
        let slide = |buttons: u8| {
            let mut world = World::new();
            world.hold(buttons, 60);
            let start = world.player.body.x;
            world.hold(NONE, 60);
            assert_eq!(world.player.body.vx, 0);
            (world.player.body.x - start) / crate::physics::SUBPIXEL
        };
        let (walk, run) = (slide(RIGHT), slide(RIGHT | B));
        assert!((3..=6).contains(&walk), "walk slide was {walk} px");
        assert!((14..=20).contains(&run), "run slide was {run} px");
    }

    #[test]
    fn letting_go_slides_to_a_stop_and_left_turns_around() {
        let mut world = World::new();
        world.hold(RIGHT, 40);
        world.hold(NONE, 40);
        assert_eq!(world.player.body.vx, 0);
        assert!(!world.player.facing_left);
        world.hold(LEFT, 40);
        assert_eq!(world.player.body.vx, -WALK_SPEED);
        assert!(world.player.facing_left);
    }
}
