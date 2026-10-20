//! A tiny HTTP server used by the client tests.
//!
//! It answers with canned responses and records the requests it received, so
//! the tests can check headers, caching behaviour and error handling without
//! touching the real API.
//!
//! Each test file is its own binary, so not every helper is used by all of
//! them.
#![allow(dead_code)]

use std::net::TcpListener;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

/// A recorded request.
#[derive(Debug, Clone)]
pub struct RecordedRequest {
    /// The request line, for example `GET /v2/summary HTTP/1.1`.
    pub request_line: String,
    /// Every header of the request, lowercased.
    pub headers: Vec<(String, String)>,
}

impl RecordedRequest {
    /// The value of a header, if it was sent.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

/// A canned reply.
#[derive(Debug, Clone)]
pub struct CannedResponse {
    /// The status line, for example `HTTP/1.1 200 OK`.
    pub status: String,
    /// The response headers.
    pub headers: Vec<(String, String)>,
    /// The response body.
    pub body: String,
}

impl CannedResponse {
    /// A `200 OK` reply with a JSON body.
    pub fn ok(body: impl Into<String>) -> Self {
        Self {
            status: "HTTP/1.1 200 OK".to_owned(),
            headers: vec![("Content-Type".to_owned(), "application/json".to_owned())],
            body: body.into(),
        }
    }

    /// A reply with the given status line and no body.
    pub fn status(status: &str) -> Self {
        Self {
            status: status.to_owned(),
            headers: Vec::new(),
            body: String::new(),
        }
    }

    /// Adds a header to the reply.
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_owned(), value.to_owned()));
        self
    }
}

/// Decides which reply a request gets.
pub type Responder = Box<dyn Fn(&str) -> CannedResponse + Send>;

/// A server that answers every request with the same reply.
pub struct MockServer {
    base_url: String,
    requests: Receiver<RecordedRequest>,
    _sender: Sender<RecordedRequest>,
}

impl MockServer {
    /// Starts a server that answers with `response`.
    pub fn start(response: CannedResponse) -> Self {
        Self::start_sequence(vec![response])
    }

    /// Starts a server that answers with each response in turn.
    ///
    /// Once the responses run out, the last one is repeated.
    pub fn start_sequence(responses: Vec<CannedResponse>) -> Self {
        let last = responses.last().cloned();
        let responses = Arc::new(Mutex::new(responses.into_iter()));
        Self::start_with(move |_| {
            let mut responses = responses.lock().expect("the response lock should not be poisoned");
            match responses.next() {
                Some(response) => response,
                None => last.clone().unwrap_or_else(|| {
                    CannedResponse::status("HTTP/1.1 500 Internal Server Error")
                }),
            }
        })
    }

    /// Starts a server that picks its reply based on the request path.
    pub fn start_with<F>(responder: F) -> Self
    where
        F: Fn(&str) -> CannedResponse + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("the test server should bind");
        let address = listener.local_addr().expect("the test server should have an address");
        let (sender, requests) = channel();
        let thread_sender = sender.clone();
        let responder = Arc::new(responder);

        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let Some(request) = read_request(&mut stream) else {
                    continue;
                };
                let path = request
                    .request_line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default()
                    .to_owned();
                if thread_sender.send(request).is_err() {
                    break;
                }
                write_response(&mut stream, &responder(&path));
            }
        });

        Self {
            base_url: format!("http://{address}/v2"),
            requests,
            _sender: sender,
        }
    }

    /// Starts a server that answers the assignments endpoint with a count and
    /// the summary endpoint with the given body.
    ///
    /// The path has to match exactly, so a request with the query string
    /// percent-encoded into the path is rejected rather than quietly
    /// answering the wrong thing.
    pub fn start_with_count(count: u32, summary: &str) -> Self {
        let count_body = format!(r#"{{"object":"collection","total_count":{count},"data":[]}}"#);
        let summary = summary.to_owned();
        Self::start_with(move |request| {
            // The path without the query string.
            let path = request.split('?').next().unwrap_or(request);
            match path {
                "/v2/assignments" => CannedResponse::ok(count_body.clone()),
                "/v2/summary" => CannedResponse::ok(summary.clone()),
                _ => CannedResponse::status("HTTP/1.1 404 Not Found"),
            }
        })
    }

    /// The base URL the server listens on.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Waits for the next request and returns it.
    pub fn next_request(&self) -> RecordedRequest {
        self.requests
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("the test server should have received a request")
    }
}

/// Reads a single HTTP request off the stream.
fn read_request(stream: &mut std::net::TcpStream) -> Option<RecordedRequest> {
    use std::io::Read;

    let mut buffer = [0u8; 8192];
    let read = stream.read(&mut buffer).ok()?;
    let text = String::from_utf8_lossy(&buffer[..read]).to_string();

    let mut lines = text.split("\r\n");
    let request_line = lines.next()?.to_owned();
    let headers = lines
        .take_while(|line| !line.is_empty())
        .filter_map(|line| {
            let (name, value) = line.split_once(':')?;
            Some((name.trim().to_lowercase(), value.trim().to_owned()))
        })
        .collect();

    Some(RecordedRequest {
        request_line,
        headers,
    })
}

/// Writes a canned reply to the stream.
fn write_response(stream: &mut std::net::TcpStream, response: &CannedResponse) {
    use std::io::Write;

    let mut reply = format!("{}\r\n", response.status);
    for (name, value) in &response.headers {
        reply.push_str(&format!("{name}: {value}\r\n"));
    }
    reply.push_str(&format!("Content-Length: {}\r\n", response.body.len()));
    reply.push_str("Connection: close\r\n\r\n");
    reply.push_str(&response.body);

    let _ = stream.write_all(reply.as_bytes());
    let _ = stream.flush();
}
