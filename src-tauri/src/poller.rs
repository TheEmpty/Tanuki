//! The background task that watches the WaniKani review queue.

use crate::audio::ChimePlayer;
use crate::state::AppState;
use chrono::Utc;
use notifier_core::api::WanikaniClient;
use notifier_core::detect::{next_poll_delay, Event};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Listener};
use tauri_plugin_notification::NotificationExt;

/// The event the app sends to its window whenever the status changes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Status {
    /// How many reviews are waiting.
    pub reviews: u32,
    /// When the next batch of reviews unlocks.
    pub next_review_at: Option<String>,
    /// When the last check happened.
    pub last_checked: Option<String>,
    /// The last error, if a check failed.
    pub error: Option<String>,
}

/// Starts the polling loop.
pub fn spawn(app: AppHandle, state: Arc<AppState>, audio: Arc<ChimePlayer>) {
    // The user can ask for a check at any time, so the loop waits on both a
    // timer and a message.
    let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<()>();

    app.listen("check-now", move |_| {
        let _ = sender.send(());
    });

    tauri::async_runtime::spawn(async move {
        // The client is kept between checks so its ETag cache can turn
        // repeated polls into cheap `304 Not Modified` responses.
        let mut client: Option<WanikaniClient> = None;

        loop {
            let delay = check(&app, &state, &audio, &mut client).await;

            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = receiver.recv() => {}
            }
        }
    });
}

/// Runs a single check and returns how long to wait before the next one.
async fn check(
    app: &AppHandle,
    state: &Arc<AppState>,
    audio: &Arc<ChimePlayer>,
    client: &mut Option<WanikaniClient>,
) -> Duration {
    let token = match state.token() {
        Ok(Some(token)) => token,
        Ok(None) => {
            // Without a token there is nothing to poll. Wait for the user to
            // add one rather than spinning.
            report(app, state, &Status {
                reviews: 0,
                next_review_at: None,
                last_checked: None,
                error: None,
            });
            return Duration::from_secs(60);
        }
        Err(error) => {
            report(app, state, &Status {
                reviews: 0,
                next_review_at: None,
                last_checked: Some(now_iso()),
                error: Some(error.to_string()),
            });
            return Duration::from_secs(60);
        }
    };

    // Rebuild the client only when the token changes, so the cache survives.
    if client.as_ref().is_none_or(|client| client.token() != token) {
        *client = Some(WanikaniClient::new(token));
    }
    let client = client.as_mut().expect("the client was just created");

    let reviews = match client.available_review_count().await {
        Ok(count) => count,
        Err(error) => {
            report(app, state, &Status {
                reviews: 0,
                next_review_at: None,
                last_checked: Some(now_iso()),
                error: Some(error.to_string()),
            });
            return Duration::from_secs(60);
        }
    };

    // The summary tells us when the next batch unlocks, so we can sleep
    // until then instead of guessing.
    let next_unlock = match client.summary().await {
        Ok(summary) => summary.data.next_review_unlock(Utc::now()),
        Err(_) => None,
    };

    let event = {
        let mut detector = state
            .detector
            .lock()
            .expect("the detector lock should not be poisoned");
        detector.observe(reviews)
    };
    if let Err(error) = state.save_detector() {
        eprintln!("tanuki: could not save the detector state: {error}");
    }

    let pending = state
        .detector
        .lock()
        .expect("the detector lock should not be poisoned")
        .is_pending();

    if let Event::BecameAvailable { count } = event {
        announce(app, state, audio, count);
    }

    report(app, state, &Status {
        reviews,
        next_review_at: next_unlock.map(|moment| moment.to_rfc3339()),
        last_checked: Some(now_iso()),
        error: None,
    });

    next_poll_delay(Utc::now(), pending, next_unlock)
}

/// Tells the user that reviews are ready.
fn announce(app: &AppHandle, state: &Arc<AppState>, audio: &Arc<ChimePlayer>, count: u32) {
    let settings = state
        .settings
        .lock()
        .expect("the settings lock should not be poisoned")
        .clone();

    if settings.sound {
        audio.play();
    }

    if settings.notification {
        let body = match count {
            1 => "You have a review waiting.".to_owned(),
            count => format!("You have {count} reviews waiting."),
        };
        let result = app
            .notification()
            .builder()
            .title("Your reviews are ready")
            .body(body)
            .show();
        if let Err(error) = result {
            eprintln!("tanuki: could not show the notification: {error}");
        }
    }
}

/// Sends the current status to the window and updates the tray.
fn report(app: &AppHandle, state: &Arc<AppState>, status: &Status) {
    if let Err(error) = app.emit("status", status) {
        eprintln!("tanuki: could not send the status to the window: {error}");
    }

    let text = match status.reviews {
        0 => "No reviews waiting".to_owned(),
        1 => "1 review waiting".to_owned(),
        count => format!("{count} reviews waiting"),
    };
    crate::tray::set_status(&state.status_item, &text);
}

/// The current time as an ISO 8601 string.
fn now_iso() -> String {
    Utc::now().to_rfc3339()
}
