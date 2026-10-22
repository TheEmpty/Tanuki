//! Tanuki — an unofficial third-party WaniKani review notifier.

mod audio;
mod poller;
mod state;
mod tray;

#[cfg(target_os = "macos")]
mod menu;

use audio::ChimePlayer;
use chrono::Utc;
use notifier_core::theme::{Look, Theme};
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
    /// The look that is in use right now.
    pub look: Look,
}

/// A theme the user can pick, as it is shown in the dropdown.
#[derive(Debug, Clone, serde::Serialize)]
struct ThemeOption {
    /// The identifier that is stored in the settings.
    pub id: String,
    /// The name to show.
    pub name: String,
}

/// Returns every theme the user can pick.
#[tauri::command]
fn get_themes() -> Vec<ThemeOption> {
    Theme::ALL
        .iter()
        .map(|theme| ThemeOption {
            id: theme.id().to_owned(),
            name: theme.name().to_owned(),
        })
        .collect()
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
        look: shared
            .state
            .settings
            .lock()
            .expect("the settings lock should not be poisoned")
            .theme
            .resolve(Utc::now()),
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

/// Saves the user's settings and returns the look that is now in use.
#[tauri::command]
fn save_settings(
    app: AppHandle,
    shared: State<'_, Shared>,
    settings: notifier_core::Settings,
) -> Result<Look, SettingsError> {
    let (launch_at_login, quits, look) = {
        let mut current = shared
            .state
            .settings
            .lock()
            .expect("the settings lock should not be poisoned");
        *current = settings;
        (
            current.launch_at_login,
            current.quit_with_command_q,
            current.theme.resolve(Utc::now()),
        )
    };
    shared.state.save_settings()?;
    apply_autostart(&app, launch_at_login);
    update_command_q_item(&app, quits);
    Ok(look)
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
    let look = {
        let settings = shared
            .state
            .settings
            .lock()
            .expect("the settings lock should not be poisoned");
        settings.sound_look(Utc::now())
    };

    shared.audio.play(notifier_core::Sound::Chime(look));

    // macOS does not put a banner up for a notification from the app that is
    // in front; it goes straight into Notification Center. The window is put
    // away first so that the test looks like the real thing, which is a
    // banner in the corner of the screen.
    let handle = app.clone();
    std::thread::spawn(move || {
        hide_window(handle.clone());

        // Give macOS a moment to notice that the app is no longer in front.
        std::thread::sleep(std::time::Duration::from_millis(400));

        let result = handle
            .notification()
            .builder()
            .title("Tanuki")
            .body("This is what a review notification looks like.")
            .show();
        if let Err(error) = result {
            eprintln!("tanuki: could not show the test notification: {error}");
        }
    });
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

/// Creates the settings window.
///
/// The window is described in `tauri.conf.json`, but it is built here rather
/// than by Tauri, because Tauri can only be told to send cache headers for
/// a window it builds itself. The configuration says `create: false`, so
/// Tauri leaves the window alone and this builds it from that same
/// configuration.
fn create_main_window(app: &tauri::App) -> tauri::Result<()> {
    let Some(config) = app.config().app.windows.iter().find(|w| w.label == "main") else {
        return Ok(());
    };

    tauri::WebviewWindowBuilder::from_config(app.handle(), config)?
        .on_web_resource_request(|_request, response| {
            // The interface must never be cached. A user who updates the app
            // should not be shown the settings window they had before.
            response.headers_mut().insert(
                tauri::http::header::CACHE_CONTROL,
                tauri::http::HeaderValue::from_static("no-store"),
            );
        })
        .build()?;

    Ok(())
}

/// Starts Tanuki.
pub fn run() {
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            tray::show_settings(app);
        }))
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ));

    // The stock macOS menu quits the app on Command-Q. Tanuki brings its
    // own menu, where Command-Q puts the window away instead.
    #[cfg(target_os = "macos")]
    {
        builder = builder
            .menu(menu::build)
            .on_menu_event(|app, event| match event.id().as_ref() {
                menu::HIDE => command_q(app),
                menu::QUIT => app.exit(0),
                _ => {}
            });
    }

    builder
        .setup(|app| {
            // The settings live in the platform's config directory.
            let config_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&config_dir)?;

            // The window comes first, because everything else may want to
            // show it.
            create_main_window(app)?;

            // Build the menu bar icon first, so its status item can be part
            // of the shared state.
            let status_item = tray::build(app.handle())?;

            let credentials = notifier_core::TokenStore::in_dir(&config_dir);
            let state = Arc::new(AppState::load(&config_dir, credentials, status_item)?);
            let audio = Arc::new(ChimePlayer::new());

            // Keep the state around for the commands.
            app.manage(Shared {
                state: state.clone(),
                audio: audio.clone(),
            });

            // Make sure the login item matches the saved preference.
            let (launch_at_login, quits) = {
                let settings = state
                    .settings
                    .lock()
                    .expect("the settings lock should not be poisoned");
                (settings.launch_at_login, settings.quit_with_command_q)
            };
            apply_autostart(app.handle(), launch_at_login);
            update_command_q_item(app.handle(), quits);

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
            get_themes,
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

/// What Command-Q does.
///
/// By default it only puts the window away, because a notifier that stops
/// watching the review queue because of a stray keypress is no use at all.
/// The user can ask for it to quit instead, in which case it does.
fn command_q(app: &AppHandle) {
    if quits_with_command_q(app) {
        app.exit(0);
        return;
    }

    hide_window(app.clone());

    // Say so once, so the user knows where to go to quit.
    let Some(shared) = app.try_state::<Shared>() else {
        return;
    };
    if !shared.state.should_warn_about_command_q() {
        return;
    }
    shared.state.remember_command_q_warning();

    let result = app
        .notification()
        .builder()
        .title("Tanuki is still running")
        .body("You can quit Tanuki from its icon in the menu bar.")
        .show();
    if let Err(error) = result {
        eprintln!("tanuki: could not show the notification: {error}");
    }
}

/// Whether the user has asked for Command-Q to quit the app.
fn quits_with_command_q(app: &AppHandle) -> bool {
    app.try_state::<Shared>()
        .map(|shared| {
            shared
                .state
                .settings
                .lock()
                .expect("the settings lock should not be poisoned")
                .quit_with_command_q
        })
        .unwrap_or(false)
}

/// Keeps the Command-Q menu item in step with the setting, so it says what
/// it is going to do.
fn update_command_q_item(app: &AppHandle, quits: bool) {
    let Some(menu) = app.menu() else {
        return;
    };
    let Some(item) = menu
        .get(menu::HIDE)
        .and_then(|kind| kind.as_menuitem().cloned())
    else {
        return;
    };

    let text = if quits {
        "Quit Tanuki"
    } else {
        "Hide to the Menu Bar"
    };
    let _ = item.set_text(text);
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
