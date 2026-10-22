//! A small, typed client for the WaniKani v2 API.
//!
//! Only the parts Tanuki needs are modelled: the review queue and the
//! summary report. Authentication is a bearer token obtained from
//! <https://www.wanikani.com/settings/personal_access_tokens>.

use chrono::{DateTime, Utc};
use reqwest::header::{AUTHORIZATION, ETAG, IF_NONE_MATCH};
use reqwest::{StatusCode, Url};
use serde::Deserialize;
use std::time::Duration;

/// The API revision this client speaks.
pub const API_REVISION: &str = "20170710";

const DEFAULT_BASE_URL: &str = "https://api.wanikani.com/v2";

/// The environment variable that overrides the API root.
///
/// This exists so the app can be pointed at a local server while it is being
/// developed. It is not meant to be used in normal operation.
pub const BASE_URL_ENV: &str = "TANUKI_API_URL";

/// Anything that can go wrong while talking to the WaniKani API.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// The API token was rejected.
    #[error("the API token was rejected")]
    Unauthorized,

    /// The API asked us to slow down.
    #[error("rate limited by the WaniKani API")]
    RateLimited,

    /// The API answered with something we do not understand.
    #[error("unexpected response from the WaniKani API: HTTP {0}")]
    UnexpectedStatus(u16),

    /// The API could not be reached at all.
    #[error("could not reach the WaniKani API: {0}")]
    Transport(#[from] reqwest::Error),

    /// The API answered with something we could not parse.
    #[error("could not parse the WaniKani API response: {0}")]
    Decode(#[from] serde_json::Error),

    /// The URL of the API could not be built.
    #[error("could not build a request URL: {0}")]
    InvalidUrl(String),
}

/// One hour's worth of subjects that become available at the same moment.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Bucket {
    /// The moment the subjects in this bucket become available.
    pub available_at: DateTime<Utc>,
    /// The subjects that become available at [`Self::available_at`].
    pub subject_ids: Vec<i64>,
}

/// The body of the `summary` report.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SummaryData {
    /// Lessons waiting to be started, grouped by the hour they unlock.
    pub lessons: Vec<Bucket>,
    /// When the next batch of reviews becomes available.
    pub next_reviews_at: Option<DateTime<Utc>>,
    /// Reviews becoming available, grouped by the hour they unlock.
    pub reviews: Vec<Bucket>,
}

/// The `summary` report.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Summary {
    /// When the report itself was last regenerated.
    pub data_updated_at: Option<DateTime<Utc>>,
    /// The report contents.
    pub data: SummaryData,
}

impl SummaryData {
    /// The next moment at which reviews become available.
    ///
    /// The summary groups upcoming reviews into hourly buckets. Only the
    /// buckets that still lie in the future matter here: a bucket whose
    /// moment has passed describes reviews the user has not cleared yet, and
    /// those are reported by the review queue instead.
    pub fn next_review_unlock(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.reviews
            .iter()
            .filter(|bucket| !bucket.subject_ids.is_empty() && bucket.available_at > now)
            .map(|bucket| bucket.available_at)
            .min()
    }

    /// The next moment at which lessons become available.
    pub fn next_lesson_unlock(&self, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.lessons
            .iter()
            .filter(|bucket| !bucket.subject_ids.is_empty() && bucket.available_at > now)
            .map(|bucket| bucket.available_at)
            .min()
    }

    /// The number of subjects that unlock in each of the next `limit` hours.
    pub fn upcoming_review_counts(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> Vec<(DateTime<Utc>, usize)> {
        self.reviews
            .iter()
            .filter(|bucket| bucket.available_at > now)
            .take(limit)
            .map(|bucket| (bucket.available_at, bucket.subject_ids.len()))
            .collect()
    }
}

/// A client for the WaniKani API.
///
/// The client caches the last response of each endpoint together with its
/// `ETag`, so repeated polls can be answered with a cheap `304 Not Modified`.
/// Because of that cache the fetching methods take `&mut self`.
#[derive(Debug, Clone)]
pub struct WanikaniClient {
    http: reqwest::Client,
    token: String,
    base_url: Url,
    assignments_etag: Option<String>,
    assignments_count: Option<u32>,
    summary_etag: Option<String>,
    summary: Option<Summary>,
    user_etag: Option<String>,
    level: Option<u32>,
}

impl WanikaniClient {
    /// Creates a client for the API using the given personal access token.
    pub fn new(token: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .user_agent(concat!("Tanuki/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("a client with default TLS settings should always build");

        let base_url = std::env::var(BASE_URL_ENV)
            .ok()
            .and_then(|value| Url::parse(&value).ok())
            .unwrap_or_else(|| {
                Url::parse(DEFAULT_BASE_URL).expect("the default base URL is valid")
            });

        Self {
            http,
            token: token.into(),
            base_url,
            assignments_etag: None,
            assignments_count: None,
            summary_etag: None,
            summary: None,
            user_etag: None,
            level: None,
        }
    }

    /// The token this client authenticates with.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Points the client at a different API root.
    ///
    /// Used by the test suite to talk to a local server.
    pub fn with_base_url(mut self, base_url: impl AsRef<str>) -> Self {
        self.base_url =
            Url::parse(base_url.as_ref()).expect("the base URL should be a valid absolute URL");
        self
    }

    /// The number of reviews waiting to be done right now.
    ///
    /// This is the authoritative count: it reflects exactly what the review
    /// queue on the website would offer.
    pub async fn available_review_count(&mut self) -> Result<u32, ApiError> {
        let url = self.url("assignments", Some("immediately_available_for_review=true"))?;
        let mut request = self.authorised(url);
        if let Some(etag) = &self.assignments_etag {
            request = request.header(IF_NONE_MATCH, etag);
        }

        let response = request.send().await?;
        if response.status() == StatusCode::NOT_MODIFIED {
            return Ok(self.assignments_count.unwrap_or_default());
        }
        Self::check_status(response.status())?;

        let etag = etag_of(&response);
        let envelope: CountEnvelope = response.json().await?;
        self.assignments_etag = etag;
        self.assignments_count = Some(envelope.total_count);
        Ok(envelope.total_count)
    }

    /// Fetches the summary report describing lessons and upcoming reviews.
    pub async fn summary(&mut self) -> Result<Summary, ApiError> {
        let url = self.url("summary", None)?;
        let mut request = self.authorised(url);
        if let Some(etag) = &self.summary_etag {
            request = request.header(IF_NONE_MATCH, etag);
        }

        let response = request.send().await?;
        if response.status() == StatusCode::NOT_MODIFIED {
            if let Some(summary) = &self.summary {
                return Ok(summary.clone());
            }
        }
        Self::check_status(response.status())?;

        let etag = etag_of(&response);
        let summary: Summary = response.json().await?;
        self.summary_etag = etag;
        self.summary = Some(summary.clone());
        Ok(summary)
    }

    /// The user's current level.
    ///
    /// The level goes up when the user finishes enough lessons, and is worth
    /// congratulating them on.
    pub async fn level(&mut self) -> Result<u32, ApiError> {
        let url = self.url("user", None)?;
        let mut request = self.authorised(url);
        if let Some(etag) = &self.user_etag {
            request = request.header(IF_NONE_MATCH, etag);
        }

        let response = request.send().await?;
        if response.status() == StatusCode::NOT_MODIFIED {
            return Ok(self.level.unwrap_or_default());
        }
        Self::check_status(response.status())?;

        let etag = etag_of(&response);
        let envelope: UserEnvelope = response.json().await?;
        self.user_etag = etag;
        self.level = Some(envelope.data.level);
        Ok(envelope.data.level)
    }

    /// Forgets any cached responses, so the next call re-reads from the API.
    pub fn invalidate_cache(&mut self) {
        self.assignments_etag = None;
        self.assignments_count = None;
        self.summary_etag = None;
        self.summary = None;
        self.user_etag = None;
        self.level = None;
    }

    fn url(&self, path: &str, query: Option<&str>) -> Result<Url, ApiError> {
        // The path and the query have to be set separately: putting a `?` in
        // the path would percent-encode it and the filter would never be
        // sent.
        let mut url = self.base_url.clone();
        let base_path = url.path().trim_end_matches('/').to_owned();
        url.set_path(&format!("{base_path}/{path}"));
        url.set_query(query);
        Ok(url)
    }

    fn authorised(&self, url: Url) -> reqwest::RequestBuilder {
        self.http
            .get(url)
            .header(AUTHORIZATION, format!("Bearer {}", self.token))
            .header("Wanikani-Revision", API_REVISION)
    }

    fn check_status(status: StatusCode) -> Result<(), ApiError> {
        if status == StatusCode::UNAUTHORIZED {
            Err(ApiError::Unauthorized)
        } else if status == StatusCode::TOO_MANY_REQUESTS {
            Err(ApiError::RateLimited)
        } else if !status.is_success() {
            Err(ApiError::UnexpectedStatus(status.as_u16()))
        } else {
            Ok(())
        }
    }
}

/// The parts of a collection response we care about.
#[derive(Debug, Clone, Deserialize)]
struct CountEnvelope {
    total_count: u32,
}

/// The part of the user endpoint Tanuki cares about.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct UserEnvelope {
    data: UserData,
}

/// The user's own details.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct UserData {
    /// The level the user has reached.
    level: u32,
}

fn etag_of(response: &reqwest::Response) -> Option<String> {
    response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}
