//! The parts of Tanuki that do not depend on any user interface.
//!
//! Everything here is plain Rust: talking to the WaniKani API, deciding when
//! the user should be told that reviews are waiting, synthesising the chime
//! and holding the settings. That keeps the interesting logic testable
//! without a window or an audio device.

pub mod api;
pub mod chime;
pub mod credential;
pub mod detect;
pub mod settings;

pub use api::{ApiError, Summary, SummaryData, WanikaniClient};
pub use chime::Chime;
pub use credential::{CredentialError, CredentialStore};
pub use detect::{Detector, Event};
pub use settings::Settings;
