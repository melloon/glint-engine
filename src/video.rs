//! The console's "video hardware": a fixed master palette and a framebuffer
//! of palette indices. Nothing in here knows about the host window; it is
//! plain data, which keeps it easy to test.

pub const SCREEN_W: usize = 256;
pub const SCREEN_H: usize = 240;

/// The console can only ever show these 64 colours. A pixel is a 6-bit index
/// into this table, never an RGB value.
pub const MASTER_COLORS: usize = 64;

pub type Rgb = [u8; 3];

/// Build the 64-colour master palette: 4 brightness rows of 16 columns.
/// Column 0 is a grey, columns 1-12 walk around the colour wheel, column 13
/// is a darker grey, and columns 14-15 are black (as on period hardware,
/// where several entries were duplicates of black).
pub fn master_palette() -> [Rgb; MASTER_COLORS] {
    // (saturation, value) for the hue columns of each row, dark to pastel.
    const ROWS: [(f32, f32); 4] = [(1.0, 0.45), (0.9, 0.75), (0.6, 1.0), (0.3, 1.0)];
    const GREY: [f32; 4] = [0.40, 0.70, 1.0, 1.0];
    const DARK_GREY: [f32; 4] = [0.0, 0.0, 0.30, 0.70];

    let mut palette = [[0u8; 3]; MASTER_COLORS];
    for row in 0..4 {
        for col in 0..16 {
            palette[row * 16 + col] = match col {
                0 => grey(GREY[row]),
                1..=12 => {
                    // Start at blue and step 30 degrees per column.
                    let hue = (240.0 + (col as f32 - 1.0) * 30.0) % 360.0;
                    hsv_to_rgb(hue, ROWS[row].0, ROWS[row].1)
                }
                13 => grey(DARK_GREY[row]),
                _ => [0, 0, 0],
            };
        }
    }
    palette
}

fn grey(value: f32) -> Rgb {
    let v = (value * 255.0).round() as u8;
    [v, v, v]
}

fn hsv_to_rgb(hue: f32, sat: f32, val: f32) -> Rgb {
    let c = val * sat;
    let h = hue / 60.0;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = val - c;
    let to_byte = |f: f32| ((f + m) * 255.0).round() as u8;
    [to_byte(r), to_byte(g), to_byte(b)]
}

/// The console's palette memory: the handful of master colours that are
/// "loaded" right now. Tiles never name a master colour directly; they pick
/// a slot in one of these small palettes.
pub struct Palettes {
    /// The one colour shown wherever a tile pixel is 0, shared by all tiles.
    pub backdrop: u8,
    /// Four background palettes of three colours each (slots 1-3).
    pub background: [[u8; 3]; 4],
    /// Four sprite palettes of three colours each (slots 1-3). Slot 0 of a
    /// sprite is not a colour at all: it is transparent.
    pub sprite: [[u8; 3]; 4],
}

impl Palettes {
    pub fn new() -> Self {
        Self {
            backdrop: 0x0F,
            background: [[0x0F; 3]; 4],
            sprite: [[0x0F; 3]; 4],
        }
    }

    /// Resolve a sprite pixel (0-3) drawn with a sprite palette (0-3).
    /// `None` means transparent: whatever is behind shows through.
    pub fn sprite_color(&self, palette: u8, value: u8) -> Option<u8> {
        match value & 3 {
            0 => None,
            slot => Some(self.sprite[(palette & 3) as usize][slot as usize - 1]),
        }
    }

    /// Resolve a tile pixel (0-3) drawn with a background palette (0-3) to a
    /// master-palette index.
    pub fn background_color(&self, palette: u8, value: u8) -> u8 {
        match value & 3 {
            0 => self.backdrop,
            slot => self.background[(palette & 3) as usize][slot as usize - 1],
        }
    }
}

/// 256x240 pixels, each one a master-palette index (0-63).
pub struct Framebuffer {
    pixels: Vec<u8>,
}

impl Framebuffer {
    pub fn new() -> Self {
        Self {
            pixels: vec![0x0F; SCREEN_W * SCREEN_H], // 0x0F is black
        }
    }

    /// Plot one pixel. Off-screen coordinates are silently ignored, so
    /// callers can draw things that hang over the edge.
    pub fn set(&mut self, x: i32, y: i32, color: u8) {
        if x >= 0 && y >= 0 && (x as usize) < SCREEN_W && (y as usize) < SCREEN_H {
            // Only 6 bits reach the "hardware", exactly like the real thing.
            self.pixels[y as usize * SCREEN_W + x as usize] = color & 0x3F;
        }
    }

    /// Not used by the tile renderer; kept for debug overlays such as
    /// collision boxes in a later step.
    #[allow(dead_code)]
    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: u8) {
        for py in y..y + h {
            for px in x..x + w {
                self.set(px, py, color);
            }
        }
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_writes_the_right_pixel() {
        let mut fb = Framebuffer::new();
        fb.set(3, 2, 0x21);
        assert_eq!(fb.pixels()[2 * SCREEN_W + 3], 0x21);
    }

    #[test]
    fn off_screen_writes_are_ignored() {
        let mut fb = Framebuffer::new();
        fb.set(-1, 0, 0x21);
        fb.set(0, -1, 0x21);
        fb.set(SCREEN_W as i32, 0, 0x21);
        fb.set(0, SCREEN_H as i32, 0x21);
        assert!(fb.pixels().iter().all(|&p| p == 0x0F));
    }

    #[test]
    fn colour_index_is_masked_to_six_bits() {
        let mut fb = Framebuffer::new();
        fb.set(0, 0, 0xFF);
        assert_eq!(fb.pixels()[0], 0x3F);
    }

    #[test]
    fn rect_is_clipped_at_the_edges() {
        let mut fb = Framebuffer::new();
        fb.fill_rect(-4, -4, 8, 8, 0x30);
        let lit = fb.pixels().iter().filter(|&&p| p == 0x30).count();
        assert_eq!(lit, 16); // only the 4x4 corner that is on screen
    }

    #[test]
    fn value_zero_is_the_backdrop_in_every_palette() {
        let mut p = Palettes::new();
        p.backdrop = 0x2C;
        p.background[2] = [0x15, 0x05, 0x25];
        for palette in 0..4 {
            assert_eq!(p.background_color(palette, 0), 0x2C);
        }
        assert_eq!(p.background_color(2, 1), 0x15);
        assert_eq!(p.background_color(2, 3), 0x25);
    }

    #[test]
    fn palette_has_the_expected_anchors() {
        let pal = master_palette();
        assert_eq!(pal[0x0F], [0, 0, 0]); // black
        assert_eq!(pal[0x20], [255, 255, 255]); // white
        assert_eq!(pal[0x11], [19, 19, 191]); // medium blue
    }
}
