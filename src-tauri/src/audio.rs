//! Playing the chime.
//!
//! Audio output is kept on a thread of its own so that a slow or missing
//! sound device can never hold up the rest of the app. The thread owns the
//! output stream and is told to play by sending it a message.

use notifier_core::chime::{self, Chime};
use rodio::buffer::SamplesBuffer;
use rodio::{ChannelCount, Player, SampleRate};
use std::sync::mpsc::{self, SyncSender, TrySendError};

/// Plays the chime on the default audio device.
#[derive(Debug)]
pub struct ChimePlayer {
    sender: SyncSender<()>,
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

    /// Plays the chime.
    ///
    /// Does nothing if the sound device is unavailable.
    pub fn play(&self) {
        // A pending chime is enough; a second one would just overlap.
        match self.sender.try_send(()) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {}
            Err(TrySendError::Disconnected(_)) => {}
        }
    }
}

/// Owns the output stream and plays the chime on request.
fn audio_thread(receiver: mpsc::Receiver<()>) {    let chime = chime::render();
    let source = to_source(&chime);

    let mut player: Option<Player> = None;

    while receiver.recv().is_ok() {
        if player.is_none() {
            let sink = match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(sink) => sink,
                Err(error) => {
                    eprintln!("tanuki: no audio output available: {error}");
                    // Try again the next time we are asked to play.
                    continue;
                }
            };
            player = Some(Player::connect_new(sink.mixer()));
        }

        let player = player.as_ref().expect("the player was just created");
        player.append(source.clone());
    }
}

/// Turns the rendered chime into something the audio device can play.
fn to_source(chime: &Chime) -> SamplesBuffer {
    let sample_rate = SampleRate::new(chime.sample_rate)
        .expect("the chime sample rate should be a valid non-zero rate");
    let channels = ChannelCount::new(1).expect("one channel is valid");

    SamplesBuffer::new(channels, sample_rate, chime.samples.clone())
}
