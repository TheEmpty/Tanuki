//! The little chime Tanuki plays when reviews become available.
//!
//! The sound is synthesised rather than loaded from a file, which keeps the
//! binary small and lets the tone be tuned in code. It is a two-note bell:
//! a bright upper note followed by a lower one, each built from a handful of
//! inharmonic partials the way a struck bell is.
//!
//! Each theme has its own voice, so a seasonal theme can ring differently
//! from the one the app launched with.

use crate::theme::Theme;
use std::f32::consts::TAU;

/// The sample rate the chime is generated at.
pub const SAMPLE_RATE: u32 = 44_100;

/// How long the whole chime lasts.
pub const DURATION_SECONDS: f32 = 1.6;

/// The peak amplitude of the finished chime.
const TARGET_PEAK: f32 = 0.55;

/// A note of the chime.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Note {
    /// The fundamental frequency in hertz.
    frequency: f32,
    /// When the note starts, in seconds from the beginning of the chime.
    offset: f32,
    /// How long the note rings for, in seconds.
    duration: f32,
    /// The overall volume of this note.
    gain: f32,
}

/// The partials of a struck bell: frequency ratio, amplitude and how much
/// faster the partial decays than the fundamental.
const PARTIALS: [(f32, f32, f32); 5] = [
    (1.0, 1.00, 1.0),
    (2.0, 0.45, 1.5),
    (2.4, 0.30, 1.8),
    (3.0, 0.22, 2.2),
    (4.5, 0.12, 3.2),
];

/// The two notes of the bell: a high "ding" and a low "dong".
const BELL: &[Note] = &[
    Note {
        frequency: 1_318.51, // E6
        offset: 0.0,
        duration: 0.9,
        gain: 1.0,
    },
    Note {
        frequency: 880.0, // A5
        offset: 0.16,
        duration: 1.3,
        gain: 0.85,
    },
];

/// A rendered sound, ready to be handed to an audio device.
#[derive(Debug, Clone, PartialEq)]
pub struct Chime {
    /// The sample rate of the samples.
    pub sample_rate: u32,
    /// Interleaved-free mono samples in the range `-1.0..=1.0`.
    pub samples: Vec<f32>,
}

/// The voice a theme rings with.
///
/// Both themes share the bell for now. A new theme brings its own notes
/// here, and nothing else in the app has to change.
fn voice(theme: Theme) -> &'static [Note] {
    match theme {
        Theme::Automatic => BELL,
        Theme::LaunchPurple => BELL,
    }
}

/// Renders the chime for a theme.
pub fn render(theme: Theme) -> Chime {
    let total = (DURATION_SECONDS * SAMPLE_RATE as f32) as usize;
    let mut samples = vec![0.0f32; total];

    for note in voice(theme) {
        render_note(note, &mut samples);
    }

    // Soften the very end so the sound never ends on a click.
    fade_out(&mut samples, (0.05 * SAMPLE_RATE as f32) as usize);

    normalise(&mut samples, TARGET_PEAK);
    Chime {
        sample_rate: SAMPLE_RATE,
        samples,
    }
}

/// Adds one note into the sample buffer.
fn render_note(note: &Note, samples: &mut [f32]) {
    let rate = SAMPLE_RATE as f32;
    let start = (note.offset * rate) as usize;
    let length = (note.duration * rate) as usize;

    for (index, sample) in samples.iter_mut().enumerate().skip(start) {
        let elapsed = index as f32 - start as f32;
        if elapsed >= length as f32 {
            break;
        }
        let time = elapsed / rate;
        let mut value = 0.0;
        for (ratio, amplitude, decay) in PARTIALS {
            let envelope = (-time * (6.0 * decay)).exp();
            value += amplitude * envelope * (TAU * note.frequency * ratio * time).sin();
        }

        // A short attack ramp avoids the click a hard start would produce.
        let attack = (elapsed / (0.004 * rate)).min(1.0);
        *sample += value * attack * note.gain / 2.0;
    }
}

/// Fades the last `length` samples out to silence.
fn fade_out(samples: &mut [f32], length: usize) {
    let length = length.min(samples.len());
    let start = samples.len() - length;
    for (offset, sample) in samples[start..].iter_mut().enumerate() {
        let position = offset as f32 / length as f32;
        *sample *= 1.0 - position;
    }
}

/// Scales the buffer so its loudest sample reaches `peak`.
fn normalise(samples: &mut [f32], peak: f32) {
    let loudest = samples
        .iter()
        .fold(0.0f32, |acc, value| acc.max(value.abs()));
    if loudest <= f32::EPSILON {
        return;
    }
    for sample in samples.iter_mut() {
        *sample = (*sample / loudest) * peak;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_chime_has_the_expected_length() {
        let chime = render(Theme::Automatic);
        let expected = (DURATION_SECONDS * SAMPLE_RATE as f32) as usize;
        assert_eq!(chime.samples.len(), expected);
        assert_eq!(chime.sample_rate, SAMPLE_RATE);
    }

    #[test]
    fn the_chime_stays_within_the_normal_range() {
        let chime = render(Theme::Automatic);
        assert!(
            chime.samples.iter().all(|sample| sample.abs() <= 1.0),
            "samples must not clip"
        );
    }

    #[test]
    fn the_chime_actually_makes_sound() {
        let chime = render(Theme::Automatic);
        let loudest = chime.samples.iter().fold(0.0f32, |acc, v| acc.max(v.abs()));
        assert!(loudest > 0.1, "the chime should not be silent");
    }

    #[test]
    fn the_chime_fades_out_at_the_end() {
        let chime = render(Theme::Automatic);
        let tail = &chime.samples[chime.samples.len() - 100..];
        assert!(tail.iter().all(|sample| sample.abs() < 0.01));
    }

    #[test]
    fn the_chime_starts_quietly() {
        let chime = render(Theme::Automatic);
        assert!(chime.samples[0].abs() < 0.05);
    }

    #[test]
    fn every_theme_rings_something() {
        for theme in Theme::ALL {
            let chime = render(theme);
            let loudest = chime.samples.iter().fold(0.0f32, |acc, v| acc.max(v.abs()));
            assert!(loudest > 0.1, "the {theme:?} theme should not be silent");
            assert_eq!(
                chime.samples.len(),
                (DURATION_SECONDS * SAMPLE_RATE as f32) as usize
            );
        }
    }

    #[test]
    fn the_second_note_follows_the_first() {
        let chime = render(Theme::Automatic);
        let rate = SAMPLE_RATE as f32;

        // Energy in the first note's window versus the gap just before the
        // second note arrives.
        let energy = |from: f32, to: f32| -> f32 {
            let (from, to) = (from as usize, to as usize);
            chime.samples[from..to]
                .iter()
                .map(|sample| sample * sample)
                .sum::<f32>()
        };

        let first = energy(0.0, 0.10 * rate);
        let before_second = energy(0.13 * rate, 0.16 * rate);
        assert!(first > before_second, "the second note should start later");
    }
}
