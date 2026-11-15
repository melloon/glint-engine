//! The demo game's sound effects, written as notes for the sound chip.
//! This is game data, like the drawings in `art.rs`.

use crate::synth::{Note, Wave};

// Pitches in Hz. The number is the octave; C5 is one octave above middle C.
const C5: f32 = 523.25;
const E5: f32 = 659.25;
const G5: f32 = 783.99;
const A5: f32 = 880.00;
const C6: f32 = 1046.50;
const E6: f32 = 1318.51;

/// Every sound the game can make. The game asks for these by name and never
/// touches audio itself.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Sfx {
    Jump,
    Gem,
    Stomp,
    Hurt,
    Flag,
    Pause,
}

impl Sfx {
    /// All effects, in the same order as the enum.
    pub const ALL: [Sfx; 6] = [Sfx::Jump, Sfx::Gem, Sfx::Stomp, Sfx::Hurt, Sfx::Flag, Sfx::Pause];

    pub fn notes(self) -> Vec<Note> {
        let square = Wave::Pulse(4);
        match self {
            // A quick upward sweep on a thin pulse.
            Sfx::Jump => vec![Note::tone(Wave::Pulse(2), 280.0, 9, 9).slide(620.0).fade()],
            // Three rising notes, the last one ringing out.
            Sfx::Gem => vec![
                Note::tone(square, G5, 3, 10),
                Note::tone(square, C6, 3, 10),
                Note::tone(square, E6, 9, 10).fade(),
            ],
            // A burst of noise for the impact, then a soft falling thud.
            Sfx::Stomp => vec![
                Note::tone(Wave::Noise, 2000.0, 2, 12),
                Note::tone(Wave::Triangle, 420.0, 7, 15).slide(140.0).fade(),
            ],
            // A long downward slide.
            Sfx::Hurt => vec![
                Note::tone(square, 520.0, 6, 11).slide(330.0),
                Note::tone(square, 330.0, 16, 11).slide(110.0).fade(),
            ],
            // A short fanfare up the chord and back to its top note.
            Sfx::Flag => vec![
                Note::tone(square, C5, 5, 10),
                Note::tone(square, E5, 5, 10),
                Note::tone(square, G5, 5, 10),
                Note::tone(square, C6, 5, 10),
                Note::tone(square, G5, 5, 10),
                Note::tone(square, C6, 20, 10).fade(),
            ],
            // Two short blips.
            Sfx::Pause => vec![
                Note::tone(square, A5, 3, 8),
                Note::rest(2),
                Note::tone(square, A5, 3, 8),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::{SAMPLE_RATE, render};

    #[test]
    fn every_effect_is_audible_and_short() {
        for (index, sfx) in Sfx::ALL.into_iter().enumerate() {
            assert_eq!(sfx as usize, index); // ALL is in enum order
            let samples = render(&sfx.notes());
            assert!(samples.iter().any(|&s| s != 0), "{sfx:?} is silent");
            assert!(samples.len() < SAMPLE_RATE as usize, "{sfx:?} is over a second long");
        }
    }
}
