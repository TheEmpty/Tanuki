//! The icon in the menu bar (macOS) or the notification area (Windows).

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};
use tauri_plugin_opener::OpenerExt;

/// The identifier of the tray icon.
pub const TRAY_ID: &str = "main";

/// The menu bar icon is a template image, so macOS recolours it for light and
/// dark menu bars. Windows gets the full colour mascot instead.
#[cfg(target_os = "macos")]
const TRAY_ICON: &[u8] = include_bytes!("../tray-icons/macos-template.png");

/// The full colour mascot, used for the Windows notification area.
#[cfg(not(target_os = "macos"))]
const TRAY_ICON: &[u8] = include_bytes!("../tray-icons/windows.png");

/// Builds the tray icon and its menu.
///
/// Returns the menu item that shows the current status, so it can be updated
/// later.
pub fn build(app: &AppHandle) -> tauri::Result<MenuItem<Wry>> {
    let icon = Image::from_bytes(TRAY_ICON)?;

    let status = MenuItem::with_id(app, "status", "No reviews waiting", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open WaniKani", true, None::<&str>)?;
    let check = MenuItem::with_id(app, "check", "Check now", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Tanuki", true, None::<&str>)?;

    let menu = Menu::with_items(
        app,
        &[
            &status,
            &PredefinedMenuItem::separator(app)?,
            &open,
            &check,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Tanuki — no reviews waiting")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => open_wanikani(app),
            "check" => crate::trigger_check(app),
            "settings" => show_settings(app),
            "quit" => app.exit(0),
            _ => {}
        });

    // Only macOS understands template images.
    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }

    let tray = builder.build(app)?;

    // Double clicking the icon opens the settings window.
    let handle = app.clone();
    tray.on_tray_icon_event(move |_, event| {
        if let TrayIconEvent::DoubleClick { .. } = event {
            show_settings(&handle);
        }
    });

    Ok(status)
}

/// Opens the WaniKani dashboard in the default browser.
fn open_wanikani(app: &AppHandle) {
    if let Err(error) = open_url(app, "https://www.wanikani.com/dashboard") {
        eprintln!("tanuki: could not open WaniKani: {error}");
    }
}

/// Shows the settings window and brings it to the front.
pub fn show_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Updates the status line in the tray menu.
pub fn set_status(status: &MenuItem<Wry>, text: &str) {
    let _ = status.set_text(text);
}

/// Opens a URL in the default browser.
pub fn open_url(app: &AppHandle, url: &str) -> Result<(), Box<dyn std::error::Error>> {
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| error.to_string().into())
}
