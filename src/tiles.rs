//! Tile graphics: the console's "pattern table".
//!
//! Everything on screen is built from 8x8 tiles. A tile does not contain
//! colours. Each of its pixels is a 2-bit value (0-3) that says *which slot*
//! of a 4-colour palette to use, so one tile can be shown in different
//! colours just by pointing it at a different palette.

pub const TILE_SIZE: usize = 8;

/// One 8x8 tile, stored the way 8-bit hardware stored it: as two bitplanes.
/// Bytes 0-7 hold the low bit of every pixel (one byte per row, leftmost
/// pixel in the top bit) and bytes 8-15 hold the high bit. 16 bytes total.
#[derive(Clone, Copy)]
pub struct Tile {
    planes: [u8; 16],
}

impl Tile {
    pub const EMPTY: Tile = Tile { planes: [0; 16] };

    /// Build a tile from ASCII art: 8 rows of 8 characters, where `.` is
    /// value 0 and `1`, `2`, `3` are the three palette slots.
    pub fn from_art(rows: [&str; TILE_SIZE]) -> Tile {
        let mut planes = [0u8; 16];
        for (y, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), TILE_SIZE, "tile rows must be 8 characters");
            for (x, ch) in row.bytes().enumerate() {
                let value = match ch {
                    b'.' => 0,
                    b'1' => 1,
                    b'2' => 2,
                    b'3' => 3,
                    other => panic!("bad tile character {:?}", other as char),
                };
                let bit = 7 - x;
                planes[y] |= (value & 1) << bit;
                planes[y + 8] |= (value >> 1) << bit;
            }
        }
        Tile { planes }
    }

    /// The 2-bit value (0-3) of one pixel: recombine the two bitplanes.
    pub fn pixel(&self, x: usize, y: usize) -> u8 {
        let bit = 7 - x;
        let low = (self.planes[y] >> bit) & 1;
        let high = (self.planes[y + 8] >> bit) & 1;
        high << 1 | low
    }
}

/// The full set of tiles the console can draw from: 256 slots, so a tile is
/// named by a single byte.
pub struct PatternTable {
    tiles: [Tile; 256],
}

impl PatternTable {
    pub fn new() -> Self {
        Self {
            tiles: [Tile::EMPTY; 256],
        }
    }

    pub fn set(&mut self, index: u8, tile: Tile) {
        self.tiles[index as usize] = tile;
    }

    pub fn get(&self, index: u8) -> &Tile {
        &self.tiles[index as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ART: [&str; 8] = [
        "..112233", "........", "........", "........", "........", "........", "........",
        "3......1",
    ];

    #[test]
    fn art_round_trips_through_pixel() {
        let tile = Tile::from_art(ART);
        let top: Vec<u8> = (0..8).map(|x| tile.pixel(x, 0)).collect();
        assert_eq!(top, [0, 0, 1, 1, 2, 2, 3, 3]);
        assert_eq!(tile.pixel(0, 7), 3);
        assert_eq!(tile.pixel(7, 7), 1);
        assert_eq!(tile.pixel(3, 3), 0);
    }

    #[test]
    fn tile_is_stored_as_two_bitplanes() {
        let tile = Tile::from_art(ART);
        assert_eq!(tile.planes[0], 0b0011_0011); // low bits of row 0
        assert_eq!(tile.planes[8], 0b0000_1111); // high bits of row 0
    }
}
