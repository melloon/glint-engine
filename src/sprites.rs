//! Sprites: small movable pictures drawn on top of the background.
//!
//! A sprite is one 8x8 tile placed at any pixel position. The hardware limits
//! are the ones that shaped 8-bit games:
//!   - at most 64 sprites on screen,
//!   - at most 8 of them on any one horizontal line of the picture,
//!   - 3 colours each, with pixel value 0 transparent.

use crate::tiles::{PatternTable, TILE_SIZE};
use crate::video::{Framebuffer, Palettes, SCREEN_H};

pub const MAX_SPRITES: usize = 64;
pub const MAX_PER_LINE: usize = 8;

#[derive(Clone, Copy)]
pub struct Sprite {
    /// Screen position of the top-left corner, in pixels.
    pub x: i32,
    pub y: i32,
    /// Which tile of the sprite pattern table to show.
    pub tile: u8,
    /// Which sprite palette (0-3) to colour it with.
    pub palette: u8,
    /// Mirror the tile, so one drawing can face both ways.
    pub flip_x: bool,
    pub flip_y: bool,
}

/// The list of sprites to show this frame. The game clears it and refills it
/// every frame; earlier entries have priority over later ones.
pub struct SpriteList {
    list: Vec<Sprite>,
}

impl SpriteList {
    pub fn new() -> Self {
        Self {
            list: Vec::with_capacity(MAX_SPRITES),
        }
    }

    pub fn clear(&mut self) {
        self.list.clear();
    }

    /// Add a sprite. Returns false (and drops it) if all 64 slots are taken.
    pub fn push(&mut self, sprite: Sprite) -> bool {
        if self.list.len() == MAX_SPRITES {
            return false;
        }
        self.list.push(sprite);
        true
    }

    /// Draw the sprites over whatever is already in the framebuffer.
    ///
    /// This works one horizontal line at a time, like the real hardware,
    /// because that is where the 8-per-line limit comes from.
    pub fn draw(&self, fb: &mut Framebuffer, patterns: &PatternTable, palettes: &Palettes) {
        let size = TILE_SIZE as i32;
        for line in 0..SCREEN_H as i32 {
            // Scan the list in order and keep the first 8 sprites that touch
            // this line. A ninth is simply not drawn on this line.
            let mut on_line = [0usize; MAX_PER_LINE];
            let mut count = 0;
            for (i, sprite) in self.list.iter().enumerate() {
                if line >= sprite.y && line < sprite.y + size {
                    if count == MAX_PER_LINE {
                        break;
                    }
                    on_line[count] = i;
                    count += 1;
                }
            }

            // Draw back to front so the earliest sprite ends up on top.
            for &i in on_line[..count].iter().rev() {
                let sprite = &self.list[i];
                let tile = patterns.get(sprite.tile);
                let row = (line - sprite.y) as usize;
                let ty = if sprite.flip_y { TILE_SIZE - 1 - row } else { row };
                for col in 0..TILE_SIZE {
                    let tx = if sprite.flip_x { TILE_SIZE - 1 - col } else { col };
                    if let Some(color) = palettes.sprite_color(sprite.palette, tile.pixel(tx, ty)) {
                        fb.set(sprite.x + col as i32, line, color);
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
    use crate::video::SCREEN_W;

    const SOLID: u8 = 0; // every pixel is value 1
    const CORNER: u8 = 1; // value 3 in the top-left pixel, rest transparent

    fn patterns() -> PatternTable {
        let mut t = PatternTable::new();
        t.set(SOLID, Tile::from_art(["11111111"; 8]));
        t.set(CORNER, Tile::from_art(["3.......", "........", "........", "........",
                                      "........", "........", "........", "........"]));
        t
    }

    fn palettes() -> Palettes {
        let mut p = Palettes::new();
        p.sprite[0] = [0x11, 0x12, 0x13];
        p.sprite[1] = [0x21, 0x22, 0x23];
        p
    }

    fn sprite(x: i32, y: i32, tile: u8, palette: u8) -> Sprite {
        Sprite { x, y, tile, palette, flip_x: false, flip_y: false }
    }

    fn render(sprites: &SpriteList) -> Framebuffer {
        let mut fb = Framebuffer::new(); // starts all 0x0F
        sprites.draw(&mut fb, &patterns(), &palettes());
        fb
    }

    fn at(fb: &Framebuffer, x: usize, y: usize) -> u8 {
        fb.pixels()[y * SCREEN_W + x]
    }

    #[test]
    fn value_zero_is_transparent() {
        let mut sprites = SpriteList::new();
        sprites.push(sprite(10, 20, CORNER, 0));
        let fb = render(&sprites);
        assert_eq!(at(&fb, 10, 20), 0x13); // the one opaque pixel
        assert_eq!(at(&fb, 11, 20), 0x0F); // the rest is untouched
    }

    #[test]
    fn flips_mirror_the_tile() {
        let mut sprites = SpriteList::new();
        sprites.push(Sprite { flip_x: true, flip_y: true, ..sprite(10, 20, CORNER, 0) });
        let fb = render(&sprites);
        assert_eq!(at(&fb, 10, 20), 0x0F);
        assert_eq!(at(&fb, 17, 27), 0x13); // corner moved to bottom-right
    }

    #[test]
    fn earlier_sprite_is_drawn_on_top() {
        let mut sprites = SpriteList::new();
        sprites.push(sprite(10, 20, SOLID, 0));
        sprites.push(sprite(10, 20, SOLID, 1));
        assert_eq!(at(&render(&sprites), 10, 20), 0x11);
    }

    #[test]
    fn ninth_sprite_on_a_line_is_dropped() {
        let mut sprites = SpriteList::new();
        for i in 0..9 {
            sprites.push(sprite(i * 16, 20, SOLID, 0));
        }
        let fb = render(&sprites);
        assert_eq!(at(&fb, 7 * 16, 20), 0x11); // eighth is there
        assert_eq!(at(&fb, 8 * 16, 20), 0x0F); // ninth is not
    }

    #[test]
    fn list_holds_at_most_64_sprites() {
        let mut sprites = SpriteList::new();
        for i in 0..64 {
            assert!(sprites.push(sprite(0, i * 8, SOLID, 0)));
        }
        assert!(!sprites.push(sprite(0, 0, SOLID, 0)));
    }
}
