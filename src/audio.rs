//! The "speaker": takes notes from the sound chip and plays them through the
//! host's audio. This is the only file that touches macroquad's audio.

use macroquad::audio::{PlaySoundParams, Sound, load_sound_from_bytes, play_sound};

use crate::synth::{self, Note};

/// Overall loudness, from 0 to 1.
const VOLUME: f32 = 0.5;

pub struct Speaker {
    /// One ready-to-play sound per effect; `None` if it could not be loaded.
    sounds: Vec<Option<Sound>>,
    muted: bool,
}

impl Speaker {
    /// Run every effect through the chip once, up front, and keep the
    /// results. `effects[i]` is then played with `play(i)`.
    pub async fn new(effects: &[Vec<Note>]) -> Self {
        let mut sounds = Vec::new();
        for notes in effects {
            let wav = synth::wav_bytes(&synth::render(notes));
            match load_sound_from_bytes(&wav).await {
                Ok(sound) => sounds.push(Some(sound)),
                Err(error) => {
                    eprintln!("sound unavailable: {error:?}");
                    sounds.push(None);
                }
            }
        }
        Self { sounds, muted: false }
    }

    pub fn play(&self, index: usize) {
        if self.muted {
            return;
        }
        if let Some(Some(sound)) = self.sounds.get(index) {
            play_sound(sound, PlaySoundParams { looped: false, volume: VOLUME });
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }
}
