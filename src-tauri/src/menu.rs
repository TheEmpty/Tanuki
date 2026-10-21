//! Tanuki's own application menu.
//!
//! Tanuki lives in the menu bar and has no menu of its own to show, so this
//! menu is never seen. It is here for its keyboard shortcuts, and the one
//! that matters is Command-Q.
//!
//! The stock macOS menu quits the app on Command-Q, and a notifier that
//! stops watching the review queue because of a stray keypress is no use at
//! all. So Tanuki brings its own menu, where Command-Q puts the window
//! away instead, and Command-Option-Q is the way out.

use tauri::menu::{AboutMetadata, Menu, MenuItemBuilder, SubmenuBuilder};
use tauri::{AppHandle, Wry};

/// The menu item that puts the window away.
pub const HIDE: &str = "hide-to-menu-bar";

/// The menu item that quits the app.
pub const QUIT: &str = "quit-tanuki";

/// Builds the application menu.
pub fn build(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let hide = MenuItemBuilder::with_id(HIDE, "Hide to the Menu Bar")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;

    let quit = MenuItemBuilder::with_id(QUIT, "Quit Tanuki")
        .accelerator("CmdOrCtrl+Alt+Q")
        .build(app)?;

    let application = SubmenuBuilder::new(app, "Tanuki")
        .about(Some(AboutMetadata {
            name: Some("Tanuki".to_owned()),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            copyright: Some(
                "Unofficial third-party tool. Not affiliated with WaniKani.".to_owned(),
            ),
            ..Default::default()
        }))
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .item(&hide)
        .separator()
        .item(&quit)
        .build()?;

    // The edit shortcuts are how a token gets pasted in, so they have to be
    // there even though the menu is never shown.
    let edit = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let window = SubmenuBuilder::new(app, "Window")
        .minimize()
        .close_window()
        .build()?;

    Menu::with_items(app, &[&application, &edit, &window])
}
