//! A tiny sound chip.
//!
//! 8-bit consoles had no recorded audio. They had a chip with a few simple
//! tone generators, and every sound was a list of instructions for them:
//! which wave shape, what pitch, how loud, for how long. This module is that
//! chip in software. It turns a list of notes into raw audio samples, and
//! knows nothing about speakers; `audio.rs` handles actually playing them.

pub const SAMPLE_RATE: u32 = 44_100;
/// Audio samples in one game tick (1/60 s). Note lengths are counted in
/// ticks, the same clock the rest of the engine runs on.
pub const SAMPLES_PER_TICK: usize = SAMPLE_RATE as usize / 60;

/// Loudest sample value at full volume. Well below the 16-bit maximum of
/// 32,767 so several sounds can overlap without distorting.
const AMPLITUDE: f32 = 8_000.0;

/// The wave shapes the chip can produce.
#[derive(Clone, Copy)]
pub enum Wave {
    /// A square wave. The number is how many eighths of each cycle it spends
    /// high: 4 is a hollow, even tone, while 1 or 2 sound thinner and buzzier.
    Pulse(u8),
    /// A staircase triangle: softer, good for bass. On the real chip this
    /// voice had no volume control, but it is allowed one here.
    Triangle,
    /// Random on/off, for hits and crashes. The pitch sets how fast the
    /// randomness changes: high is a hiss, low is a rumble.
    Noise,
}

#[derive(Clone, Copy)]
pub struct Note {
    pub wave: Wave,
    /// Pitch in Hz at the start and at the end. Different values slide.
    pub from_hz: f32,
    pub to_hz: f32,
    /// Length in ticks.
    pub ticks: u32,
    /// Loudness from 0 to 15: the chip's volume control had only 16 steps.
    pub volume: u8,
    /// Fade down to silence over the length of the note.
    pub fade: bool,
}

impl Note {
    /// A steady note.
    pub fn tone(wave: Wave, hz: f32, ticks: u32, volume: u8) -> Note {
        Note { wave, from_hz: hz, to_hz: hz, ticks, volume, fade: false }
    }

    /// A stretch of silence.
    pub fn rest(ticks: u32) -> Note {
        Note::tone(Wave::Pulse(4), 440.0, ticks, 0)
    }

    /// Slide the pitch to `hz` by the end of the note.
    pub fn slide(mut self, hz: f32) -> Note {
        self.to_hz = hz;
        self
    }

    /// Fade the note out.
    pub fn fade(mut self) -> Note {
        self.fade = true;
        self
    }
}

/// Play a list of notes one after another and return the audio samples.
pub fn render(notes: &[Note]) -> Vec<i16> {
    let mut samples = Vec::new();
    // How far through the current wave cycle we are, from 0 to 1.
    let mut phase = 0.0f32;
    // The noise generator: a 15-bit shift register, as on the real chip.
    let mut shift: u16 = 1;

    for note in notes {
        let length = note.ticks as usize * SAMPLES_PER_TICK;
        for i in 0..length {
            let progress = i as f32 / length as f32;
            let hz = note.from_hz + (note.to_hz - note.from_hz) * progress;

            // Volume moves in whole steps, which gives fades their slightly
            // gritty, stepped character.
            let step = if note.fade {
                (note.volume as f32 * (1.0 - progress)).ceil()
            } else {
                note.volume as f32
            };

            phase += hz / SAMPLE_RATE as f32;
            if phase >= 1.0 {
                phase -= 1.0;
                // Each completed cycle clocks the noise generator once.
                let bit = (shift ^ (shift >> 1)) & 1;
                shift = (shift >> 1) | (bit << 14);
            }

            let wave = match note.wave {
                Wave::Pulse(eighths) => {
                    if phase < eighths as f32 / 8.0 { 1.0 } else { -1.0 }
                }
                Wave::Triangle => {
                    // 32 steps per cycle: 16 up, then 16 back down.
                    let position = (phase * 32.0) as i32;
                    let height = if position < 16 { position } else { 31 - position };
                    height as f32 / 7.5 - 1.0
                }
                Wave::Noise => {
                    if shift & 1 == 1 { 1.0 } else { -1.0 }
                }
            };
            samples.push((wave * step / 15.0 * AMPLITUDE) as i16);
        }
    }
    samples
}

/// Wrap samples in a WAV file header (mono, 16-bit), so they can be handed
/// to anything that plays ordinary sound files.
pub fn wav_bytes(samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + samples.len() * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // size of the format block
    wav.extend_from_slice(&1u16.to_le_bytes()); // uncompressed
    wav.extend_from_slice(&1u16.to_le_bytes()); // one channel
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes()); // bytes per second
    wav.extend_from_slice(&2u16.to_le_bytes()); // bytes per sample
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How many times the wave swings from negative to positive.
    fn cycles(samples: &[i16]) -> usize {
        samples.windows(2).filter(|pair| pair[0] < 0 && pair[1] >= 0).count()
    }

    fn loudest(samples: &[i16]) -> i16 {
        samples.iter().map(|s| s.abs()).max().unwrap_or(0)
    }

    #[test]
    fn length_is_counted_in_ticks() {
        let samples = render(&[Note::tone(Wave::Pulse(4), 440.0, 10, 15), Note::rest(5)]);
        assert_eq!(samples.len(), 15 * SAMPLES_PER_TICK);
        assert!(samples[10 * SAMPLES_PER_TICK..].iter().all(|&s| s == 0)); // the rest is silent
    }

    #[test]
    fn pitch_is_the_number_of_cycles_per_second() {
        let one_second = render(&[Note::tone(Wave::Pulse(4), 441.0, 60, 15)]);
        assert!((440..=442).contains(&cycles(&one_second)));
    }

    #[test]
    fn pulse_width_is_set_in_eighths() {
        let samples = render(&[Note::tone(Wave::Pulse(2), 441.0, 60, 15)]);
        let high = samples.iter().filter(|&&s| s > 0).count() as f32 / samples.len() as f32;
        assert!((high - 0.25).abs() < 0.01, "high for {high} of the time");
    }

    #[test]
    fn volume_scales_and_fades() {
        let full = render(&[Note::tone(Wave::Pulse(4), 440.0, 6, 15)]);
        let third = render(&[Note::tone(Wave::Pulse(4), 440.0, 6, 5)]);
        assert_eq!(loudest(&full), AMPLITUDE as i16);
        assert_eq!(loudest(&third), (AMPLITUDE / 3.0) as i16);

        let fading = render(&[Note::tone(Wave::Pulse(4), 440.0, 30, 15).fade()]);
        let (start, end) = (&fading[..1000], &fading[fading.len() - 1000..]);
        assert_eq!(loudest(start), AMPLITUDE as i16);
        assert!(loudest(end) <= (AMPLITUDE / 15.0) as i16 + 1); // down to the last step
    }

    #[test]
    fn slide_changes_pitch_through_the_note() {
        let samples = render(&[Note::tone(Wave::Pulse(4), 200.0, 60, 15).slide(800.0)]);
        let (first, second) = samples.split_at(samples.len() / 2);
        assert!(cycles(second) > cycles(first) * 3 / 2);
    }

    #[test]
    fn triangle_is_a_staircase_of_sixteen_levels() {
        let samples = render(&[Note::tone(Wave::Triangle, 110.0, 30, 15)]);
        let mut levels = samples.clone();
        levels.sort();
        levels.dedup();
        assert_eq!(levels.len(), 16);
    }

    #[test]
    fn noise_does_not_repeat_like_a_tone() {
        let samples = render(&[Note::tone(Wave::Noise, 4000.0, 30, 15)]);
        // Lengths of runs spent high or low. A tone has one or two lengths;
        // noise has many.
        let mut runs = Vec::new();
        let mut run = 0;
        for pair in samples.windows(2) {
            run += 1;
            if (pair[0] > 0) != (pair[1] > 0) {
                runs.push(run);
                run = 0;
            }
        }
        runs.sort();
        runs.dedup();
        assert!(runs.len() > 5, "only {} distinct run lengths", runs.len());
    }

    #[test]
    fn wav_header_describes_the_samples() {
        let wav = wav_bytes(&[1, -2, 3]);
        assert_eq!(wav.len(), 44 + 6);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 44_100);
        assert_eq!(u32::from_le_bytes(wav[40..44].try_into().unwrap()), 6);
        assert_eq!(&wav[44..], [1, 0, 0xFE, 0xFF, 3, 0]);
    }
}
