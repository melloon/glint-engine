//! The demo game: one scrolling level with gems to collect, pits to jump and
//! a flag to reach. This is game-side code built on the engine; the drawings
//! and palettes it uses live in `art.rs`.

use crate::art::*;
use crate::background::Background;
use crate::camera::Camera;
use crate::enemy::{Enemy, EnemyState};
use crate::input::{Button, Controller};
use crate::physics::Terrain;
use crate::player::Player;
use crate::sfx::Sfx;
use crate::sprites::{Sprite, SpriteList};
use crate::tiles::PatternTable;
use crate::video::{Framebuffer, Palettes, SCREEN_H, SCREEN_W};

/// The brick highlight colour steps through these, one step every 8 ticks.
const GLINT: [u8; 4] = [0x25, 0x35, 0x20, 0x35];

/// The level is three screens wide.
const LEVEL_COLS: usize = 96;

/// Where the hero appears: in the air above the grass, so it drops in.
const START: (i32, i32) = (16, 160);

/// The goal flag stands in this tile column; the flag is at FLAG_ROW and the
/// pole runs from there down to the grass.
const FLAG_COL: usize = 94;
const FLAG_ROW: usize = 21;
const POLE_ROWS: usize = 5;

/// After the flag is touched the sky changes colour for three seconds, then
/// the level starts again. The colours are pastel and change slowly on
/// purpose: a fast full-screen flash is unpleasant and can be harmful.
const CELEBRATION_TICKS: u32 = 180;
const CELEBRATION_SKY: [u8; 3] = [0x3C, 0x37, 0x3A];

/// A touch counts as a stomp when the hero's feet are no more than this many
/// pixels below the top of the enemy.
const STOMP_DEPTH: i32 = 8;

struct Gem {
    x: i32,
    y: i32,
    collected: bool,
}

pub struct Demo {
    bg_tiles: PatternTable,
    sprite_tiles: PatternTable,
    palettes: Palettes,
    background: Background,
    sprites: SpriteList,
    /// Game time: stops while paused.
    ticks: u32,
    /// Counts every tick, paused or not, to keep sprite flicker rotating.
    flicker: u32,
    paused: bool,
    /// Which background tile numbers the hero cannot pass through.
    solid: [bool; 256],
    player: Player,
    camera: Camera,
    hero_color: usize,
    gems: Vec<Gem>,
    enemies: Vec<Enemy>,
    /// Sounds requested since the engine last collected them.
    sounds: Vec<Sfx>,
    /// `Some(ticks so far)` once the flag has been reached.
    celebration: Option<u32>,
}

impl Demo {
    pub fn new() -> Self {
        let mut demo = Self {
            bg_tiles: background_tiles(),
            sprite_tiles: sprite_tiles(),
            palettes: palettes(),
            background: build_level(),
            sprites: SpriteList::new(),
            ticks: 0,
            flicker: 0,
            paused: false,
            solid: solid_tiles(),
            player: Player::new(START.0, START.1),
            camera: Camera::new(),
            hero_color: 0,
            gems: place_gems(),
            enemies: place_enemies(),
            sounds: Vec::new(),
            celebration: None,
        };
        demo.build_sprites();
        demo
    }

    /// Advance by exactly one tick (1/60 s).
    pub fn update(&mut self, pad: &Controller) {
        // Start toggles pause. This must be `pressed`, not `held`: a held
        // button would flip the pause 60 times a second.
        if pad.pressed(Button::Start) {
            self.paused = !self.paused;
            self.sounds.push(Sfx::Pause);
        }

        self.flicker += 1;
        if !self.paused {
            self.ticks += 1;
            // Palette cycling: no pixels are redrawn by the game. Changing one
            // palette entry recolours every brick on screen at once.
            let step = (self.ticks / 8) as usize % GLINT.len();
            self.palettes.background[PAL_BRICK as usize][2] = GLINT[step];

            self.play(pad);
        }

        self.build_sprites();
    }

    /// Hand over the sounds requested since the last call. The game never
    /// plays audio itself: it queues names, and the engine's main loop plays
    /// them. That keeps the game runnable (and testable) with no speakers.
    pub fn take_sounds(&mut self) -> Vec<Sfx> {
        std::mem::take(&mut self.sounds)
    }

    /// One tick of actual gameplay.
    fn play(&mut self, pad: &Controller) {
        // Once the flag is reached the hero stops listening to the
        // controller: hand it one with nothing pressed.
        let hands_off = Controller::new();
        let pad = if self.celebration.is_some() { &hands_off } else { pad };

        let terrain = Terrain {
            background: &self.background,
            solid: &self.solid,
        };
        self.player.update(pad, &terrain);
        if self.player.jumped {
            self.sounds.push(Sfx::Jump);
        }

        // Fell down a pit: back to the start. Collected gems stay collected.
        if self.player.body.top() > SCREEN_H as i32 {
            self.player = Player::new(START.0, START.1);
            self.sounds.push(Sfx::Hurt);
        }

        // Enemies walk their patrols, then each is checked against the hero.
        for enemy in &mut self.enemies {
            enemy.update(&terrain);
            let (hero, foe) = (&self.player.body, &enemy.body);
            if !enemy.is_dangerous() || !hero.overlaps(foe.left(), foe.top(), foe.w, foe.h) {
                continue;
            }
            // Coming down onto its head squashes it and bounces the hero.
            // Any other touch sends the hero back to the start.
            let feet = hero.top() + hero.h;
            if !hero.on_ground && hero.vy >= 0 && feet <= foe.top() + STOMP_DEPTH {
                enemy.squash();
                self.player.bounce(pad.held(Button::A));
                self.sounds.push(Sfx::Stomp);
            } else {
                self.player = Player::new(START.0, START.1);
                self.sounds.push(Sfx::Hurt);
            }
        }

        // Follow the middle of the hero. This runs after the hero has moved,
        // so the camera never lags a tick behind.
        self.camera.follow(self.player.sprite_x() + 8, self.background.width());

        // Pickups: a gem is collected the moment the hero's box overlaps it.
        for gem in &mut self.gems {
            if !gem.collected && self.player.body.overlaps(gem.x, gem.y, 8, 8) {
                gem.collected = true;
                self.sounds.push(Sfx::Gem);
            }
        }

        match self.celebration {
            None => {
                let (x, y) = ((FLAG_COL * 8) as i32, (FLAG_ROW * 8) as i32);
                if self.player.body.overlaps(x, y, 8, (POLE_ROWS * 8) as i32) {
                    self.celebration = Some(0);
                    self.sounds.push(Sfx::Flag);
                }
            }
            Some(ticks) if ticks >= CELEBRATION_TICKS => self.restart(),
            Some(ticks) => {
                // The whole sky is one palette entry, so one write recolours it.
                let step = (ticks / 20) as usize % CELEBRATION_SKY.len();
                self.palettes.backdrop = CELEBRATION_SKY[step];
                self.celebration = Some(ticks + 1);
            }
        }

        if pad.pressed(Button::Select) {
            self.hero_color = (self.hero_color + 1) % HERO_COLORS.len();
            self.palettes.sprite[PAL_HERO as usize][0] = HERO_COLORS[self.hero_color];
        }
    }

    /// Put the level back the way it started.
    fn restart(&mut self) {
        self.player = Player::new(START.0, START.1);
        self.camera = Camera::new();
        self.gems = place_gems();
        self.enemies = place_enemies();
        self.celebration = None;
        self.palettes.backdrop = SKY;
    }

    /// Refill the sprite list from scratch, as 8-bit games did every frame.
    fn build_sprites(&mut self) {
        self.sprites.clear();

        // Everything in the game lives at level positions. Sprites are
        // drawn at screen positions, so subtract the camera on the way out.
        let camera_x = self.camera.x;

        // The gem counter. The background scrolls as one piece, so anything
        // that must stay put on screen is drawn with sprites instead. It goes
        // in first, which puts it on top and means it is never dropped.
        let collected = self.gems.iter().filter(|gem| gem.collected).count();
        for (i, tile) in hud_tiles(collected, self.gems.len()).into_iter().enumerate() {
            self.sprites.push(Sprite {
                x: 8 + i as i32 * 8 + if i > 0 { 4 } else { 0 }, // small gap after the icon
                y: 8,
                tile,
                palette: PAL_GEM,
                flip_x: false,
                flip_y: false,
            });
        }

        // The hero comes next, so it also keeps priority over the gems.
        push_hero(
            &mut self.sprites,
            self.player.sprite_x() - camera_x,
            self.player.sprite_y(),
            self.player.facing_left,
            self.player.frame(),
        );

        for enemy in &self.enemies {
            push_enemy(&mut self.sprites, enemy, camera_x);
        }

        // Only gems that are still there and in view get a sprite, so the
        // rest do not use up sprite slots or count toward the 8 per line.
        let visible: Vec<&Gem> = self
            .gems
            .iter()
            .filter(|gem| !gem.collected && gem.x > camera_x - 8 && gem.x < camera_x + SCREEN_W as i32)
            .collect();

        // The first screen has ten gems on the same lines, but only 8 sprites
        // fit on a line. Starting from a different gem each tick means a
        // different pair is dropped each time: every gem flickers instead of
        // two vanishing.
        for i in 0..visible.len() {
            let gem = visible[(self.flicker as usize + i) % visible.len()];
            self.sprites.push(Sprite {
                x: gem.x - camera_x,
                y: gem.y,
                tile: GEM,
                palette: PAL_GEM,
                flip_x: false,
                flip_y: false,
            });
        }
    }

    pub fn draw(&self, fb: &mut Framebuffer) {
        self.background.draw(fb, &self.bg_tiles, &self.palettes, self.camera.x);
        self.sprites.draw(fb, &self.sprite_tiles, &self.palettes);
    }
}

/// The hero is 16x16: a "metasprite" made of four 8x8 sprites. The top two
/// are the head, shared by every pose; the bottom two depend on the pose.
fn push_hero(sprites: &mut SpriteList, x: i32, y: i32, facing_left: bool, frame: u8) {
    for row in 0..2 {
        for col in 0..2 {
            // Mirroring the whole figure means each tile is flipped *and* the
            // left and right halves swap places.
            let source_col = if facing_left { 1 - col } else { col } as u8;
            let tile = if row == 0 {
                HERO_HEAD + source_col
            } else {
                HERO_LEGS + frame * 2 + source_col
            };
            sprites.push(Sprite {
                x: x + col * 8,
                y: y + row * 8,
                tile,
                palette: PAL_HERO,
                flip_x: facing_left,
                flip_y: false,
            });
        }
    }
}

/// An enemy is 16x16 and symmetrical: each row is one tile plus its mirror image.
fn push_enemy(sprites: &mut SpriteList, enemy: &Enemy, camera_x: i32) {
    let (x, y) = (enemy.sprite_x() - camera_x, enemy.sprite_y());
    if enemy.state == EnemyState::Gone || x <= -16 || x >= SCREEN_W as i32 {
        return;
    }
    let mut row = |tile: u8, y: i32| {
        for (dx, flip_x) in [(0, false), (8, true)] {
            sprites.push(Sprite {
                x: x + dx,
                y,
                tile,
                palette: PAL_ENEMY,
                flip_x,
                flip_y: false,
            });
        }
    };
    if enemy.state == EnemyState::Walking {
        row(ENEMY_TOP, y);
        row(ENEMY_FEET + enemy.frame(), y + 8);
    } else {
        // Squashed: just the top half, sitting on the floor. It reads as
        // flattened without needing a new drawing.
        row(ENEMY_TOP, y + 8);
    }
}

/// The sprite tiles that spell out the counter: a gem icon, then
/// "collected/total" as two digits each, such as 07/18.
fn hud_tiles(collected: usize, total: usize) -> [u8; 6] {
    let digit = |n: usize| DIGIT + (n % 10) as u8;
    let (collected, total) = (collected.min(99), total.min(99));
    [
        GEM,
        digit(collected / 10),
        digit(collected),
        SLASH,
        digit(total / 10),
        digit(total),
    ]
}

/// The tile property table: ground and blocks are solid; clouds and the
/// flag are not.
fn solid_tiles() -> [bool; 256] {
    let mut solid = [false; 256];
    for tile in [BLOCK, GRASS, DIRT] {
        solid[tile as usize] = true;
    }
    solid
}

fn build_level() -> Background {
    let mut bg = Background::with_width(LEVEL_COLS);

    // Ground: one row of grass on top of three rows of dirt, end to end,
    // with two pits cut out of it.
    bg.fill(0, 26, LEVEL_COLS, 1, GRASS, PAL_GROUND);
    bg.fill(0, 27, LEVEL_COLS, 3, DIRT, PAL_GROUND);
    bg.fill(60, 26, 4, 4, EMPTY, PAL_GROUND);
    bg.fill(84, 26, 4, 4, EMPTY, PAL_GROUND);

    // Everything below is built from the *same* block tile. Only the palette
    // differs, which is why some of it is stone and some is brick.

    // First screen: two platforms and a pillar.
    bg.fill(6, 20, 8, 2, BLOCK, PAL_STONE);
    bg.fill(18, 14, 8, 2, BLOCK, PAL_BRICK);
    bg.fill(28, 22, 2, 4, BLOCK, PAL_BRICK);

    // Second screen: a stone staircase, then two platforms before the first pit.
    bg.fill(36, 24, 2, 2, BLOCK, PAL_STONE);
    bg.fill(38, 22, 2, 4, BLOCK, PAL_STONE);
    bg.fill(40, 20, 2, 6, BLOCK, PAL_STONE);
    bg.fill(46, 18, 6, 2, BLOCK, PAL_BRICK);
    bg.fill(54, 14, 6, 2, BLOCK, PAL_STONE);

    // Third screen: rising brick steps over the second pit, then a wall too
    // tall to jump from the ground. The only way over is from the top step.
    bg.fill(68, 20, 4, 2, BLOCK, PAL_BRICK);
    bg.fill(74, 16, 4, 2, BLOCK, PAL_BRICK);
    bg.fill(80, 12, 4, 2, BLOCK, PAL_BRICK);
    bg.fill(90, 16, 2, 10, BLOCK, PAL_STONE);

    // The goal: a flag on a pole, just past the wall.
    bg.fill(FLAG_COL, FLAG_ROW, 1, 1, FLAG, PAL_STONE);
    bg.fill(FLAG_COL, FLAG_ROW + 1, 1, POLE_ROWS - 1, POLE, PAL_STONE);

    // Clouds use palette 0, which every block starts with.
    for (col, row, middle) in [
        (4, 4, 2), (20, 8, 1), (26, 3, 3),
        (38, 5, 2), (50, 9, 1), (58, 3, 2),
        (70, 6, 3), (84, 4, 1), (88, 9, 2),
    ] {
        cloud(&mut bg, col, row, middle);
    }

    bg
}

/// A cloud: left cap, `middle` centre pieces, right cap.
fn cloud(bg: &mut Background, col: usize, row: usize, middle: usize) {
    bg.set_tile(col, row, CLOUD_L);
    for i in 0..middle {
        bg.set_tile(col + 1 + i, row, CLOUD_M);
    }
    bg.set_tile(col + 1 + middle, row, CLOUD_R);
}

/// Gem positions, in level pixels.
fn place_gems() -> Vec<Gem> {
    // Ten in a row over the first screen (two more than fit on one line)...
    let mut spots: Vec<(i32, i32)> = (0..10).map(|i| (48 + i * 16, 88)).collect();
    // ...then a few above the staircase, the platforms and each brick step.
    // Each floats 12 pixels above the hero's head, so it takes a jump.
    spots.extend([
        (324, 132),
        (384, 116), (400, 116),
        (448, 84), (464, 84),
        (552, 132), (600, 100), (648, 68),
    ]);
    spots.into_iter().map(|(x, y)| Gem { x, y, collected: false }).collect()
}

/// Where the enemies start, as the top-left of each 16x16 drawing. None can
/// reach the spot where the hero appears.
fn place_enemies() -> Vec<Enemy> {
    [
        (80, 144),  // on the first stone platform
        (256, 192), // between the pillar and the staircase
        (400, 192), // between the staircase and the first pit
        (592, 192), // between the two pits, under the brick steps
    ]
    .into_iter()
    .map(|(x, y)| Enemy::new(x, y))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const RIGHT: u8 = Button::Right as u8;
    const A: u8 = Button::A as u8;
    const B: u8 = Button::B as u8;

    /// Hold the given buttons for a number of ticks.
    fn hold(demo: &mut Demo, pad: &mut Controller, raw: u8, ticks: u32) {
        for _ in 0..ticks {
            pad.update(raw);
            demo.update(pad);
        }
    }

    fn collected(demo: &Demo) -> usize {
        demo.gems.iter().filter(|gem| gem.collected).count()
    }

    #[test]
    fn hero_drops_onto_the_grass_and_stays_there() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        hold(&mut demo, &mut pad, 0, 120);
        assert!(demo.player.body.on_ground);
        // Grass surface is row 26 (y=208); the hero is 16 tall.
        assert_eq!((demo.player.sprite_x(), demo.player.sprite_y()), (16, 192));
    }

    #[test]
    fn hero_can_jump_up_onto_the_brick_pillar() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        hold(&mut demo, &mut pad, 0, 60); // land
        hold(&mut demo, &mut pad, RIGHT, 200); // walk until the pillar blocks us
        assert_eq!(demo.player.body.left() + 10, 224); // flush against its left side
        hold(&mut demo, &mut pad, RIGHT | A, 20); // jump, drifting right
        hold(&mut demo, &mut pad, A, 20); // keep rising
        hold(&mut demo, &mut pad, 0, 40); // come down and settle
        assert!(demo.player.body.on_ground);
        assert_eq!(demo.player.sprite_y(), 160); // pillar top is row 22 (y=176)
    }

    #[test]
    fn camera_follows_the_hero_across_the_level() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        demo.enemies.clear(); // this test is about the camera only
        hold(&mut demo, &mut pad, 0, 60);
        assert_eq!(demo.camera.x, 0); // near the left end: nothing to scroll yet

        // Walk to the pillar: the hero's middle passes the band, so we scroll.
        hold(&mut demo, &mut pad, RIGHT, 200);
        assert_eq!(demo.camera.x, 59);

        // Hop onto the pillar, then run on until the staircase blocks the way.
        hold(&mut demo, &mut pad, RIGHT | A, 20);
        hold(&mut demo, &mut pad, A, 20);
        hold(&mut demo, &mut pad, RIGHT | B, 200);
        assert_eq!(demo.player.body.left() + 10, 36 * 8); // flush against the first step
        let on_screen = demo.player.sprite_x() + 8 - demo.camera.x;
        assert_eq!(on_screen, 160); // held at the right edge of the band
        assert!(demo.camera.x > 100);

        // Walking back does not scroll until the hero crosses the band.
        let before = demo.camera.x;
        hold(&mut demo, &mut pad, Button::Left as u8, 20);
        assert_eq!(demo.camera.x, before);
    }

    #[test]
    fn touching_a_gem_collects_it() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        demo.player = Player::new(320, 144); // on top of the staircase, under a gem
        hold(&mut demo, &mut pad, 0, 5);
        assert_eq!(collected(&demo), 0); // standing under it is not enough
        hold(&mut demo, &mut pad, A, 30); // jump up into it
        assert_eq!(collected(&demo), 1);
    }

    #[test]
    fn counter_spells_out_collected_and_total() {
        assert_eq!(
            hud_tiles(7, 18),
            [GEM, DIGIT, DIGIT + 7, SLASH, DIGIT + 1, DIGIT + 8]
        );
        assert_eq!(hud_tiles(0, 18)[1..3], [DIGIT, DIGIT]);
        assert_eq!(hud_tiles(250, 18)[1..3], [DIGIT + 9, DIGIT + 9]); // capped at 99
    }

    #[test]
    fn counter_stays_put_while_the_level_scrolls() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        let counter = |demo: &Demo| {
            let mut fb = Framebuffer::new();
            demo.draw(&mut fb);
            // The 8 rows of the screen that hold the counter.
            fb.pixels()[8 * SCREEN_W..16 * SCREEN_W][..64].to_vec()
        };
        let before = counter(&demo);
        assert!(before.contains(&0x20)); // something white is drawn there
        hold(&mut demo, &mut pad, 0, 60);
        hold(&mut demo, &mut pad, RIGHT, 200);
        assert!(demo.camera.x > 0); // the level has scrolled...
        // ...but the first row of the counter is pixel-for-pixel where it was.
        assert_eq!(counter(&demo), before);
    }

    #[test]
    fn falling_down_a_pit_sends_the_hero_back_to_the_start() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        demo.player = Player::new(488, 192); // over the first pit
        hold(&mut demo, &mut pad, 0, 10);
        assert!(demo.player.sprite_y() > 208); // dropping below ground level
        hold(&mut demo, &mut pad, 0, 120);
        assert!(demo.player.body.on_ground);
        assert_eq!((demo.player.sprite_x(), demo.player.sprite_y()), (16, 192));
        assert_eq!(demo.camera.x, 0);
    }

    #[test]
    fn the_end_wall_cannot_be_jumped_from_the_ground() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        demo.player = Player::new(704, 192); // between the second pit and the wall
        hold(&mut demo, &mut pad, 0, 5);
        hold(&mut demo, &mut pad, RIGHT | B | A, 120); // biggest possible jump at it
        hold(&mut demo, &mut pad, RIGHT, 60);
        assert!(demo.player.body.left() + 10 <= 720); // still on the near side
        assert!(demo.celebration.is_none());
    }

    #[test]
    fn reaching_the_flag_celebrates_then_restarts_the_level() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        demo.enemies[0].squash(); // and defeat an enemy
        demo.player = Player::new(320, 144);
        hold(&mut demo, &mut pad, 0, 5); // settle, so the jump counts
        hold(&mut demo, &mut pad, A, 30); // collect a gem on the way
        assert_eq!(collected(&demo), 1);

        // From the top brick step, a running jump clears the pit and lands
        // on the wall; carrying on right drops down to the flag.
        demo.player = Player::new(644, 80);
        hold(&mut demo, &mut pad, 0, 10);
        assert!(demo.player.body.on_ground);
        hold(&mut demo, &mut pad, RIGHT | B | A, 30);
        hold(&mut demo, &mut pad, RIGHT | B, 90);
        assert!(demo.celebration.is_some());
        assert_ne!(demo.palettes.backdrop, SKY); // the sky has changed colour

        // The controller is ignored during the celebration...
        let x = demo.player.sprite_x();
        hold(&mut demo, &mut pad, Button::Left as u8, 60);
        assert!(demo.player.sprite_x() >= x);

        // ...and afterwards everything is back where it started.
        hold(&mut demo, &mut pad, 0, CELEBRATION_TICKS);
        assert!(demo.celebration.is_none());
        assert_eq!(demo.palettes.backdrop, SKY);
        assert_eq!(collected(&demo), 0);
        assert!(demo.enemies.iter().all(|enemy| enemy.is_dangerous()));
        assert_eq!(demo.camera.x, 0);
        assert!(demo.player.sprite_x() < 32);
    }

    #[test]
    fn landing_on_an_enemy_squashes_it_and_bounces_the_hero() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        hold(&mut demo, &mut pad, 0, 10); // let the enemies settle
        let (foe_x, foe_y) = (demo.enemies[2].sprite_x(), demo.enemies[2].sprite_y());
        demo.player = Player::new(foe_x - 2, foe_y - 20); // just above its head
        let mut bounced = false;
        for _ in 0..40 {
            hold(&mut demo, &mut pad, 0, 1);
            bounced |= demo.player.body.vy < 0; // moving up again after the stomp
        }
        assert!(bounced);
        assert!(!demo.enemies[2].is_dangerous());
        assert!(demo.player.sprite_x() > 300); // not sent back to the start
        hold(&mut demo, &mut pad, 0, 30);
        assert_eq!(demo.enemies[2].state, EnemyState::Gone);
    }

    #[test]
    fn walking_into_an_enemy_sends_the_hero_back_to_the_start() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        hold(&mut demo, &mut pad, 0, 10);
        let foe_x = demo.enemies[2].sprite_x();
        demo.player = Player::new(foe_x - 40, 192); // on the ground, in its path
        for _ in 0..120 {
            hold(&mut demo, &mut pad, RIGHT, 1); // walk straight into it
            if demo.player.sprite_x() < 100 {
                break; // sent back
            }
        }
        hold(&mut demo, &mut pad, 0, 60);
        assert_eq!((demo.player.sprite_x(), demo.player.sprite_y()), (16, 192));
        assert!(demo.enemies[2].is_dangerous()); // the enemy is unharmed
    }

    #[test]
    fn each_event_asks_for_its_sound_exactly_once() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        hold(&mut demo, &mut pad, 0, 30);
        assert_eq!(demo.take_sounds(), []); // landing at the start is silent

        // A held jump button makes one jump sound, not one per tick.
        hold(&mut demo, &mut pad, A, 20);
        assert_eq!(demo.take_sounds(), [Sfx::Jump]);
        hold(&mut demo, &mut pad, 0, 60);

        // Jumping into a gem: the jump, then the pickup.
        demo.player = Player::new(320, 144);
        hold(&mut demo, &mut pad, 0, 5);
        hold(&mut demo, &mut pad, A, 30);
        assert_eq!(demo.take_sounds(), [Sfx::Jump, Sfx::Gem]);

        // Dropping onto an enemy.
        let (foe_x, foe_y) = (demo.enemies[2].sprite_x(), demo.enemies[2].sprite_y());
        demo.player = Player::new(foe_x - 2, foe_y - 20);
        hold(&mut demo, &mut pad, 0, 15);
        assert_eq!(demo.take_sounds(), [Sfx::Stomp]);

        // Falling down a pit.
        demo.player = Player::new(488, 192);
        hold(&mut demo, &mut pad, 0, 60);
        assert_eq!(demo.take_sounds(), [Sfx::Hurt]);

        // Pausing and unpausing.
        hold(&mut demo, &mut pad, Button::Start as u8, 10);
        hold(&mut demo, &mut pad, 0, 1);
        hold(&mut demo, &mut pad, Button::Start as u8, 1);
        assert_eq!(demo.take_sounds(), [Sfx::Pause, Sfx::Pause]);

        // Touching the flag.
        demo.player = Player::new(748, 192);
        hold(&mut demo, &mut pad, 0, 30);
        assert_eq!(demo.take_sounds(), [Sfx::Flag]);
    }

    #[test]
    fn no_enemy_ever_reaches_the_starting_spot() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        hold(&mut demo, &mut pad, 0, 3600); // a full minute of standing still
        assert_eq!((demo.player.sprite_x(), demo.player.sprite_y()), (16, 192));
        assert!(demo.enemies.iter().all(|enemy| enemy.is_dangerous()));
    }

    #[test]
    fn at_most_eight_gems_are_drawn_on_the_first_screen() {
        let demo = Demo::new();
        let mut fb = Framebuffer::new();
        demo.draw(&mut fb);
        // Count gems by their shine pixel (white, at offset 3,1 of each gem).
        let shown = (0..10)
            .filter(|i| fb.pixels()[89 * SCREEN_W + (48 + i * 16 + 3)] == 0x20)
            .count();
        assert_eq!(shown, 8);
    }

    #[test]
    fn holding_start_pauses_once_and_freezes_the_hero() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        hold(&mut demo, &mut pad, Button::Start as u8, 30); // held, not tapped
        assert!(demo.paused);
        hold(&mut demo, &mut pad, RIGHT, 10);
        assert_eq!((demo.player.sprite_x(), demo.player.sprite_y()), (16, 160)); // frozen mid-air
        hold(&mut demo, &mut pad, Button::Start as u8, 1);
        assert!(!demo.paused);
    }

    #[test]
    fn holding_select_changes_colour_only_once() {
        let (mut demo, mut pad) = (Demo::new(), Controller::new());
        hold(&mut demo, &mut pad, Button::Select as u8, 30);
        assert_eq!(demo.hero_color, 1);
        assert_eq!(demo.palettes.sprite[PAL_HERO as usize][0], HERO_COLORS[1]);
    }
}
