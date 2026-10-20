//! Tests for the WaniKani API client.

mod support;

use notifier_core::api::{ApiError, WanikaniClient, API_REVISION};
use support::{CannedResponse, MockServer};

const SUMMARY_BODY: &str = r#"{
  "object": "report",
  "url": "https://api.wanikani.com/v2/summary",
  "data_updated_at": "2024-05-01T12:00:00.000000Z",
  "data": {
    "lessons": [
      { "available_at": "2024-05-01T12:00:00.000000Z", "subject_ids": [1, 2] }
    ],
    "next_reviews_at": "2024-05-01T13:00:00.000000Z",
    "reviews": [
      { "available_at": "2024-05-01T12:00:00.000000Z", "subject_ids": [3, 4, 5] },
      { "available_at": "2024-05-01T13:00:00.000000Z", "subject_ids": [6, 7] },
      { "available_at": "2024-05-01T14:00:00.000000Z", "subject_ids": [] }
    ]
  }
}"#;

fn client_for(server: &MockServer) -> WanikaniClient {
    WanikaniClient::new("test-token").with_base_url(server.base_url())
}

#[tokio::test]
async fn the_review_count_comes_from_the_total_count() {
    let server = MockServer::start(CannedResponse::ok(
        r#"{"object":"collection","total_count":42,"data":[]}"#,
    ));
    let mut client = client_for(&server);

    assert_eq!(client.available_review_count().await.unwrap(), 42);
}

#[tokio::test]
async fn requests_carry_the_token_and_the_revision() {
    let server = MockServer::start(CannedResponse::ok(r#"{"total_count":0}"#));
    let mut client = client_for(&server);

    client.available_review_count().await.unwrap();

    let request = server.next_request();
    assert!(request.request_line.contains("/v2/assignments"));
    assert_eq!(request.header("authorization"), Some("Bearer test-token"));
    assert_eq!(request.header("wanikani-revision"), Some(API_REVISION));
}

#[tokio::test]
async fn the_review_filter_is_requested() {
    let server = MockServer::start(CannedResponse::ok(r#"{"total_count":0}"#));
    let mut client = client_for(&server);

    client.available_review_count().await.unwrap();

    let request = server.next_request();
    assert!(request
        .request_line
        .contains("immediately_available_for_review=true"));
    // The filter has to arrive as a query string, not as part of the path.
    assert!(
        !request.request_line.contains("%3F"),
        "the query string must not be percent-encoded into the path: {}",
        request.request_line
    );
}

#[tokio::test]
async fn a_rejected_token_is_reported_as_unauthorized() {
    let server = MockServer::start(CannedResponse::status("HTTP/1.1 401 Unauthorized"));
    let mut client = client_for(&server);

    let error = client.available_review_count().await.unwrap_err();
    assert!(matches!(error, ApiError::Unauthorized));
}

#[tokio::test]
async fn rate_limiting_is_reported() {
    let server = MockServer::start(CannedResponse::status("HTTP/1.1 429 Too Many Requests"));
    let mut client = client_for(&server);

    let error = client.available_review_count().await.unwrap_err();
    assert!(matches!(error, ApiError::RateLimited));
}

#[tokio::test]
async fn a_server_error_is_reported_with_its_status() {
    let server = MockServer::start(CannedResponse::status("HTTP/1.1 500 Internal Server Error"));
    let mut client = client_for(&server);

    let error = client.available_review_count().await.unwrap_err();
    assert!(matches!(error, ApiError::UnexpectedStatus(500)));
}

#[tokio::test]
async fn the_summary_is_parsed() {
    let server = MockServer::start(CannedResponse::ok(SUMMARY_BODY));
    let mut client = client_for(&server);

    let summary = client.summary().await.unwrap();
    assert_eq!(summary.data.reviews.len(), 3);
    assert_eq!(summary.data.reviews[0].subject_ids, vec![3, 4, 5]);
    assert!(summary.data.next_reviews_at.is_some());
}

#[tokio::test]
async fn the_next_unlock_ignores_buckets_that_have_already_passed() {
    let server = MockServer::start(CannedResponse::ok(SUMMARY_BODY));
    let mut client = client_for(&server);

    let summary = client.summary().await.unwrap();
    let now = chrono::DateTime::parse_from_rfc3339("2024-05-01T12:30:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);

    let next = summary.data.next_review_unlock(now).unwrap();
    assert_eq!(next.to_rfc3339(), "2024-05-01T13:00:00+00:00");
}

#[tokio::test]
async fn an_etag_makes_the_next_request_conditional() {
    let server = MockServer::start_sequence(vec![
        CannedResponse::ok(r#"{"total_count":7}"#).with_header("ETag", "\"abc123\""),
        CannedResponse::status("HTTP/1.1 304 Not Modified"),
    ]);
    let mut client = client_for(&server);

    assert_eq!(client.available_review_count().await.unwrap(), 7);
    let first = server.next_request();
    assert!(first.header("if-none-match").is_none());

    // The server answers 304, so the cached count stands and the request
    // carries the ETag we were given.
    assert_eq!(client.available_review_count().await.unwrap(), 7);
    let second = server.next_request();
    assert_eq!(second.header("if-none-match"), Some("\"abc123\""));
}

#[tokio::test]
async fn invalidating_the_cache_forgets_the_etag() {
    let server = MockServer::start_sequence(vec![
        CannedResponse::ok(r#"{"total_count":1}"#).with_header("ETag", "\"tag\""),
    ]);
    let mut client = client_for(&server);
    client.available_review_count().await.unwrap();
    let _ = server.next_request();

    client.invalidate_cache();
    client.available_review_count().await.unwrap();
    let request = server.next_request();
    assert!(request.header("if-none-match").is_none());
}
