//! Port of `Network/HttpRequestAction.cs`.

use crate::background;
use ferroui_base::threading::Dispatcher;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::rc::Rc;
use std::time::Duration;

/// The time a request may take (the default timeout of the HTTP client of the reference).
const REQUEST_TIMEOUT: Duration = Duration::from_secs(100);

/// An action that performs an HTTP request.
///
/// The request runs on a background thread; [`response_status_code`](Self::response_status_code)
/// and [`response_content`](Self::response_content) are set on the thread of the user interface
/// when it completes (the message of the error and the status 0 when it fails).
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference sends the request with `HttpClient`; the
/// port speaks HTTP/1.1 over a socket of the standard library, so only `http` URLs are supported
/// (an `https` URL fails as a request that could not be sent). Behind the feature `network`.
#[repr(C)]
pub struct HttpRequestAction {
    base: StyledElementAction,
}

ferro_class!(HttpRequestAction: StyledElementAction);
ferro_class_info!(HttpRequestAction { new: HttpRequestAction::new });
ferroui_base::ferro_impl_classes!(HttpRequestAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl HttpRequestAction {
        /// Identifies the `Url` property.
        pub fn url_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<HttpRequestAction, _>("Url", None)
        }

        /// Identifies the `Method` property.
        pub fn method_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<HttpRequestAction, _>("Method", Some("GET".to_string()))
        }

        /// Identifies the `Content` property.
        pub fn content_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<HttpRequestAction, _>("Content", None)
        }

        /// Identifies the `ContentType` property.
        pub fn content_type_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<HttpRequestAction, _>("ContentType", Some("application/json".to_string()))
        }

        /// Identifies the `ResponseContent` property.
        pub fn response_content_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<HttpRequestAction, _>("ResponseContent", None)
        }

        /// Identifies the `ResponseStatusCode` property.
        pub fn response_status_code_property() -> StyledProperty<i32> {
            FerroProperty::register::<HttpRequestAction, _>("ResponseStatusCode", 0)
        }
    }
}

impl StyledElementActionImpl for HttpRequestAction {
    /// Starts the request. Returns true if it is started (the URL is set); else false.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let Some(url) = this.url().filter(|url| !url.is_empty()) else {
            return Some(Rc::new(false));
        };

        this.make_request(url);
        Some(Rc::new(true))
    }
}

impl HttpRequestAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the URL.
    pub fn url(&self) -> Option<String> {
        self.get_value(Self::url_property())
    }

    pub fn set_url(&self, value: Option<String>) {
        self.set_value(Self::url_property(), value)
    }

    /// Gets or sets the HTTP method.
    pub fn method(&self) -> Option<String> {
        self.get_value(Self::method_property())
    }

    pub fn set_method(&self, value: Option<String>) {
        self.set_value(Self::method_property(), value)
    }

    /// Gets or sets the content to send.
    pub fn content(&self) -> Option<String> {
        self.get_value(Self::content_property())
    }

    pub fn set_content(&self, value: Option<String>) {
        self.set_value(Self::content_property(), value)
    }

    /// Gets or sets the content type.
    pub fn content_type(&self) -> Option<String> {
        self.get_value(Self::content_type_property())
    }

    pub fn set_content_type(&self, value: Option<String>) {
        self.set_value(Self::content_type_property(), value)
    }

    /// Gets or sets the response content.
    pub fn response_content(&self) -> Option<String> {
        self.get_value(Self::response_content_property())
    }

    pub fn set_response_content(&self, value: Option<String>) {
        self.set_value(Self::response_content_property(), value)
    }

    /// Gets or sets the response status code.
    pub fn response_status_code(&self) -> i32 {
        self.get_value(Self::response_status_code_property())
    }

    pub fn set_response_status_code(&self, value: i32) {
        self.set_value(Self::response_status_code_property(), value)
    }

    fn make_request(&self, url: String) {
        let method = self.method().map(|method| method.to_uppercase()).unwrap_or_else(|| "GET".to_string());
        let content = self.content().filter(|content| !content.is_empty());
        let content_type = self.content_type().unwrap_or_else(|| "application/json".to_string());

        let id = background::register(self);
        let dispatcher = Dispatcher::ui_thread();
        let spawned = std::thread::Builder::new().name("HttpRequestAction".to_string()).spawn(move || {
            let (status_code, response_content) = match send_request(&method, &url, content.as_deref(), &content_type) {
                Ok(response) => response,
                Err(message) => (0, message),
            };

            background::post_to(&dispatcher, id, move |target| {
                background::unregister(id);
                if let Some(action) = target.downcast_ref::<HttpRequestAction>() {
                    action.set_response_status_code(status_code);
                    action.set_response_content(Some(response_content));
                }
            });
        });

        if let Err(error) = spawned {
            background::unregister(id);
            self.set_response_content(Some(error.to_string()));
            self.set_response_status_code(0);
        }
    }
}

/// The parts of an `http` URL: the host to connect to, the port, the value of the `Host` header and
/// the path with the query.
struct HttpUrl {
    host: String,
    port: u16,
    host_header: String,
    path: String,
}

fn parse_url(url: &str) -> Result<HttpUrl, String> {
    let lower = url.to_ascii_lowercase();
    let rest = if lower.starts_with("http://") {
        &url["http://".len()..]
    } else if lower.starts_with("https://") {
        return Err("The SSL connection could not be established: https is not supported.".to_string());
    } else {
        return Err("Only 'http' and 'https' schemes are allowed. (Parameter 'requestUri')".to_string());
    };

    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let mut path = rest[authority_end..].to_string();
    if let Some(fragment) = path.find('#') {
        path.truncate(fragment);
    }
    if !path.starts_with('/') {
        path.insert(0, '/');
    }

    // Credentials in the authority are not sent.
    let authority = authority.rsplit('@').next().unwrap_or(authority);
    let (host, port) = if let Some(stripped) = authority.strip_prefix('[') {
        let end = stripped.find(']').ok_or_else(|| "Invalid URI: The hostname could not be parsed.".to_string())?;
        let host = &stripped[..end];
        let port = stripped[end + 1..].strip_prefix(':');
        (host.to_string(), port)
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) => (host.to_string(), Some(port)),
            None => (authority.to_string(), None),
        }
    };

    if host.is_empty() {
        return Err("Invalid URI: The hostname could not be parsed.".to_string());
    }

    let port = match port.filter(|port| !port.is_empty()) {
        Some(port) => port.parse::<u16>().map_err(|_| "Invalid URI: Invalid port specified.".to_string())?,
        None => 80,
    };

    Ok(HttpUrl { host, port, host_header: authority.to_string(), path })
}

/// Sends a request and reads the response: its status code and its content as text.
fn send_request(method: &str, url: &str, content: Option<&str>, content_type: &str) -> Result<(i32, String), String> {
    let url = parse_url(url)?;

    let mut stream = TcpStream::connect((url.host.as_str(), url.port)).map_err(|error| error.to_string())?;
    stream.set_read_timeout(Some(REQUEST_TIMEOUT)).map_err(|error| error.to_string())?;
    stream.set_write_timeout(Some(REQUEST_TIMEOUT)).map_err(|error| error.to_string())?;

    let mut request =
        format!("{method} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept: */*\r\n", url.path, url.host_header);
    if let Some(content) = content {
        request.push_str(&format!(
            "Content-Type: {content_type}; charset=utf-8\r\nContent-Length: {}\r\n",
            content.len()
        ));
    }
    request.push_str("\r\n");
    if let Some(content) = content {
        request.push_str(content);
    }

    stream.write_all(request.as_bytes()).map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;

    let mut response = Vec::new();
    stream.read_to_end(&mut response).map_err(|error| error.to_string())?;

    parse_response(&response)
}

/// The status code and the content of a response.
fn parse_response(response: &[u8]) -> Result<(i32, String), String> {
    let header_end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "The response ended prematurely.".to_string())?;
    let head = String::from_utf8_lossy(&response[..header_end]);
    let mut body = &response[header_end + 4..];

    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or_default();
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse::<i32>().ok())
        .ok_or_else(|| "The server returned an invalid or unrecognized response.".to_string())?;

    let mut is_chunked = false;
    let mut content_length: Option<usize> = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("transfer-encoding") && value.to_ascii_lowercase().contains("chunked") {
            is_chunked = true;
        } else if name.eq_ignore_ascii_case("content-length") {
            content_length = value.parse().ok();
        }
    }

    let content = if is_chunked {
        decode_chunked(body)?
    } else {
        if let Some(length) = content_length {
            body = &body[..length.min(body.len())];
        }
        body.to_vec()
    };

    Ok((status_code, String::from_utf8_lossy(&content).into_owned()))
}

/// The content of a body sent with the chunked transfer coding.
fn decode_chunked(mut body: &[u8]) -> Result<Vec<u8>, String> {
    let mut content = Vec::new();
    loop {
        let line_end = body
            .windows(2)
            .position(|window| window == b"\r\n")
            .ok_or_else(|| "The response ended prematurely.".to_string())?;
        let size_text = String::from_utf8_lossy(&body[..line_end]);
        let size_text = size_text.split(';').next().unwrap_or_default().trim();
        let size = usize::from_str_radix(size_text, 16).map_err(|_| "Invalid chunk size.".to_string())?;
        body = &body[line_end + 2..];
        if size == 0 {
            return Ok(content);
        }

        if body.len() < size {
            return Err("The response ended prematurely.".to_string());
        }
        content.extend_from_slice(&body[..size]);
        body = &body[size..];
        body = body.strip_prefix(b"\r\n").unwrap_or(body);
    }
}
