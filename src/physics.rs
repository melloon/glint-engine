//! Movement and collision against the tile map.
//!
//! Two ideas from 8-bit games live here:
//!
//! 1. Sub-pixel positions. Positions and speeds are whole numbers counted in
//!    1/256ths of a pixel, so a body can move at 1.5 pixels per tick, or
//!    accelerate by a tenth of a pixel, without any floating point.
//! 2. Tile collision. Nothing is tested against individual pixels or
//!    polygons. A body is a rectangle, and the only question ever asked is
//!    "is the tile under this point solid?".

use crate::background::{Background, ROWS};
use crate::tiles::TILE_SIZE;

/// Sub-pixels per pixel.
pub const SUBPIXEL: i32 = 256;

const TILE: i32 = TILE_SIZE as i32;

/// Bodies are checked once per tick at their new position, so anything
/// moving a whole tile per tick could jump clean over a wall. Cap speed
/// just below that.
const MAX_SPEED: i32 = TILE * SUBPIXEL - 1;

/// The solid parts of the world: the background map plus a table saying
/// which tile numbers are solid.
pub struct Terrain<'a> {
    pub background: &'a Background,
    pub solid: &'a [bool; 256],
}

impl Terrain<'_> {
    /// Is the pixel at (x, y) inside a solid tile?
    pub fn solid_at(&self, x: i32, y: i32) -> bool {
        let (col, row) = (x.div_euclid(TILE), y.div_euclid(TILE));
        if col < 0 || col >= self.background.cols() as i32 {
            return true; // the left and right ends of the map are walls
        }
        if row < 0 || row >= ROWS as i32 {
            return false; // open sky above, a pit below
        }
        self.solid[self.background.tile_at(col as usize, row as usize) as usize]
    }

    /// Is any tile solid along the vertical line at `x` from `top` to `bottom`?
    fn solid_in_column(&self, x: i32, top: i32, bottom: i32) -> bool {
        (top.div_euclid(TILE)..=bottom.div_euclid(TILE)).any(|row| self.solid_at(x, row * TILE))
    }

    /// Is any tile solid along the horizontal line at `y` from `left` to `right`?
    fn solid_in_row(&self, y: i32, left: i32, right: i32) -> bool {
        (left.div_euclid(TILE)..=right.div_euclid(TILE)).any(|col| self.solid_at(col * TILE, y))
    }
}

/// A moving rectangle.
pub struct Body {
    /// Top-left corner, in sub-pixels.
    pub x: i32,
    pub y: i32,
    /// Speed, in sub-pixels per tick. Positive is right / down.
    pub vx: i32,
    pub vy: i32,
    /// Size, in whole pixels.
    pub w: i32,
    pub h: i32,
    /// True when standing on something solid. Updated by `step`.
    pub on_ground: bool,
}

impl Body {
    /// A body at rest with its top-left corner at pixel (x, y).
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            x: x * SUBPIXEL,
            y: y * SUBPIXEL,
            vx: 0,
            vy: 0,
            w,
            h,
            on_ground: false,
        }
    }

    // The pixels the body covers: left..=right across, top..=bottom down.
    pub fn left(&self) -> i32 {
        self.x.div_euclid(SUBPIXEL)
    }
    pub fn top(&self) -> i32 {
        self.y.div_euclid(SUBPIXEL)
    }
    fn right(&self) -> i32 {
        self.left() + self.w - 1
    }
    fn bottom(&self) -> i32 {
        self.top() + self.h - 1
    }

    /// Does the body overlap the rectangle at pixel (x, y) of size w by h?
    /// Used for touching things that are not solid, such as pickups.
    pub fn overlaps(&self, x: i32, y: i32, w: i32, h: i32) -> bool {
        self.left() < x + w && self.right() >= x && self.top() < y + h && self.bottom() >= y
    }

    /// Move by the current speed for one tick, stopping at solid tiles.
    ///
    /// The two axes are handled one after the other. That way, when the body
    /// ends up inside a tile, we always know which direction it came from and
    /// therefore which way to push it back out.
    pub fn step(&mut self, terrain: &Terrain) {
        self.vx = self.vx.clamp(-MAX_SPEED, MAX_SPEED);
        self.vy = self.vy.clamp(-MAX_SPEED, MAX_SPEED);

        // Horizontal: move, then check the edge that is leading the way.
        self.x += self.vx;
        if self.vx > 0 && terrain.solid_in_column(self.right(), self.top(), self.bottom()) {
            let wall = self.right().div_euclid(TILE) * TILE; // left side of the tile we hit
            self.x = (wall - self.w) * SUBPIXEL;
            self.vx = 0;
        } else if self.vx < 0 && terrain.solid_in_column(self.left(), self.top(), self.bottom()) {
            let wall = (self.left().div_euclid(TILE) + 1) * TILE; // right side of that tile
            self.x = wall * SUBPIXEL;
            self.vx = 0;
        }

        // Vertical: the same again, for landing and for bumping a ceiling.
        self.y += self.vy;
        if self.vy > 0 && terrain.solid_in_row(self.bottom(), self.left(), self.right()) {
            let floor = self.bottom().div_euclid(TILE) * TILE;
            self.y = (floor - self.h) * SUBPIXEL;
            self.vy = 0;
        } else if self.vy < 0 && terrain.solid_in_row(self.top(), self.left(), self.right()) {
            let ceiling = (self.top().div_euclid(TILE) + 1) * TILE;
            self.y = ceiling * SUBPIXEL;
            self.vy = 0;
        }

        // Standing means: not moving upward, and solid directly under the feet.
        self.on_ground =
            self.vy >= 0 && terrain.solid_in_row(self.bottom() + 1, self.left(), self.right());
        if self.on_ground {
            // Rest exactly on the surface, so gravity does not slowly build
            // up a fraction of a pixel while standing still.
            self.vy = 0;
            self.y = self.top() * SUBPIXEL;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRAVITY: i32 = 0x40;

    fn solid() -> [bool; 256] {
        let mut s = [false; 256];
        s[1] = true;
        s
    }

    /// Floor from y=160 down, a wall at x=80..88 standing on it (y=128..160),
    /// and a floating slab at x=160..192, y=120..128.
    fn world() -> Background {
        let mut bg = Background::with_width(32);
        bg.fill(0, 20, 32, 10, 1, 0);
        bg.fill(10, 16, 1, 4, 1, 0);
        bg.fill(20, 15, 4, 1, 1, 0);
        bg
    }

    /// One tick of falling plus an optional constant sideways push.
    fn tick(body: &mut Body, terrain: &Terrain, vx: i32) {
        body.vx = vx;
        body.vy += GRAVITY;
        body.step(terrain);
    }

    #[test]
    fn falls_lands_and_rests_on_the_floor() {
        let (bg, solid) = (world(), solid());
        let terrain = Terrain { background: &bg, solid: &solid };
        let mut body = Body::new(16, 100, 10, 16);
        for _ in 0..200 {
            tick(&mut body, &terrain, 0);
        }
        assert!(body.on_ground);
        assert_eq!(body.top(), 144); // feet exactly on the floor at y=160
        assert_eq!((body.vy, body.y % SUBPIXEL), (0, 0)); // fully at rest
    }

    #[test]
    fn wall_stops_the_body_flush_against_it() {
        let (bg, solid) = (world(), solid());
        let terrain = Terrain { background: &bg, solid: &solid };
        let mut body = Body::new(40, 144, 10, 16);
        for _ in 0..60 {
            tick(&mut body, &terrain, 0x200);
        }
        assert_eq!(body.left(), 70); // right edge touching the wall at x=80
        assert_eq!(body.vx, 0);
        assert!(body.on_ground); // walking along the floor never snagged
    }

    #[test]
    fn ceiling_stops_a_jump() {
        let (bg, solid) = (world(), solid());
        let terrain = Terrain { background: &bg, solid: &solid };
        let mut body = Body::new(165, 144, 10, 16);
        body.vy = -0x480;
        let mut highest = body.top();
        for _ in 0..60 {
            tick(&mut body, &terrain, 0);
            highest = highest.min(body.top());
        }
        assert_eq!(highest, 128); // the underside of the slab
        assert!(body.on_ground); // and it came back down
    }

    #[test]
    fn walking_off_a_ledge_starts_a_fall() {
        let (bg, solid) = (world(), solid());
        let terrain = Terrain { background: &bg, solid: &solid };
        let mut body = Body::new(170, 104, 10, 16); // standing on the slab
        tick(&mut body, &terrain, 0);
        assert!(body.on_ground);
        while body.left() < 192 {
            tick(&mut body, &terrain, 0x200);
        }
        assert!(!body.on_ground);
    }

    #[test]
    fn a_wide_map_extends_past_the_first_screen() {
        let mut bg = Background::with_width(64);
        bg.fill(0, 20, 64, 10, 1, 0);
        let solid = solid();
        let terrain = Terrain { background: &bg, solid: &solid };
        let mut body = Body::new(240, 144, 10, 16);
        for _ in 0..200 {
            tick(&mut body, &terrain, 0x200);
        }
        assert_eq!(body.left(), 64 * 8 - 10); // stopped by the end of the map, not the screen
    }

    #[test]
    fn overlap_needs_a_shared_pixel_not_just_a_shared_edge() {
        let body = Body::new(100, 50, 10, 16); // covers x 100..110, y 50..66
        assert!(body.overlaps(109, 65, 8, 8)); // shares the corner pixel
        assert!(!body.overlaps(110, 50, 8, 8)); // touching on the right
        assert!(!body.overlaps(100, 42, 8, 8)); // touching above
        assert!(body.overlaps(96, 40, 8, 11)); // reaches one pixel in
    }

    #[test]
    fn screen_edges_are_walls() {
        let (bg, solid) = (world(), solid());
        let terrain = Terrain { background: &bg, solid: &solid };
        let mut body = Body::new(5, 144, 10, 16);
        for _ in 0..20 {
            tick(&mut body, &terrain, -0x200);
        }
        assert_eq!(body.left(), 0);
    }

    #[test]
    fn half_a_pixel_per_tick_moves_every_other_tick() {
        let (bg, solid) = (world(), solid());
        let terrain = Terrain { background: &bg, solid: &solid };
        let mut body = Body::new(16, 144, 10, 16);
        let mut seen = Vec::new();
        for _ in 0..4 {
            tick(&mut body, &terrain, SUBPIXEL / 2);
            seen.push(body.left());
        }
        assert_eq!(seen, [16, 17, 17, 18]);
    }
}
