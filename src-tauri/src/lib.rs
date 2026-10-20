//! Tanuki — an unofficial third-party WaniKani review notifier.

mod audio;
mod poller;
mod state;
mod tray;

use audio::ChimePlayer;
use state::{AppState, SettingsError, Status};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_notification::NotificationExt;

/// The state the window and the background poller share.
struct Shared {
    state: Arc<AppState>,
    audio: Arc<ChimePlayer>,
}

/// Asks the poller to run a check right away.
fn trigger_check(app: &AppHandle) {
    // The poller sleeps between checks. This event wakes it up.
    let _ = app.emit("check-now", ());
}

/// The settings the window can change.
#[derive(Debug, Clone, serde::Serialize)]
struct WindowSettings {
    /// Whether a token has been saved.
    pub configured: bool,
    /// The user's settings.
    pub settings: notifier_core::Settings,
}

/// Returns whether a token is saved and the current settings.
#[tauri::command]
fn get_settings(shared: State<'_, Shared>) -> WindowSettings {
    let settings = shared
        .state
        .settings
        .lock()
        .expect("the settings lock should not be poisoned")
        .clone();

    WindowSettings {
        configured: shared
            .state
            .token()
            .map(|token| token.is_some())
            .unwrap_or(false),
        settings,
    }
}

/// Returns what the app last found out about the review queue.
///
/// The window asks for this when it opens, so it never shows "Checking…"
/// while it waits for the next poll.
#[tauri::command]
fn get_status(shared: State<'_, Shared>) -> Status {
    shared.state.status()
}

/// Saves the user's settings.
#[tauri::command]
fn save_settings(
    app: AppHandle,
    shared: State<'_, Shared>,
    settings: notifier_core::Settings,
) -> Result<(), SettingsError> {
    let launch_at_login = {
        let mut current = shared
            .state
            .settings
            .lock()
            .expect("the settings lock should not be poisoned");
        *current = settings;
        current.launch_at_login
    };
    shared.state.save_settings()?;
    apply_autostart(&app, launch_at_login);
    Ok(())
}

/// Stores the API token.
#[tauri::command]
fn save_token(app: AppHandle, shared: State<'_, Shared>, token: String) -> Result<(), String> {
    let token = token.trim().to_owned();
    if token.is_empty() {
        return Err("The API token cannot be empty.".to_owned());
    }

    shared
        .state
        .set_token(&token)
        .map_err(|error| error.to_string())?;

    // Poll straight away so the user sees the result of their change.
    trigger_check(&app);
    Ok(())
}

/// Removes the stored API token.
#[tauri::command]
fn clear_token(app: AppHandle, shared: State<'_, Shared>) -> Result<(), String> {
    shared
        .state
        .clear_token()
        .map_err(|error| error.to_string())?;
    trigger_check(&app);
    Ok(())
}

/// Plays the chime and shows a notification, so the user can check them.
#[tauri::command]
fn test_notification(app: AppHandle, shared: State<'_, Shared>) {
    shared.audio.play();
    let result = app
        .notification()
        .builder()
        .title("Tanuki")
        .body("This is what a review notification looks like.")
        .show();
    if let Err(error) = result {
        eprintln!("tanuki: could not show the test notification: {error}");
    }
}

/// Opens the page where WaniKani personal access tokens live.
#[tauri::command]
fn open_token_page(app: AppHandle) {
    let _ = tray::open_url(
        &app,
        "https://www.wanikani.com/settings/personal_access_tokens",
    );
}

/// Opens the WaniKani dashboard.
#[tauri::command]
fn open_wanikani(app: AppHandle) {
    let _ = tray::open_url(&app, "https://www.wanikani.com/dashboard");
}

/// Hides the settings window.
#[tauri::command]
fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
}

/// Starts Tanuki.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tray::show_settings(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            // The settings live in the platform's config directory.
            let config_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&config_dir)?;

            // Build the menu bar icon first, so its status item can be part
            // of the shared state.
            let status_item = tray::build(app.handle())?;

            let credentials = notifier_core::credential::default_store(&config_dir);
            let state = Arc::new(AppState::load(&config_dir, credentials, status_item)?);
            let audio = Arc::new(ChimePlayer::new());

            // Keep the state around for the commands.
            app.manage(Shared {
                state: state.clone(),
                audio: audio.clone(),
            });

            // Make sure the login item matches the saved preference.
            let launch_at_login = state
                .settings
                .lock()
                .expect("the settings lock should not be poisoned")
                .launch_at_login;
            apply_autostart(app.handle(), launch_at_login);

            // The end to end test provides a token on the command line, so
            // it does not have to go through the credential store.
            if let Ok(token) = std::env::var("TANUKI_TEST_TOKEN") {
                if let Err(error) = state.set_token(&token) {
                    eprintln!("tanuki: could not store the test token: {error}");
                }
            }

            // Without a token there is nothing to do, so show the window and
            // let the user get started.
            if state.token().map(|token| token.is_none()).unwrap_or(true) {
                tray::show_settings(app.handle());
            }

            // Start watching the review queue.
            poller::spawn(app.handle().clone(), state, audio);

            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window hides Tanuki; it keeps running in the tray.
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            get_status,
            save_settings,
            save_token,
            clear_token,
            test_notification,
            open_token_page,
            open_wanikani,
            hide_window,
        ])
        .run(tauri::generate_context!())
        .expect("the app should start");
}

/// Registers or removes the login item so Tanuki starts with the system.
fn apply_autostart(app: &AppHandle, enabled: bool) {
    use tauri_plugin_autostart::ManagerExt;

    let result = if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    };

    if let Err(error) = result {
        eprintln!("tanuki: could not change the login item: {error}");
    }
}
