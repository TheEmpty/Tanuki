//! The parts of Tanuki that do not depend on any user interface.
//!
//! Everything here is plain Rust: talking to the WaniKani API, deciding when
//! the user should be told that reviews are waiting, synthesising the chime
//! and holding the settings. That keeps the interesting logic testable
//! without a window or an audio device.

pub mod api;
pub mod chime;
pub mod detect;
pub mod level;
pub mod settings;
pub mod theme;
pub mod token;

pub use api::{ApiError, Summary, SummaryData, WanikaniClient};
pub use chime::{Chime, Sound};
pub use detect::{Detector, Event};
pub use level::LevelWatcher;
pub use settings::Settings;
pub use theme::Theme;
pub use token::{TokenError, TokenStore};
