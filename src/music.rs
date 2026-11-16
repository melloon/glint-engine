//! The demo level's background music, written out for the sound chip.
//! This is game data, like the drawings in `art.rs` and the effects in
//! `sfx.rs`.
//!
//! The tune uses four voices, matching the four tone generators a console
//! of the era had: two pulse waves (the lead and a quieter harmony), the
//! triangle (bass) and noise (drums). It is 16 bars long and loops.

use crate::synth::{Note, Wave, sequence};

/// The tune is written in eighth notes, each 12 ticks long. That makes a
/// beat 24 ticks, or 150 beats a minute, and a bar of four beats 96 ticks.
const EIGHTH: u32 = 12;
/// Drums are written twice as finely.
const SIXTEENTH: u32 = 6;

#[cfg(test)]
const BARS: u32 = 16;
#[cfg(test)]
const TICKS_PER_BAR: u32 = 8 * EIGHTH;

/// The lead. Eight steps to a bar; see `synth::sequence` for the notation.
const MELODY: &str = "
    E5 G5 C6 .  G5 .  E5 .  |  D5 E5 G5 .  .  .  -  .  |
    F5 A5 C6 .  A5 .  F5 .  |  G5 B5 D6 .  .  .  B5 G5 |
    E6 .  D6 C6 .  G5 .  E5 |  A5 .  C6 A5 .  E5 .  .  |
    F5 A5 C6 A5 G5 B5 D6 B5 |  C6 .  .  .  -  .  G5 .  |

    A5 .  E5 A5 .  C6 B5 A5 |  E5 .  .  .  -  E5 A5 B5 |
    C6 .  A5 C6 .  F6 E6 C6 |  A5 .  .  .  -  .  C6 .  |
    D6 .  A5 D6 .  F6 E6 D6 |  B5 .  D6 .  G5 .  B5 .  |
    C6 E6 G6 .  E6 .  C6 .  |  D6 .  B5 .  G5 A5 B5 D6 |
";

// The harmony and the bass repeat one bar-long pattern per chord, so each
// pattern is written once and the tune just lists which chord each bar uses.

/// Harmony: the notes of the chord played one at a time, up and down.
const ARP_C: &str = "E4 G4 C5 G4 E4 G4 C5 G4";
const ARP_F: &str = "F4 A4 C5 A4 F4 A4 C5 A4";
const ARP_G: &str = "G4 B4 D5 B4 G4 B4 D5 B4";
const ARP_AM: &str = "E4 A4 C5 A4 E4 A4 C5 A4";
const ARP_DM: &str = "F4 A4 D5 A4 F4 A4 D5 A4";
const ARP_FG: &str = "F4 A4 C5 A4 G4 B4 D5 B4"; // half a bar of each

const HARMONY: [&str; 16] = [
    ARP_C, ARP_C, ARP_F, ARP_G, ARP_C, ARP_AM, ARP_FG, ARP_C, //
    ARP_AM, ARP_AM, ARP_F, ARP_F, ARP_DM, ARP_G, ARP_C, ARP_G,
];

/// Bass: the chord's lowest note and the note five steps of the scale above
/// it, taking turns on the beat.
const BASS_C: &str = "C3 . G3 . C3 . G3 .";
const BASS_F: &str = "F3 . C4 . F3 . C4 .";
const BASS_G: &str = "G3 . D4 . G3 . D4 .";
const BASS_AM: &str = "A2 . E3 . A2 . E3 .";
const BASS_DM: &str = "D3 . A3 . D3 . A3 .";
const BASS_FG: &str = "F3 . C4 . G3 . D4 .";
// The last bar of each half walks up into the next one.
const BASS_C_END: &str = "C3 . G3 . C3 . E3 G3";
const BASS_G_END: &str = "G3 . D4 . G3 . B3 D4";

const BASS: [&str; 16] = [
    BASS_C, BASS_C, BASS_F, BASS_G, BASS_C, BASS_AM, BASS_FG, BASS_C_END, //
    BASS_AM, BASS_AM, BASS_F, BASS_F, BASS_DM, BASS_G, BASS_C, BASS_G_END,
];

/// Drums, sixteen steps to a bar: `k` kick, `s` snare, `h` hi-hat, `.`
/// nothing.
const BEAT: &str = "k.h.s.h.k.h.s.h.";
const FILL: &str = "k.h.s.h.k.s.ssss";

const DRUMS: [&str; 16] = [
    BEAT, BEAT, BEAT, BEAT, BEAT, BEAT, BEAT, FILL, //
    BEAT, BEAT, BEAT, BEAT, BEAT, BEAT, BEAT, FILL,
];

/// All three drums are the noise voice. Only the pitch and length differ:
/// slow noise is a thump, fast noise is a hiss.
fn drums(pattern: &str) -> Vec<Note> {
    let mut notes = Vec::new();
    for step in pattern.chars() {
        let (hz, ticks, volume) = match step {
            'k' => (700.0, 5, 10),
            's' => (5_000.0, 5, 7),
            'h' => (14_000.0, 2, 4),
            '.' => (0.0, 0, 0),
            other => panic!("unknown drum {other:?}"),
        };
        if ticks > 0 {
            notes.push(Note::tone(Wave::Noise, hz, ticks, volume).fade());
        }
        notes.push(Note::rest(SIXTEENTH - ticks));
    }
    notes
}

/// The level theme, one list of notes per voice. Mix them to hear it.
pub fn theme() -> Vec<Vec<Note>> {
    vec![
        sequence(Wave::Pulse(2), 6, EIGHTH, MELODY),
        sequence(Wave::Pulse(4), 3, EIGHTH, &HARMONY.join(" ")),
        sequence(Wave::Triangle, 11, EIGHTH, &BASS.join(" ")),
        drums(&DRUMS.concat()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{mix, render};

    #[test]
    fn every_voice_is_exactly_sixteen_bars() {
        // If one voice were a step short, it would drift out of time with
        // the others on every loop.
        for (index, voice) in theme().iter().enumerate() {
            let ticks: u32 = voice.iter().map(|note| note.ticks).sum();
            assert_eq!(ticks, BARS * TICKS_PER_BAR, "voice {index}");
        }
    }

    #[test]
    fn every_voice_plays_and_the_mix_has_headroom() {
        let voices: Vec<Vec<i16>> = theme().iter().map(|voice| render(voice)).collect();
        for (index, samples) in voices.iter().enumerate() {
            assert!(samples.iter().any(|&s| s != 0), "voice {index} is silent");
        }
        // All four voices together stay well inside the 16-bit range, so
        // sound effects can play over the top without distorting.
        let loudest = mix(&voices).iter().map(|s| s.unsigned_abs()).max().unwrap();
        assert!(loudest < 20_000, "peak is {loudest}");
    }
}
