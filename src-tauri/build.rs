fn main() {
    watch_the_interface();
    tauri_build::build()
}

/// Tells Cargo to re-run the build when the interface changes.
///
/// `tauri-build` asks Cargo to watch the directory the interface lives in,
/// but Cargo only notices a change to the directory itself and not to the
/// files inside it. An edited page or stylesheet would be quietly ignored
/// and the old one built into the app, which is a confusing thing to run
/// into. Watching the files themselves avoids it.
fn watch_the_interface() {
    let Ok(entries) = std::fs::read_dir("../ui") else {
        // Nothing to watch; the build will say so if it matters.
        return;
    };

    for entry in entries.flatten() {
        println!("cargo:rerun-if-changed={}", entry.path().display());
    }
}
