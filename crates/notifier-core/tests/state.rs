//! Tests for the settings and detector state on disk.

use notifier_core::detect::Detector;
use notifier_core::settings::Settings;
use std::path::{Path, PathBuf};

/// A temporary directory that removes itself.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "tanuki-state-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("the test directory should be created");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_files_fall_back_to_the_defaults() {
    let dir = TempDir::new("defaults");
    let settings: Option<Settings> = None;
    let detector: Option<Detector> = None;
    assert_eq!(settings.unwrap_or_default(), Settings::default());
    assert_eq!(detector.unwrap_or_default(), Detector::new());
    let _ = dir;
}

#[test]
fn the_detector_state_survives_being_written_and_read() {
    let dir = TempDir::new("detector");
    let path = dir.path().join("state.json");

    let mut detector = Detector::new();
    detector.observe(5);
    assert!(detector.is_pending());

    std::fs::write(&path, serde_json::to_vec(&detector).unwrap()).unwrap();

    let mut restored: Detector = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert!(restored.is_pending());
    // The restored detector does not announce the same reviews again.
    assert_eq!(
        restored.observe(5),
        notifier_core::detect::Event::StillPending { count: 5 }
    );
}

#[test]
fn the_settings_survive_being_written_and_read() {
    let dir = TempDir::new("settings");
    let path = dir.path().join("settings.json");

    let settings = Settings {
        sound: false,
        notification: true,
        launch_at_login: true,
        notify_lessons: true,
    };
    std::fs::write(&path, serde_json::to_vec_pretty(&settings).unwrap()).unwrap();

    let restored: Settings = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(settings, restored);
}
