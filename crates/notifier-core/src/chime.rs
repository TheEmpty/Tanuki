//! The little chime Tanuki plays when reviews become available.
//!
//! The sound is synthesised rather than loaded from a file, which keeps the
//! binary small and lets the tone be tuned in code.
//!
//! Each look has its own voice. The plain one is a two-note bell: a bright
//! upper note followed by a lower one, each built from a handful of
//! inharmonic partials the way a struck bell is. Halloween is a slow,
//! sour-sounding fall. Christmas is a little sleigh-bell jingle.

use crate::theme::Look;
use std::f32::consts::TAU;

/// The sample rate the chime is generated at.
pub const SAMPLE_RATE: u32 = 44_100;

/// The peak amplitude of the finished chime.
const TARGET_PEAK: f32 = 0.55;

/// A note of the chime.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Note {
    /// The fundamental frequency in hertz when the note starts.
    frequency: f32,
    /// The fundamental frequency in hertz when the note ends. The same as
    /// `frequency` for a note that holds still.
    end_frequency: f32,
    /// When the note starts, in seconds from the beginning of the chime.
    offset: f32,
    /// How long the note rings for, in seconds.
    duration: f32,
    /// The overall volume of this note.
    gain: f32,
    /// How long the note takes to reach its full volume. A slow attack is
    /// what makes a sound creep up on you.
    attack: f32,
}

impl Note {
    /// A note that holds the same pitch.
    const fn steady(frequency: f32, offset: f32, duration: f32, gain: f32) -> Note {
        Note {
            frequency,
            end_frequency: frequency,
            offset,
            duration,
            gain,
            attack: 0.004,
        }
    }

    /// A note that slides from one pitch down to another.
    const fn falling(
        from: f32,
        to: f32,
        offset: f32,
        duration: f32,
        gain: f32,
        attack: f32,
    ) -> Note {
        Note {
            frequency: from,
            end_frequency: to,
            offset,
            duration,
            gain,
            attack,
        }
    }
}

/// A partial of a tone: frequency ratio, amplitude and how much faster the
/// partial decays than the fundamental.
type Partial = (f32, f32, f32);

/// The partials of a struck bell: inharmonic, and the higher ones fade away
/// faster than the fundamental.
const BELL_PARTIALS: [Partial; 5] = [
    (1.0, 1.00, 1.0),
    (2.0, 0.45, 1.5),
    (2.4, 0.30, 1.8),
    (3.0, 0.22, 2.2),
    (4.5, 0.12, 3.2),
];

/// The partials of a plain organ-like tone, with no bell ring in it. They
/// fade slowly, so the note keeps swelling instead of dying away.
const SPOOKY_PARTIALS: [Partial; 4] = [
    (1.0, 1.00, 0.18),
    (2.0, 0.42, 0.22),
    (3.0, 0.20, 0.26),
    (4.0, 0.09, 0.30),
];

/// The two notes of the bell: a high "ding" and a low "dong".
const BELL: &[Note] = &[
    Note::steady(1_318.51, 0.0, 0.9, 1.0), // E6
    Note::steady(880.0, 0.16, 1.3, 0.85),  // A5
];

/// The Halloween voice: a sour, slow fall, like something moving in the dark.
///
/// The notes creep up over most of a second and slide a major second down,
/// which is the sound every haunted house is made of.
const SPOOKY: &[Note] = &[
    // A major second below itself, sliding down. The sourness is the point.
    Note::falling(174.61, 164.81, 0.0, 2.0, 1.0, 0.70), // F3 to E3
    Note::falling(349.23, 329.63, 0.15, 1.7, 0.50, 0.55), // F4 to E4
    // A thin wail on top of it.
    Note::falling(698.46, 622.25, 0.55, 1.3, 0.20, 0.50), // F5 to Eb5
];

/// The Christmas voice: four notes of sleigh bell, going up.
const JINGLE: &[Note] = &[
    Note::steady(523.25, 0.00, 0.40, 0.85), // C5
    Note::steady(659.25, 0.13, 0.40, 0.90), // E5
    Note::steady(783.99, 0.26, 0.40, 0.95), // G5
    Note::steady(1046.50, 0.39, 1.05, 1.0), // C6
];

/// A voice: the notes it rings and the tone they are made of.
struct Voice {
    /// The notes.
    notes: &'static [Note],
    /// The partials of the tone.
    partials: &'static [Partial],
    /// How long the whole thing lasts.
    duration: f32,
}

/// The voice a look rings with.
fn voice(look: Look) -> Voice {
    match look {
        Look::Default => Voice {
            notes: BELL,
            partials: &BELL_PARTIALS,
            duration: 1.6,
        },
        Look::Halloween => Voice {
            notes: SPOOKY,
            partials: &SPOOKY_PARTIALS,
            duration: 2.4,
        },
        Look::Christmas => Voice {
            notes: JINGLE,
            partials: &BELL_PARTIALS,
            duration: 1.8,
        },
    }
}

/// A rendered sound, ready to be handed to an audio device.
#[derive(Debug, Clone, PartialEq)]
pub struct Chime {
    /// The sample rate of the samples.
    pub sample_rate: u32,
    /// Interleaved-free mono samples in the range `-1.0..=1.0`.
    pub samples: Vec<f32>,
}

/// Renders the chime of a look.
pub fn render(look: Look) -> Chime {
    let voice = voice(look);
    let total = (voice.duration * SAMPLE_RATE as f32) as usize;
    let mut samples = vec![0.0f32; total];

    for note in voice.notes {
        render_note(note, voice.partials, &mut samples);
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
fn render_note(note: &Note, partials: &[Partial], samples: &mut [f32]) {
    let rate = SAMPLE_RATE as f32;
    let start = (note.offset * rate) as usize;
    let length = (note.duration * rate) as usize;

    for (index, sample) in samples.iter_mut().enumerate().skip(start) {
        let elapsed = index as f32 - start as f32;
        if elapsed >= length as f32 {
            break;
        }
        let time = elapsed / rate;

        // The pitch slides from one note to the other over the length of the
        // note, so the phase has to follow the pitch rather than the time.
        // Sweeping the frequency gives the integral of the pitch, which is
        // what keeps the tone from clicking as it slides.
        let swept = note.frequency * time
            + 0.5 * (note.end_frequency - note.frequency) / note.duration * time * time;

        let mut value = 0.0;
        for (ratio, amplitude, decay) in partials {
            let envelope = (-time * (6.0 * decay)).exp();
            value += amplitude * envelope * (TAU * ratio * swept).sin();
        }

        // A short attack ramp avoids the click a hard start would produce.
        // A long one is how a note creeps up on you.
        let attack = (elapsed / (note.attack * rate)).min(1.0);
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
        let chime = render(Look::Default);
        let expected = (1.6 * SAMPLE_RATE as f32) as usize;
        assert_eq!(chime.samples.len(), expected);
        assert_eq!(chime.sample_rate, SAMPLE_RATE);
    }

    #[test]
    fn the_chime_stays_within_the_normal_range() {
        for look in Look::ALL {
            let chime = render(look);
            assert!(
                chime.samples.iter().all(|sample| sample.abs() <= 1.0),
                "the {look:?} samples must not clip"
            );
        }
    }

    #[test]
    fn the_chime_actually_makes_sound() {
        for look in Look::ALL {
            let chime = render(look);
            let loudest = chime.samples.iter().fold(0.0f32, |acc, v| acc.max(v.abs()));
            assert!(loudest > 0.1, "the {look:?} chime should not be silent");
        }
    }

    #[test]
    fn the_chime_fades_out_at_the_end() {
        for look in Look::ALL {
            let chime = render(look);
            let tail = &chime.samples[chime.samples.len() - 100..];
            assert!(tail.iter().all(|sample| sample.abs() < 0.01));
        }
    }

    #[test]
    fn the_chime_starts_quietly() {
        for look in Look::ALL {
            let chime = render(look);
            assert!(chime.samples[0].abs() < 0.05);
        }
    }

    #[test]
    fn every_look_sounds_different() {
        let plain = render(Look::Default);
        let halloween = render(Look::Halloween);
        let christmas = render(Look::Christmas);

        assert_ne!(plain.samples, halloween.samples);
        assert_ne!(plain.samples, christmas.samples);
        assert_ne!(halloween.samples, christmas.samples);
    }

    #[test]
    fn the_halloween_chime_creeps_up_on_you() {
        // The slow attack means the first sound is barely there.
        let chime = render(Look::Halloween);
        let loudest = chime.samples.iter().fold(0.0f32, |acc, v| acc.max(v.abs()));
        let early = chime.samples[..(0.05 * SAMPLE_RATE as f32) as usize]
            .iter()
            .fold(0.0f32, |acc, v| acc.max(v.abs()));

        assert!(
            early < loudest * 0.25,
            "the spooky chime should start very quietly"
        );
    }

    #[test]
    fn the_second_note_follows_the_first() {
        let chime = render(Look::Default);
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

    #[test]
    fn the_christmas_jingle_goes_up() {
        // Each note of the jingle is higher than the one before it.
        let notes: Vec<f32> = JINGLE.iter().map(|note| note.frequency).collect();
        for pair in notes.windows(2) {
            assert!(pair[1] > pair[0], "the jingle should climb");
        }
    }
}
