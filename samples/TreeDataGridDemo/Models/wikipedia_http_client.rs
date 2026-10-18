//! Port of `Models/WikipediaHttpClient.cs`: the HTTP client of the Wikipedia page, which sends
//! the user agent of the demo with every request.
//!
//! The reference builds on the HTTP client of the runtime library (`HttpClient`, with an
//! injectable `HttpMessageHandler`). Its counterpart here is part of this module: an
//! [`HttpClient`] that sends its requests through an [`HttpMessageHandler`] trait object, so
//! the tests serve canned responses and failures as the reference's tests do with their stub
//! handler. The default handler ([`HttpClientHandler`]) sends the requests over the network
//! when the demo is built with the feature `online` (on by default, not available on
//! `wasm32`); without it every request fails, and the page shows the reference's error text.
//!
//! A request blocks the thread that sends it, so the asynchronous forms
//! ([`HttpClient::get_string_async`], [`HttpClient::get_byte_array_async`]) send it from a
//! thread of their own and run the continuation on the dispatcher of the calling thread, as an
//! `await` of the reference resumes on the UI thread.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::ThreadBound;
use std::fmt;
use std::sync::{Arc, OnceLock};

/// `WikipediaHttpClient`: the shared client of the demo and the factory of clients with the
/// user agent of the demo.
pub struct WikipediaHttpClient;

impl WikipediaHttpClient {
    /// The user agent the Wikimedia API asks its clients to identify themselves with.
    pub const USER_AGENT: &'static str = "FerroUITreeDataGridSample/1.0 (https://github.com/wieslawsoltes/FerroUI)";

    /// `Shared`: the client of the demo, which sends its requests over the network.
    pub fn shared() -> Arc<HttpClient> {
        static SHARED: OnceLock<Arc<HttpClient>> = OnceLock::new();
        SHARED.get_or_init(|| Self::create(None)).clone()
    }

    /// `Create(handler)`: a client with the user agent of the demo that sends its requests
    /// through `handler`, or over the network without one.
    pub fn create(handler: Option<Arc<dyn HttpMessageHandler>>) -> Arc<HttpClient> {
        let mut result = match handler {
            None => HttpClient::new(),
            Some(handler) => HttpClient::with_handler(handler),
        };
        result.add_default_request_header("User-Agent", Self::USER_AGENT);
        Arc::new(result)
    }
}

// --- the HTTP client of the runtime library -------------------------------------------------

/// `HttpRequestException`: a request failed, or its response does not indicate success.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequestException {
    message: String,
}

impl HttpRequestException {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }

    /// `Message`.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for HttpRequestException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for HttpRequestException {}

/// `HttpRequestMessage`: the method, the URI and the headers of a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequestMessage {
    method: String,
    request_uri: String,
    headers: Vec<(String, String)>,
}

impl HttpRequestMessage {
    /// A request of `method` (`GET`, ...) for `request_uri`, without headers.
    pub fn new(method: &str, request_uri: &str) -> Self {
        Self { method: method.to_string(), request_uri: request_uri.to_string(), headers: Vec::new() }
    }

    /// `Method`.
    pub fn method(&self) -> &str {
        &self.method
    }

    /// `RequestUri`.
    pub fn request_uri(&self) -> &str {
        &self.request_uri
    }

    /// `Headers`: the headers of the request, in the order they were added.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// Adds the header `name` with `value`.
    pub fn add_header(&mut self, name: &str, value: &str) {
        self.headers.push((name.to_string(), value.to_string()));
    }

    /// The values of the header `name` (compared without regard to case), joined as one header
    /// value; `None` when the request has no such header.
    pub fn header(&self, name: &str) -> Option<String> {
        let values: Vec<&str> =
            self.headers.iter().filter(|(key, _)| key.eq_ignore_ascii_case(name)).map(|(_, value)| value.as_str()).collect();
        if values.is_empty() {
            None
        } else {
            Some(values.join(" "))
        }
    }

    /// `Headers.UserAgent.ToString()`: the user agent of the request, or empty.
    pub fn user_agent(&self) -> String {
        self.header("User-Agent").unwrap_or_default()
    }
}

/// `HttpResponseMessage`: the status, the reason phrase and the content of a response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponseMessage {
    status_code: u16,
    reason_phrase: Option<String>,
    content_type: Option<String>,
    content: Vec<u8>,
}

impl HttpResponseMessage {
    /// `new HttpResponseMessage(statusCode)`: a response without content.
    pub fn new(status_code: u16) -> Self {
        Self { status_code, reason_phrase: None, content_type: None, content: Vec::new() }
    }

    /// The response with the reason phrase `value` in place of the one of its status code.
    pub fn with_reason_phrase(mut self, value: impl Into<String>) -> Self {
        self.reason_phrase = Some(value.into());
        self
    }

    /// The response with the content `bytes`.
    pub fn with_content(mut self, bytes: Vec<u8>) -> Self {
        self.content = bytes;
        self
    }

    /// `Content = new StringContent(text, Encoding.UTF8, mediaType)`.
    pub fn with_string_content(mut self, text: &str, media_type: &str) -> Self {
        self.content = text.as_bytes().to_vec();
        self.content_type = Some(format!("{media_type}; charset=utf-8"));
        self
    }

    /// `StatusCode`.
    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    /// `ReasonPhrase`: the phrase the response came with, or the one of its status code.
    pub fn reason_phrase(&self) -> String {
        self.reason_phrase.clone().unwrap_or_else(|| reason_phrase_of(self.status_code).to_string())
    }

    /// The media type of the content, if the response states one.
    pub fn content_type(&self) -> Option<&str> {
        self.content_type.as_deref()
    }

    /// `Content`: the bytes of the content.
    pub fn content(&self) -> &[u8] {
        &self.content
    }

    /// Takes the bytes of the content.
    pub fn into_content(self) -> Vec<u8> {
        self.content
    }

    /// `IsSuccessStatusCode`: a status of 200 to 299.
    pub fn is_success_status_code(&self) -> bool {
        (200..=299).contains(&self.status_code)
    }

    /// `EnsureSuccessStatusCode()`.
    pub fn ensure_success_status_code(self) -> Result<Self, HttpRequestException> {
        if self.is_success_status_code() {
            Ok(self)
        } else {
            Err(HttpRequestException::new(format!(
                "Response status code does not indicate success: {} ({}).",
                self.status_code,
                self.reason_phrase()
            )))
        }
    }
}

/// The reason phrase of an HTTP status code, as the runtime library describes it.
fn reason_phrase_of(status_code: u16) -> &'static str {
    match status_code {
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",
        103 => "Early Hints",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        207 => "Multi-Status",
        208 => "Already Reported",
        226 => "IM Used",
        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        305 => "Use Proxy",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        407 => "Proxy Authentication Required",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        412 => "Precondition Failed",
        413 => "Request Entity Too Large",
        414 => "Request-Uri Too Long",
        415 => "Unsupported Media Type",
        416 => "Requested Range Not Satisfiable",
        417 => "Expectation Failed",
        421 => "Misdirected Request",
        422 => "Unprocessable Entity",
        423 => "Locked",
        424 => "Failed Dependency",
        426 => "Upgrade Required",
        428 => "Precondition Required",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        451 => "Unavailable For Legal Reasons",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "Http Version Not Supported",
        506 => "Variant Also Negotiates",
        507 => "Insufficient Storage",
        508 => "Loop Detected",
        510 => "Not Extended",
        511 => "Network Authentication Required",
        _ => "",
    }
}

/// `HttpMessageHandler`: sends a request and returns its response. A response that does not
/// indicate success is a response, not an error; an error is a request that could not be sent
/// or answered.
///
/// A handler is called from the thread that sends the request, which is not the UI thread.
pub trait HttpMessageHandler: Send + Sync {
    /// `SendAsync(request, cancellationToken)`.
    fn send(&self, request: &HttpRequestMessage) -> Result<HttpResponseMessage, HttpRequestException>;
}

/// `HttpClientHandler`: the handler of a client created without one, which sends the requests
/// over the network.
#[derive(Default)]
pub struct HttpClientHandler;

impl HttpClientHandler {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(all(feature = "online", not(target_arch = "wasm32")))]
impl HttpMessageHandler for HttpClientHandler {
    fn send(&self, request: &HttpRequestMessage) -> Result<HttpResponseMessage, HttpRequestException> {
        use std::io::Read;

        // One agent for every request of the process: it keeps the connections alive.
        static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
        let agent = AGENT.get_or_init(|| ureq::AgentBuilder::new().build());

        let mut call = agent.request(request.method(), request.request_uri());
        for (name, value) in request.headers() {
            call = call.set(name, value);
        }
        let response = match call.call() {
            Ok(response) => response,
            // The status of a response that does not indicate success is for the client to
            // judge (`EnsureSuccessStatusCode`).
            Err(ureq::Error::Status(_, response)) => response,
            Err(ureq::Error::Transport(transport)) => return Err(HttpRequestException::new(transport.to_string())),
        };

        let status_code = response.status();
        let reason_phrase = response.status_text().to_string();
        let content_type = response.header("Content-Type").map(str::to_string);
        let mut content = Vec::new();
        response.into_reader().read_to_end(&mut content).map_err(|e| HttpRequestException::new(e.to_string()))?;

        let mut result = HttpResponseMessage::new(status_code).with_reason_phrase(reason_phrase).with_content(content);
        result.content_type = content_type;
        Ok(result)
    }
}

#[cfg(not(all(feature = "online", not(target_arch = "wasm32"))))]
impl HttpMessageHandler for HttpClientHandler {
    fn send(&self, _request: &HttpRequestMessage) -> Result<HttpResponseMessage, HttpRequestException> {
        Err(HttpRequestException::new(if cfg!(target_arch = "wasm32") {
            "The demo cannot send HTTP requests on this platform."
        } else {
            "The demo was built without HTTP support (the feature `online`)."
        }))
    }
}

/// `HttpClient`: sends `GET` requests with its default headers through its handler.
pub struct HttpClient {
    handler: Arc<dyn HttpMessageHandler>,
    default_request_headers: Vec<(String, String)>,
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient {
    /// `new HttpClient()`: a client that sends its requests over the network.
    pub fn new() -> Self {
        Self::with_handler(Arc::new(HttpClientHandler::new()))
    }

    /// `new HttpClient(handler)`.
    pub fn with_handler(handler: Arc<dyn HttpMessageHandler>) -> Self {
        Self { handler, default_request_headers: Vec::new() }
    }

    /// `DefaultRequestHeaders`: the headers sent with every request.
    pub fn default_request_headers(&self) -> &[(String, String)] {
        &self.default_request_headers
    }

    /// `DefaultRequestHeaders.Add(name, value)` (`UserAgent.ParseAdd(value)` for the user agent).
    pub fn add_default_request_header(&mut self, name: &str, value: &str) {
        self.default_request_headers.push((name.to_string(), value.to_string()));
    }

    /// Sends a `GET` request for `uri` and returns the response if it indicates success.
    fn get(&self, uri: &str) -> Result<HttpResponseMessage, HttpRequestException> {
        let mut request = HttpRequestMessage::new("GET", uri);
        for (name, value) in &self.default_request_headers {
            request.add_header(name, value);
        }
        self.handler.send(&request)?.ensure_success_status_code()
    }

    /// `GetStringAsync(uri)`, blocking the calling thread: the content of the response as text.
    pub fn get_string(&self, uri: &str) -> Result<String, HttpRequestException> {
        let response = self.get(uri)?;
        Ok(String::from_utf8_lossy(response.content()).into_owned())
    }

    /// `GetByteArrayAsync(uri)`, blocking the calling thread: the content of the response.
    pub fn get_byte_array(&self, uri: &str) -> Result<Vec<u8>, HttpRequestException> {
        Ok(self.get(uri)?.into_content())
    }

    /// `await GetStringAsync(uri)`: sends the request from a thread of its own and runs
    /// `completed` with the outcome on the dispatcher of the calling thread.
    pub fn get_string_async(
        self: &Arc<Self>,
        uri: String,
        completed: impl FnOnce(Result<String, HttpRequestException>) + 'static,
    ) {
        let this = self.clone();
        run_async(move || this.get_string(&uri), completed);
    }

    /// `await GetByteArrayAsync(uri)`: sends the request from a thread of its own and runs
    /// `completed` with the outcome on the dispatcher of the calling thread.
    pub fn get_byte_array_async(
        self: &Arc<Self>,
        uri: String,
        completed: impl FnOnce(Result<Vec<u8>, HttpRequestException>) + 'static,
    ) {
        let this = self.clone();
        run_async(move || this.get_byte_array(&uri), completed);
    }
}

/// Runs `work` away from the calling thread and then `completed` with its result on the
/// dispatcher of the calling thread, at the priority of the continuations of an `await`.
///
/// `completed` belongs to the calling thread (it holds the single-threaded objects of the UI);
/// it travels with the result, untouched, until the dispatcher runs it on its own thread. On
/// `wasm32`, which has no threads, the work runs at once and the continuation still runs from
/// the dispatcher, after the caller returned.
fn run_async<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static, completed: impl FnOnce(T) + 'static) {
    let dispatcher = Dispatcher::current_dispatcher();
    let completed: ThreadBound<Box<dyn FnOnce(T)>> = ThreadBound::new(Box::new(completed));
    let run = move || {
        let result = work();
        dispatcher.post(move || (completed.into_inner())(result), DispatcherPriority::DEFAULT);
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = std::thread::Builder::new().name("wikipedia-http".to_string()).spawn(run).expect("a thread for the HTTP request");
    }
    #[cfg(target_arch = "wasm32")]
    run();
}
