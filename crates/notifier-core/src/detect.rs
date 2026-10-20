//! Turning a stream of review counts into "something changed" events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// What the detector noticed between two consecutive polls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Reviews were waiting before and are waiting now.
    StillPending {
        /// How many reviews are waiting.
        count: u32,
    },
    /// Nothing is waiting, and nothing was waiting before.
    Idle,
    /// Reviews were waiting before, but the queue has been cleared.
    Cleared,
    /// Reviews just became available.
    BecameAvailable {
        /// How many reviews are waiting.
        count: u32,
    },
}

impl Event {
    /// Whether this event should be announced to the user.
    pub fn should_notify(&self) -> bool {
        matches!(self, Event::BecameAvailable { .. })
    }
}

/// Watches the review queue and reports when it changes.
///
/// The detector remembers whether reviews were pending. It only reports
/// [`Event::BecameAvailable`] on the transition from "nothing waiting" to
/// "something waiting", so a queue that stays pending never notifies twice.
///
/// The state is serialisable so it survives a restart: if the app is quit
/// while reviews are pending, restarting it will not announce them again.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Detector {
    /// Whether reviews were pending the last time we looked.
    pending: bool,
}

impl Detector {
    /// Creates a detector that has not seen any reviews yet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether reviews are currently believed to be pending.
    pub fn is_pending(&self) -> bool {
        self.pending
    }

    /// Feeds a freshly polled review count into the detector.
    pub fn observe(&mut self, count: u32) -> Event {
        let was_pending = self.pending;
        self.pending = count > 0;

        match (was_pending, count > 0) {
            (false, false) => Event::Idle,
            (false, true) => Event::BecameAvailable { count },
            (true, true) => Event::StillPending { count },
            (true, false) => Event::Cleared,
        }
    }
}

/// The longest we are willing to sleep between polls.
pub const MAX_POLL_INTERVAL: Duration = Duration::from_secs(30 * 60);

/// The shortest we are willing to sleep between polls.
pub const MIN_POLL_INTERVAL: Duration = Duration::from_secs(60);

/// How often we look again while the user still has reviews waiting.
///
/// The queue only clears when the user completes the reviews on the website,
/// so we look fairly often to notice that and re-arm the detector.
pub const PENDING_POLL_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// How long to wait before the next poll.
///
/// * When reviews are pending we look again after [`PENDING_POLL_INTERVAL`],
///   so that clearing the queue is noticed promptly.
/// * Otherwise we sleep until the next scheduled unlock, but never longer
///   than [`MAX_POLL_INTERVAL`], so a stale summary cannot make us miss
///   reviews for hours.
pub fn next_poll_delay(
    now: DateTime<Utc>,
    pending: bool,
    next_unlock: Option<DateTime<Utc>>,
) -> Duration {
    if pending {
        return PENDING_POLL_INTERVAL;
    }

    let delay = match next_unlock {
        Some(moment) if moment > now => (moment - now).to_std().unwrap_or(MIN_POLL_INTERVAL),
        _ => MAX_POLL_INTERVAL,
    };

    // Wake a little after the unlock so the API has caught up with it.
    let delay = delay + Duration::from_secs(30);
    delay.clamp(MIN_POLL_INTERVAL, MAX_POLL_INTERVAL)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_reviews_announce() {
        let mut detector = Detector::new();
        assert_eq!(
            detector.observe(3),
            Event::BecameAvailable { count: 3 }
        );
        assert!(detector.is_pending());
    }

    #[test]
    fn a_pending_queue_does_not_announce_twice() {
        let mut detector = Detector::new();
        detector.observe(3);
        assert_eq!(detector.observe(3), Event::StillPending { count: 3 });
        assert_eq!(detector.observe(1), Event::StillPending { count: 1 });
    }

    #[test]
    fn clearing_the_queue_re_arms_the_detector() {
        let mut detector = Detector::new();
        detector.observe(3);
        assert_eq!(detector.observe(0), Event::Cleared);
        assert!(!detector.is_pending());
        assert_eq!(
            detector.observe(2),
            Event::BecameAvailable { count: 2 }
        );
    }

    #[test]
    fn an_empty_queue_is_idle() {
        let mut detector = Detector::new();
        assert_eq!(detector.observe(0), Event::Idle);
        assert_eq!(detector.observe(0), Event::Idle);
    }

    #[test]
    fn only_new_reviews_are_announced() {
        assert!(Event::BecameAvailable { count: 1 }.should_notify());
        assert!(!Event::StillPending { count: 1 }.should_notify());
        assert!(!Event::Cleared.should_notify());
        assert!(!Event::Idle.should_notify());
    }

    #[test]
    fn detector_state_survives_a_round_trip() {
        let mut detector = Detector::new();
        detector.observe(4);
        let encoded = serde_json::to_string(&detector).unwrap();
        let mut restored: Detector = serde_json::from_str(&encoded).unwrap();
        assert!(restored.is_pending());
        assert_eq!(restored.observe(4), Event::StillPending { count: 4 });
    }

    #[test]
    fn pending_polls_wait_for_the_user_to_finish() {
        let now = Utc::now();
        assert_eq!(
            next_poll_delay(now, true, None),
            PENDING_POLL_INTERVAL
        );
    }

    #[test]
    fn an_idle_poll_waits_for_the_next_unlock() {
        let now = Utc::now();
        let unlock = now + chrono::Duration::minutes(10);
        let delay = next_poll_delay(now, false, Some(unlock));
        // Ten minutes plus the grace period, clamped to the maximum.
        assert!(delay >= chrono::Duration::minutes(10).to_std().unwrap());
        assert!(delay <= MAX_POLL_INTERVAL);
    }

    #[test]
    fn an_idle_poll_without_a_schedule_falls_back_to_the_maximum() {
        let now = Utc::now();
        assert_eq!(next_poll_delay(now, false, None), MAX_POLL_INTERVAL);
    }

    #[test]
    fn a_past_unlock_does_not_produce_a_negative_delay() {
        let now = Utc::now();
        let past = now - chrono::Duration::hours(2);
        let delay = next_poll_delay(now, false, Some(past));
        assert!(delay >= MIN_POLL_INTERVAL);
    }
}
