//! The "speaker": takes notes from the sound chip and plays them through the
//! host's audio. This is the only file that touches macroquad's audio.

use macroquad::audio::{
    PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume,
};

use crate::synth::{self, Note};

/// Loudness of sound effects, from 0 to 1.
const VOLUME: f32 = 0.5;
/// Loudness of the music. Lower, so effects are heard clearly over it.
const MUSIC_VOLUME: f32 = 0.25;

pub struct Speaker {
    /// One ready-to-play sound per effect; `None` if it could not be loaded.
    sounds: Vec<Option<Sound>>,
    /// The background music, once started.
    music: Option<Sound>,
    /// Whether the game wants the music heard right now.
    music_playing: bool,
    muted: bool,
}

impl Speaker {
    /// Run every effect through the chip once, up front, and keep the
    /// results. `effects[i]` is then played with `play(i)`.
    pub async fn new(effects: &[Vec<Note>]) -> Self {
        let mut sounds = Vec::new();
        for notes in effects {
            sounds.push(load(&synth::render(notes)).await);
        }
        Self { sounds, music: None, music_playing: true, muted: false }
    }

    pub fn play(&self, index: usize) {
        if self.muted {
            return;
        }
        if let Some(Some(sound)) = self.sounds.get(index) {
            play_sound(sound, PlaySoundParams { looped: false, volume: VOLUME });
        }
    }

    /// Start a piece of music, given as one list of notes per voice. The
    /// voices are mixed into a single stream that repeats until the program
    /// ends.
    pub async fn start_music(&mut self, voices: &[Vec<Note>]) {
        let rendered: Vec<Vec<i16>> = voices.iter().map(|voice| synth::render(voice)).collect();
        self.music = load(&synth::mix(&rendered)).await;
        if let Some(music) = &self.music {
            play_sound(music, PlaySoundParams { looped: true, volume: self.music_volume() });
        }
    }

    /// Silence or restore the music, for example while the game is paused.
    /// The tune keeps its place and carries on underneath.
    pub fn set_music_playing(&mut self, playing: bool) {
        if playing != self.music_playing {
            self.music_playing = playing;
            self.apply_music_volume();
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        self.apply_music_volume();
    }

    fn music_volume(&self) -> f32 {
        if self.muted || !self.music_playing { 0.0 } else { MUSIC_VOLUME }
    }

    fn apply_music_volume(&self) {
        if let Some(music) = &self.music {
            set_sound_volume(music, self.music_volume());
        }
    }
}

/// Hand samples to the host's audio as a sound it can play.
async fn load(samples: &[i16]) -> Option<Sound> {
    match load_sound_from_bytes(&synth::wav_bytes(samples)).await {
        Ok(sound) => Some(sound),
        Err(error) => {
            eprintln!("sound unavailable: {error:?}");
            None
        }
    }
}
