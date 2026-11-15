//! The demo game's graphics: tile drawings and colour palettes.
//! This is game data, kept apart from the game logic in `demo.rs`.

use crate::tiles::{PatternTable, Tile};
use crate::video::Palettes;

// Background tile numbers. Tile 0 is left empty so a blank map shows the backdrop.
pub const EMPTY: u8 = 0;
pub const BLOCK: u8 = 1;
pub const GRASS: u8 = 2;
pub const DIRT: u8 = 3;
pub const CLOUD_L: u8 = 4;
pub const CLOUD_M: u8 = 5;
pub const CLOUD_R: u8 = 6;
pub const FLAG: u8 = 7;
pub const POLE: u8 = 8;

// Background palette numbers.
pub const PAL_CLOUD: u8 = 0;
pub const PAL_GROUND: u8 = 1;
pub const PAL_BRICK: u8 = 2;
pub const PAL_STONE: u8 = 3;

// Sprite tile numbers. Sprites have their own pattern table.
//
// The hero is 16x16, so each pose needs four 8x8 tiles. Every pose has the
// same head, though, so the two head tiles are stored once and shared: a
// standard trick for fitting more animation into a small tile budget.
pub const HERO_HEAD: u8 = 0; // two tiles: left, right
pub const HERO_LEGS: u8 = 2; // two tiles (left, right) per pose, poses in order
pub const GEM: u8 = 10;
pub const DIGIT: u8 = 11; // ten tiles: 0 to 9
pub const SLASH: u8 = 21;
// The enemy is symmetrical, so only its left half is stored; the right half
// is the same tiles drawn mirrored.
pub const ENEMY_TOP: u8 = 22;
pub const ENEMY_FEET: u8 = 23; // two tiles: the two walking poses

// Sprite palette numbers.
pub const PAL_HERO: u8 = 0;
pub const PAL_GEM: u8 = 1;
pub const PAL_ENEMY: u8 = 2;

/// The normal backdrop colour.
pub const SKY: u8 = 0x2C;

/// Body colours the Select button steps through.
pub const HERO_COLORS: [u8; 4] = [0x12, 0x15, 0x19, 0x16]; // purple, red, green, orange

pub fn palettes() -> Palettes {
    let mut p = Palettes::new();
    p.backdrop = SKY;
    //                                  slot 1 slot 2 slot 3
    p.background[PAL_CLOUD as usize] = [0x20, 0x3C, 0x10]; // white, pale blue, grey
    p.background[PAL_GROUND as usize] = [0x19, 0x06, 0x16]; // green, dark brown, earth
    p.background[PAL_BRICK as usize] = [0x15, 0x05, 0x25]; // red, dark red, light red
    p.background[PAL_STONE as usize] = [0x10, 0x00, 0x20]; // grey, dark grey, white

    p.sprite[PAL_HERO as usize] = [HERO_COLORS[0], 0x0F, 0x20]; // body, black, white
    p.sprite[PAL_GEM as usize] = [0x27, 0x06, 0x20]; // yellow, dark brown, white
    p.sprite[PAL_ENEMY as usize] = [0x1B, 0x0F, 0x20]; // teal, black, white
    p
}

#[rustfmt::skip]
pub fn background_tiles() -> PatternTable {
    let mut t = PatternTable::new();

    // 1 = face, 2 = shadow, 3 = highlight
    t.set(BLOCK, Tile::from_art([
        "33333332",
        "31111112",
        "31111112",
        "31111112",
        "31111112",
        "31111112",
        "31111112",
        "22222222",
    ]));

    // 1 = grass, 2 = dark speck, 3 = earth
    t.set(GRASS, Tile::from_art([
        "11111111",
        "11111111",
        "11211121",
        "13111311",
        "33313331",
        "33333333",
        "33233333",
        "33333323",
    ]));
    t.set(DIRT, Tile::from_art([
        "33333333",
        "33323333",
        "33333333",
        "33333332",
        "32333333",
        "33333333",
        "33333233",
        "33333333",
    ]));

    // 1 = white, 2 = pale blue underside
    t.set(CLOUD_L, Tile::from_art([
        "........",
        "....1111",
        "..111111",
        ".1111111",
        "11111111",
        "11111111",
        ".2222222",
        "........",
    ]));
    t.set(CLOUD_M, Tile::from_art([
        "..1111..",
        ".111111.",
        "11111111",
        "11111111",
        "11111111",
        "11111111",
        "22222222",
        "........",
    ]));
    t.set(CLOUD_R, Tile::from_art([
        "........",
        "1111....",
        "111111..",
        "1111111.",
        "11111111",
        "11111111",
        "2222222.",
        "........",
    ]));

    // The goal flag. 1 = pole, 3 = flag
    t.set(FLAG, Tile::from_art([
        "11......",
        "1133333.",
        "11333333",
        "1133333.",
        "11333...",
        "11......",
        "11......",
        "11......",
    ]));
    t.set(POLE, Tile::from_art(["11......"; 8]));

    t
}

#[rustfmt::skip]
pub fn sprite_tiles() -> PatternTable {
    let mut t = PatternTable::new();

    // The hero, drawn facing right. 1 = body, 2 = outline, 3 = eyes and shine.
    let head = [
        "......2222......",
        ".....211112.....",
        "....21111112....",
        "...2111111112...",
        "...2133113312...",
        "...2132113212...",
        "...2111111112...",
        "....21222212....",
    ];
    // One lower half per pose, in the order of the FRAME_ numbers in player.rs.
    let legs = [
        // 0: standing
        [
            ".....222222.....",
            "....21111112....",
            "...2113111112...",
            "..221111111122..",
            "..2.21111112.2..",
            "....21122112....",
            "....212..212....",
            "...2222..2222...",
        ],
        // 1: stride, legs apart and arms swinging
        [
            ".....222222.....",
            "....21111112....",
            "..22113111112...",
            ".2.211111111222.",
            "....21111112..2.",
            "...2111221112...",
            "..2112....2112..",
            "..222......2222.",
        ],
        // 2: passing, legs together
        [
            ".....222222.....",
            "....21111112....",
            "...2113111112...",
            "...2111111112...",
            "...2211111122...",
            "....21111112....",
            ".....212212.....",
            ".....2222222....",
        ],
        // 3: in the air, arms up
        [
            ".22..222222..22.",
            ".21221111112212.",
            "..222113111222..",
            "....21111112....",
            "....21111112....",
            "...2112222112...",
            "..2112....2112..",
            "..222......222..",
        ],
    ];

    // Cut each 16x8 drawing into its left and right 8x8 tiles.
    let mut cut = |first_tile: u8, art: &[&str; 8]| {
        for half in 0..2 {
            let rows = std::array::from_fn(|y| &art[y][half * 8..half * 8 + 8]);
            t.set(first_tile + half as u8, Tile::from_art(rows));
        }
    };
    cut(HERO_HEAD, &head);
    for (pose, art) in legs.iter().enumerate() {
        cut(HERO_LEGS + pose as u8 * 2, art);
    }

    // 1 = gold, 2 = outline, 3 = shine
    t.set(GEM, Tile::from_art([
        "...22...",
        "..2312..",
        ".231112.",
        "23111112",
        "21111112",
        ".211112.",
        "..2112..",
        "...22...",
    ]));

    // The enemy's left half. 1 = body, 2 = outline, 3 = eye.
    t.set(ENEMY_TOP, Tile::from_art([
        "........",
        ".....222",
        "...22111",
        "..212111",
        ".2111211",
        ".2113311",
        "21113211",
        "21111111",
    ]));
    t.set(ENEMY_FEET, Tile::from_art([
        "21111111",
        "21111122",
        "21111111",
        ".2111111",
        "..222222",
        "..2112..",
        ".21112..",
        ".2222...",
    ]));
    t.set(ENEMY_FEET + 1, Tile::from_art([
        "21111111",
        "21111122",
        "21111111",
        ".2111111",
        "..222222",
        "...2112.",
        "...21112",
        "...22222",
    ]));

    // The counter's characters: 0-9, then a slash.
    let font = [
        [".###....", "#...#...", "#..##...", "#.#.#...", "##..#...", "#...#...", ".###....", "........"],
        ["..#.....", ".##.....", "..#.....", "..#.....", "..#.....", "..#.....", ".###....", "........"],
        [".###....", "#...#...", "....#...", "...#....", "..#.....", ".#......", "#####...", "........"],
        [".###....", "#...#...", "....#...", "..##....", "....#...", "#...#...", ".###....", "........"],
        ["...#....", "..##....", ".#.#....", "#..#....", "#####...", "...#....", "...#....", "........"],
        ["#####...", "#.......", "####....", "....#...", "....#...", "#...#...", ".###....", "........"],
        ["..##....", ".#......", "#.......", "####....", "#...#...", "#...#...", ".###....", "........"],
        ["#####...", "....#...", "...#....", "..#.....", "..#.....", "..#.....", "..#.....", "........"],
        [".###....", "#...#...", "#...#...", ".###....", "#...#...", "#...#...", ".###....", "........"],
        [".###....", "#...#...", "#...#...", ".####...", "....#...", "...#....", ".##.....", "........"],
        ["....#...", "....#...", "...#....", "..#.....", "..#.....", ".#......", ".#......", "........"],
    ];
    for (i, rows) in font.iter().enumerate() {
        t.set(DIGIT + i as u8, glyph(rows));
    }

    t
}

/// Turn a one-colour character drawing (`#` = on) into a tile: strokes in
/// slot 3, plus a drop shadow in slot 2 one pixel down and to the right, so
/// the character stays readable over any background.
fn glyph(rows: &[&str; 8]) -> Tile {
    let on = |x: usize, y: usize| rows[y].as_bytes()[x] == b'#';
    let art: [String; 8] = std::array::from_fn(|y| {
        (0..8)
            .map(|x| {
                if on(x, y) {
                    '3'
                } else if x > 0 && y > 0 && on(x - 1, y - 1) {
                    '2'
                } else {
                    '.'
                }
            })
            .collect()
    });
    Tile::from_art(std::array::from_fn(|y| art[y].as_str()))
}
