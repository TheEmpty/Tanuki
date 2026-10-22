//! End to end tests of the polling behaviour.
//!
//! These drive the real client against a local server that answers like the
//! WaniKani API, and check that the detector reports the right thing as
//! reviews come and go.

mod support;

use notifier_core::api::WanikaniClient;
use notifier_core::detect::{Detector, Event};
use support::{CannedResponse, MockServer};

const SUMMARY_BODY: &str = r#"{
  "object": "report",
  "data_updated_at": "2024-05-01T12:00:00.000000Z",
  "data": {
    "lessons": [],
    "next_reviews_at": "2024-05-01T13:00:00.000000Z",
    "reviews": [
      { "available_at": "2024-05-01T12:00:00.000000Z", "subject_ids": [] },
      { "available_at": "2024-05-01T13:00:00.000000Z", "subject_ids": [1, 2] }
    ]
  }
}"#;

/// Answers the assignments endpoint with a count and the summary with a
/// fixed body.
fn server_with_count(count: u32) -> MockServer {
    MockServer::start_with_count(count, SUMMARY_BODY)
}

#[tokio::test]
async fn no_reviews_at_all_is_idle() {
    let server = server_with_count(0);
    let mut client = WanikaniClient::new("token").with_base_url(server.base_url());
    let mut detector = Detector::new();

    let count = client.available_review_count().await.unwrap();
    assert_eq!(count, 0);
    assert_eq!(detector.observe(count), Event::Idle);
}

#[tokio::test]
async fn the_first_reviews_are_announced() {
    let server = server_with_count(12);
    let mut client = WanikaniClient::new("token").with_base_url(server.base_url());
    let mut detector = Detector::new();

    let count = client.available_review_count().await.unwrap();
    assert_eq!(count, 12);
    assert_eq!(
        detector.observe(count),
        Event::BecameAvailable { count: 12 }
    );
    assert!(detector.is_pending());
}

#[tokio::test]
async fn a_pending_queue_is_not_announced_again() {
    let server = server_with_count(12);
    let mut client = WanikaniClient::new("token").with_base_url(server.base_url());
    let mut detector = Detector::new();

    detector.observe(client.available_review_count().await.unwrap());
    assert_eq!(
        detector.observe(client.available_review_count().await.unwrap()),
        Event::StillPending { count: 12 }
    );
}

#[tokio::test]
async fn clearing_the_queue_re_arms_the_detector() {
    // The count walks down to zero and then back up.
    let counts = std::sync::Arc::new(std::sync::Mutex::new(vec![12u32, 0, 4].into_iter()));
    let server = MockServer::start_with(move |path| {
        if path.contains("summary") {
            return CannedResponse::ok(SUMMARY_BODY);
        }
        let mut counts = counts
            .lock()
            .expect("the count lock should not be poisoned");
        let count = counts.next().unwrap_or(4);
        CannedResponse::ok(format!(r#"{{"total_count":{count}}}"#))
    });
    let mut client = WanikaniClient::new("token").with_base_url(server.base_url());
    let mut detector = Detector::new();

    assert_eq!(
        detector.observe(client.available_review_count().await.unwrap()),
        Event::BecameAvailable { count: 12 }
    );
    assert_eq!(
        detector.observe(client.available_review_count().await.unwrap()),
        Event::Cleared
    );
    assert!(!detector.is_pending());
    assert_eq!(
        detector.observe(client.available_review_count().await.unwrap()),
        Event::BecameAvailable { count: 4 }
    );
}

#[tokio::test]
async fn the_summary_is_used_to_find_the_next_unlock() {
    let server = server_with_count(0);
    let mut client = WanikaniClient::new("token").with_base_url(server.base_url());

    let summary = client.summary().await.unwrap();
    let now = chrono::DateTime::parse_from_rfc3339("2024-05-01T12:30:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let next = summary.data.next_review_unlock(now).unwrap();
    assert_eq!(next.to_rfc3339(), "2024-05-01T13:00:00+00:00");
}
