//! The background layer: a grid of tile numbers, shown through a window
//! one screen wide.
//!
//! One screen is 32x30 tiles. Each cell stores one byte (which tile to
//! show), so a whole screen costs 960 bytes instead of 61,440 pixels. A level
//! can be many screens wide; the scroll position picks which part is visible.

use crate::tiles::{PatternTable, TILE_SIZE};
use crate::video::{Framebuffer, Palettes, SCREEN_W};

/// Tiles across one screen.
pub const SCREEN_COLS: usize = 32;
/// Tiles down the screen. Levels scroll sideways only, so this is fixed.
pub const ROWS: usize = 30;

pub struct Background {
    /// Width of the map in tiles: at least one screen, and always even.
    cols: usize,
    /// Which tile is in each cell, row by row.
    tiles: Vec<u8>,
    /// Which background palette (0-3) each 16x16 block (2x2 tiles) uses. Real
    /// hardware squeezed four of these into one byte; we spend a byte each.
    attributes: Vec<u8>,
}

impl Background {
    /// An empty map `cols` tiles wide. One screen is 32.
    pub fn with_width(cols: usize) -> Self {
        assert!(cols >= SCREEN_COLS && cols % 2 == 0, "bad map width {cols}");
        Self {
            cols,
            tiles: vec![0; cols * ROWS],
            attributes: vec![0; (cols / 2) * (ROWS / 2)],
        }
    }

    /// Width of the map in tiles.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Width of the map in pixels.
    pub fn width(&self) -> i32 {
        (self.cols * TILE_SIZE) as i32
    }

    pub fn set_tile(&mut self, col: usize, row: usize, tile: u8) {
        assert!(col < self.cols && row < ROWS, "tile ({col}, {row}) is off the map");
        self.tiles[row * self.cols + col] = tile;
    }

    pub fn tile_at(&self, col: usize, row: usize) -> u8 {
        self.tiles[row * self.cols + col]
    }

    /// Choose the palette for the tile at (col, row). Because palettes are
    /// per 16x16 block, this also recolours the other three tiles that share
    /// its block. That is the famous 8-bit "attribute" limit.
    pub fn set_palette(&mut self, col: usize, row: usize, palette: u8) {
        assert!(col < self.cols && row < ROWS, "tile ({col}, {row}) is off the map");
        self.attributes[(row / 2) * (self.cols / 2) + col / 2] = palette & 3;
    }

    /// Fill a rectangle of cells with one tile and one palette.
    pub fn fill(&mut self, col: usize, row: usize, w: usize, h: usize, tile: u8, palette: u8) {
        for r in row..row + h {
            for c in col..col + w {
                self.set_tile(c, r, tile);
                self.set_palette(c, r, palette);
            }
        }
    }

    /// Paint the visible part of the map into the framebuffer.
    ///
    /// `scroll_x` is the scroll register: the map pixel that appears at the
    /// left edge of the screen. It is clamped so the view never leaves the map.
    pub fn draw(
        &self,
        fb: &mut Framebuffer,
        patterns: &PatternTable,
        palettes: &Palettes,
        scroll_x: i32,
    ) {
        let scroll_x = scroll_x.clamp(0, self.width() - SCREEN_W as i32) as usize;
        // When the scroll is not a multiple of 8, a sliver of a 33rd column
        // shows at the right edge while the first column is partly cut off.
        let first_col = scroll_x / TILE_SIZE;
        let last_col = (first_col + SCREEN_COLS).min(self.cols - 1);

        for row in 0..ROWS {
            for col in first_col..=last_col {
                let tile = patterns.get(self.tiles[row * self.cols + col]);
                let palette = self.attributes[(row / 2) * (self.cols / 2) + col / 2];
                for y in 0..TILE_SIZE {
                    for x in 0..TILE_SIZE {
                        // tile pixel (0-3) -> palette slot -> master colour
                        let color = palettes.background_color(palette, tile.pixel(x, y));
                        // Map position minus scroll gives the screen position;
                        // anything off screen is clipped by `set`.
                        fb.set(
                            (col * TILE_SIZE + x) as i32 - scroll_x as i32,
                            (row * TILE_SIZE + y) as i32,
                            color,
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tiles::Tile;

    fn palettes() -> Palettes {
        let mut p = Palettes::new();
        p.backdrop = 0x2C;
        p.background[0] = [0x01, 0x02, 0x03];
        p.background[1] = [0x11, 0x12, 0x13];
        p
    }

    fn patterns() -> PatternTable {
        let mut t = PatternTable::new();
        t.set(1, Tile::from_art(["1.......", "........", "........", "........",
                                 "........", "........", "........", ".......3"]));
        t
    }

    #[test]
    fn tile_lands_at_its_cell_in_its_palette() {
        let mut bg = Background::with_width(32);
        bg.set_tile(2, 1, 1);
        bg.set_palette(2, 1, 1);
        let mut fb = Framebuffer::new();
        bg.draw(&mut fb, &patterns(), &palettes(), 0);

        let at = |x: usize, y: usize| fb.pixels()[y * SCREEN_W + x];
        assert_eq!(at(16, 8), 0x11); // top-left pixel: value 1, palette 1
        assert_eq!(at(23, 15), 0x13); // bottom-right pixel: value 3, palette 1
        assert_eq!(at(17, 8), 0x2C); // value 0 shows the backdrop
        assert_eq!(at(0, 0), 0x2C); // empty cell shows the backdrop
    }

    #[test]
    fn palette_is_shared_by_the_whole_2x2_block() {
        let mut bg = Background::with_width(32);
        bg.set_tile(4, 6, 1);
        bg.set_tile(5, 7, 1); // same 16x16 block as (4, 6)
        bg.set_tile(6, 6, 1); // the block next door
        bg.set_palette(4, 6, 1);
        let mut fb = Framebuffer::new();
        bg.draw(&mut fb, &patterns(), &palettes(), 0);

        let at = |x: usize, y: usize| fb.pixels()[y * SCREEN_W + x];
        assert_eq!(at(32, 48), 0x11); // the tile we set
        assert_eq!(at(40, 56), 0x11); // its block-mate changed too
        assert_eq!(at(48, 48), 0x01); // the neighbour block did not
    }

    #[test]
    fn scrolling_slides_the_map_and_clips_at_the_edges() {
        let mut bg = Background::with_width(64); // two screens wide
        bg.set_tile(40, 0, 1); // map x = 320..328, on the second screen
        bg.set_tile(1, 0, 1); // map x = 8..16
        let draw = |scroll_x: i32| {
            let mut fb = Framebuffer::new();
            bg.draw(&mut fb, &patterns(), &palettes(), scroll_x);
            fb
        };
        let at = |fb: &Framebuffer, x: usize, y: usize| fb.pixels()[y * SCREEN_W + x];

        let fb = draw(0);
        assert_eq!(at(&fb, 8, 0), 0x01); // tile 1 visible, tile 40 off to the right
        let fb = draw(100);
        assert_eq!(at(&fb, 220, 0), 0x01); // tile 40: 320 - 100
        assert_eq!(at(&fb, 8, 0), 0x2C); // tile 1 has scrolled away
        let fb = draw(11);
        assert_eq!(at(&fb, 4, 7), 0x03); // tile 1's last pixel (map 15,7) at 15 - 11
        assert_eq!(at(&fb, 0, 0), 0x2C); // its first pixel is cut off by the edge
        let fb = draw(9999);
        assert_eq!(at(&fb, 64, 0), 0x01); // clamped to 256: tile 40 at 320 - 256
    }
}
