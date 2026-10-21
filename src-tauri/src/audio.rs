//! Playing the chime.
//!
//! Audio output is kept on a thread of its own so that a slow or missing
//! sound device can never hold up the rest of the app. The thread owns the
//! output stream and is told to play by sending it a message.

use notifier_core::chime::{self, Chime};
use notifier_core::theme::Look;
use rodio::buffer::SamplesBuffer;
use rodio::{ChannelCount, Player, SampleRate};
use std::sync::mpsc::{self, SyncSender, TrySendError};

/// Plays the chime on the default audio device.
#[derive(Debug)]
pub struct ChimePlayer {
    sender: SyncSender<Look>,
}

impl ChimePlayer {
    /// Starts the player.
    ///
    /// The audio device is opened lazily, so a machine without a working
    /// output device can still start Tanuki.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        std::thread::Builder::new()
            .name("tanuki-audio".to_owned())
            .spawn(move || audio_thread(receiver))
            .expect("the audio thread should start");
        Self { sender }
    }

    /// Plays the chime of a look.
    ///
    /// Does nothing if the sound device is unavailable.
    pub fn play(&self, look: Look) {
        // A pending chime is enough; a second one would just overlap.
        match self.sender.try_send(look) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {}
            Err(TrySendError::Disconnected(_)) => {}
        }
    }
}

/// Owns the output stream and plays the chime on request.
fn audio_thread(receiver: mpsc::Receiver<Look>) {
    // The chime of the look that was last asked for, so that a look that
    // rings the same way twice is only rendered once.
    let mut current: Option<(Look, SamplesBuffer)> = None;

    // The sink owns the stream that writes to the audio device, so it has to
    // be kept alive for as long as the player. Dropping it ends playback,
    // which is why it is held here and never read.
    let mut _sink: Option<rodio::MixerDeviceSink> = None;
    let mut player: Option<Player> = None;

    while let Ok(look) = receiver.recv() {
        if current.as_ref().map(|(played, _)| *played) != Some(look) {
            current = Some((look, to_source(&chime::render(look))));
        }

        let source = current
            .as_ref()
            .expect("the chime was just rendered")
            .1
            .clone();

        if player.is_none() {
            match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(device) => {
                    let new_player = Player::connect_new(device.mixer());
                    _sink = Some(device);
                    player = Some(new_player);
                }
                Err(error) => {
                    eprintln!("tanuki: no audio output available: {error}");
                    // Try again the next time we are asked to play.
                    continue;
                }
            }
        }

        let player = player.as_ref().expect("the player was just created");
        player.append(source);
    }
}

/// Turns the rendered chime into something the audio device can play.
fn to_source(chime: &Chime) -> SamplesBuffer {
    let sample_rate = SampleRate::new(chime.sample_rate)
        .expect("the chime sample rate should be a valid non-zero rate");
    let channels = ChannelCount::new(1).expect("one channel is valid");

    SamplesBuffer::new(channels, sample_rate, chime.samples.clone())
}
