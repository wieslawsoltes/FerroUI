use crate::remote::html_transport::simple_web_socket_http_server::{
    SimpleWebSocket, SimpleWebSocketHttpRequest, SimpleWebSocketHttpServer,
};
use crate::remote::remote_designer_entry_point::{host_and_port_or, new_guid};
use ferroui_base::utilities::Uri;
use ferroui_remote_protocol::input::{
    InputModifiers, MouseButton, PointerEventMessageBase, PointerMovedEventMessage, PointerPressedEventMessage,
    PointerReleasedEventMessage, ScrollEventMessage,
};
use ferroui_remote_protocol::viewport::{
    ClientSupportedPixelFormatsMessage, FrameMessage, FrameReceivedMessage, PixelFormat, RequestViewportResizeMessage,
};
use ferroui_remote_protocol::{
    exception_handler, message_handler, Delegate, Error, ExceptionHandler, Guid, HandlerToken, HtmlTransportStartedMessage,
    IFerroRemoteTransportConnection, Message, MessageHandler, Task,
};
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

/// The files of the page, by the names the resources of the original have
/// without their prefix: what the build of the web application leaves in
/// `webapp/build` (`scripts/build-designer-webapp.sh`).
///
/// Deviation (DEVIATIONS.md, Designer support): the original builds the web
/// application when the project is built and embeds every file of its
/// output compressed with gzip, which the constructor decompresses. Here the
/// built files are checked in and embedded as they are, so that building the
/// crate needs neither a JavaScript toolchain nor a decompressor.
const RESOURCES: [(&str, &[u8]); 2] =
    [("index.html", include_bytes!("webapp/build/index.html")), ("index.js", include_bytes!("webapp/build/index.js"))];

const NOT_FOUND: &[u8] = b"404 - Not Found";

/// The content type of a file of the page by its extension.
fn mime(ext: &str) -> Option<&'static str> {
    match ext {
        "html" | "htm" => Some("text/html"),
        "js" => Some("text/javascript"),
        "css" => Some("text/css"),
        _ => None,
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `AutoResetEvent`: a signal that lets one wait through and resets.
struct AutoResetEvent {
    signaled: Mutex<bool>,
    changed: Condvar,
}

impl AutoResetEvent {
    fn new(initial_state: bool) -> AutoResetEvent {
        AutoResetEvent { signaled: Mutex::new(initial_state), changed: Condvar::new() }
    }

    fn set(&self) {
        *lock(&self.signaled) = true;
        self.changed.notify_one();
    }

    fn wait_one(&self, timeout: Duration) -> bool {
        let signaled = lock(&self.signaled);
        let (mut signaled, _) = self
            .changed
            .wait_timeout_while(signaled, timeout, |signaled| !*signaled)
            .unwrap_or_else(PoisonError::into_inner);
        std::mem::replace(&mut *signaled, false)
    }
}

/// What `_lock` guards in the original, and the frame messages, which the
/// original reads and writes as references without it.
struct State {
    pending_socket: Option<Arc<SimpleWebSocket>>,
    last_frame_message: Option<Message>,
    last_sent_frame_message: Option<Message>,
    on_message: Delegate<Message>,
    on_exception: Delegate<Error>,
    // The handlers added to the signal transport (`OnSignalTransportMessage`
    // and `OnSignalTransportException` as delegates, which are removed with
    // the last handler of this connection).
    signal_on_message: Option<HandlerToken>,
    signal_on_exception: Option<HandlerToken>,
}

/// The connection of the `html` method of the previewer. It serves a page
/// over HTTP and talks to it over a web socket: the frames of the previewed
/// window go to the page, and the mouse input of the page arrives as the
/// input messages of the protocol. Everything else passes through the
/// connection it is given (the signal transport), which still carries the
/// messages of the IDE. One page is served at a time: a page that connects
/// takes the place of the one before it.
///
/// # Threads
///
/// Deviation (DEVIATIONS.md, Designer support): the original runs its
/// workers as tasks of the thread pool. Here they are threads, under the
/// threading contract of the protocol library (`i_transport.rs`): a thread
/// that accepts the requests, a thread that sends the frames, and a reader
/// thread per web socket. A message of the page is raised on the reader
/// thread of its socket and a message of the signal transport on the reader
/// thread of that connection, one event at a time over both; the handlers
/// post to the thread they belong to. The workers hold the connection, as
/// the tasks of the original do: it lives until it is disposed.
pub struct HtmlWebSocketTransport {
    this: Weak<HtmlWebSocketTransport>,
    signal_transport: Arc<dyn IFerroRemoteTransportConnection>,
    simple_server: SimpleWebSocketHttpServer,
    resources: HashMap<String, Vec<u8>>,
    disposed: AtomicBool,
    lock: Mutex<State>,
    // `_listenUri.OriginalString`, all that is read of the URI later.
    listen_uri: String,
    secret_cookie: Guid,
    wakeup: AutoResetEvent,
    // One event at a time, whichever thread raises it.
    raising: Mutex<()>,
}

impl HtmlWebSocketTransport {
    /// Starts the server on the address of `listen_uri` and tells the other
    /// end of the signal transport where the page is
    /// ([`HtmlTransportStartedMessage`]). The exceptions of the constructor
    /// of the original are the errors: a scheme that is not `http`, a host
    /// that is not an IP address, an address that cannot be listened on.
    pub fn new(
        signal_transport: Arc<dyn IFerroRemoteTransportConnection>,
        listen_uri: &Uri,
    ) -> Result<Arc<HtmlWebSocketTransport>, Error> {
        if listen_uri.scheme() != "http" {
            return Err(Error::Argument("URI scheme is not HTTP. (Parameter 'listenUri')".to_string()));
        }

        let secret_cookie = new_guid();
        let resources = RESOURCES
            .iter()
            .map(|(r, data)| {
                if *r == "index.html" {
                    let result_index_string =
                        String::from_utf8_lossy(data).replace("PREVIEWER_SECURITY_COOKIE", &secret_cookie.to_string());
                    return (r.to_string(), result_index_string.into_bytes());
                }
                (r.to_string(), data.to_vec())
            })
            .collect();

        // `listenUri.Host` and `listenUri.Port`, which is the port of the
        // scheme when the URI names none.
        let (host, port) = host_and_port_or(listen_uri, Some(80))
            .ok_or_else(|| Error::Format(format!("Invalid URI: no host in '{}'", listen_uri.original_string())))?;
        let address: IpAddr =
            host.parse().map_err(|_| Error::Format("An invalid IP address was specified.".to_string()))?;

        let simple_server = SimpleWebSocketHttpServer::new(address, port);
        simple_server.listen()?;
        let this = Arc::new_cyclic(|this| HtmlWebSocketTransport {
            this: this.clone(),
            signal_transport,
            simple_server,
            resources,
            disposed: AtomicBool::new(false),
            lock: Mutex::new(State {
                pending_socket: None,
                last_frame_message: None,
                last_sent_frame_message: None,
                on_message: Delegate::new(),
                on_exception: Delegate::new(),
                signal_on_message: None,
                signal_on_exception: None,
            }),
            listen_uri: listen_uri.original_string().to_string(),
            secret_cookie,
            wakeup: AutoResetEvent::new(false),
            raising: Mutex::new(()),
        });
        let worker = this.clone();
        std::thread::Builder::new().name("designer-html-accept".to_string()).spawn(move || worker.accept_worker())?;
        let worker = this.clone();
        std::thread::Builder::new().name("designer-html-frames".to_string()).spawn(move || worker.socket_worker())?;
        this.signal_transport
            .send(Arc::new(HtmlTransportStartedMessage { uri: Some(format!("http://{address}:{port}/")) }));
        Ok(this)
    }

    /// The address the server listens on, with the port the system assigned
    /// when the URI asked for port zero; `None` once the connection is
    /// disposed. (An addition of the port, for a caller that lets the system
    /// choose the port: the message and the origin the page has to state
    /// name the port of the URI, as in the original.)
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.simple_server.local_addr()
    }

    fn accept_worker(&self) {
        loop {
            // Deviation (DEVIATIONS.md, Designer support): the worker of the
            // original is an `async void` method without a `catch`, so an
            // exception in it (the listener that was stopped, a client that
            // went away before it was answered, an origin that is not the
            // one of the page) is an unhandled exception that ends the
            // process. Here the worker ends when the server no longer
            // listens, and the failure of one request is printed and does
            // not keep the next one from being served.
            let req = match self.simple_server.accept() {
                Ok(req) => req,
                Err(e) => {
                    if !self.disposed.load(Ordering::SeqCst) {
                        eprintln!("{e}");
                    }
                    return;
                }
            };

            if let Err(e) = self.serve(&req) {
                eprintln!("{e}");
            }
            // `using (var req = ...)`.
            req.dispose();
        }
    }

    /// The body of the loop of `AcceptWorker` for one request.
    fn serve(&self, req: &SimpleWebSocketHttpRequest) -> Result<(), Error> {
        if !req.is_websocket_request() {
            let key = if req.path() == "/" {
                "index.html".to_string()
            } else {
                req.path().trim_start_matches('/').replace('/', ".")
            };
            if let Some(data) = self.resources.get(&key) {
                // `Path.GetExtension(key).Substring(1)`.
                let ext = key.rsplit_once('.').map(|(_, ext)| ext);
                let mime = ext.and_then(mime).unwrap_or("application/octet-stream");
                req.respond(200, data, mime)?;
            } else {
                req.respond(404, NOT_FOUND, "text/plain")?;
            }
        } else if self.is_valid_origin(req) {
            let socket = Arc::new(req.accept_web_socket(None)?);
            let (this, reader) = (self.this.clone(), socket.clone());
            std::thread::Builder::new().name("designer-html-socket".to_string()).spawn(move || {
                if let Some(this) = this.upgrade() {
                    this.socket_receive_worker(&reader);
                }
            })?;
            let mut state = lock(&self.lock);
            if let Some(pending_socket) = state.pending_socket.take() {
                pending_socket.dispose();
            }
            state.pending_socket = Some(socket);
        } else {
            return Err(Error::InvalidOperation("Origin doesn't match Url".to_string()));
        }
        Ok(())
    }

    fn is_valid_origin(&self, request: &SimpleWebSocketHttpRequest) -> bool {
        request
            .headers()
            .get("Origin")
            .is_some_and(|origin| *origin == self.listen_uri || origin.starts_with("vscode-webview:"))
    }

    fn socket_receive_worker(&self, socket: &SimpleWebSocket) {
        // `catch (Exception e) { Console.Error.WriteLine(e.ToString()); }`:
        // the end of the stream of a page that went away, a message that
        // cannot be parsed and a handler that failed all end the worker.
        let receive = || -> Result<(), Error> {
            // A close frame in place of the cookie is the null reference of
            // the original.
            let cookie = socket.receive_message()?.ok_or(Error::NullReference)?;
            if cookie.as_string() != self.secret_cookie.to_string() {
                socket.dispose();
                return Ok(());
            }

            loop {
                let msg = socket.receive_message()?;
                if let Some(msg) = msg.filter(|msg| msg.is_text) {
                    let message = Self::parse_message(&msg.as_string())?;
                    if let Some(message) = message {
                        self.raise_message(&message)?;
                    }
                }
            }
        };
        if let Err(e) = receive() {
            eprintln!("{e}");
        }
    }

    fn socket_worker(&self) {
        let mut socket: Option<Arc<SimpleWebSocket>> = None;
        loop {
            if self.disposed.load(Ordering::SeqCst) {
                if let Some(socket) = &socket {
                    socket.dispose();
                }
                return;
            }

            let mut send_now: Option<Message> = None;
            {
                let mut state = lock(&self.lock);
                if let Some(pending_socket) = state.pending_socket.take() {
                    if let Some(socket) = &socket {
                        socket.dispose();
                    }
                    socket = Some(pending_socket);
                    state.last_sent_frame_message = None;
                }

                let same = match (&state.last_frame_message, &state.last_sent_frame_message) {
                    (Some(last), Some(sent)) => Arc::ptr_eq(last, sent),
                    (None, None) => true,
                    _ => false,
                };
                if !same {
                    send_now = state.last_frame_message.clone();
                    state.last_sent_frame_message = send_now.clone();
                }
            }

            if let (Some(send_now), Some(current)) = (&send_now, &socket) {
                // Deviation (DEVIATIONS.md, Designer support): in the
                // original an exception of a send leaves the loop, after
                // which no page is sent a frame again, although a page that
                // is closed or reloaded is the usual way for a send to
                // fail. Here the error is printed and the socket let go of;
                // the page that connects next is sent the last frame.
                if let Err(e) = Self::send_frame(current, send_now) {
                    eprintln!("{e}");
                    current.dispose();
                    socket = None;
                }
            }

            self.wakeup.wait_one(Duration::from_secs(1));
        }
    }

    /// A frame as the page reads it: a text message with its header, then a
    /// binary message with its pixels.
    fn send_frame(socket: &SimpleWebSocket, send_now: &Message) -> Result<(), Error> {
        let Some(send_now) = send_now.downcast_ref::<FrameMessage>() else {
            return Ok(());
        };
        socket.send_message_text(&format!(
            "frame:{}:{}:{}:{}:{}:{}",
            send_now.sequence_id, send_now.width, send_now.height, send_now.stride, send_now.dpi_x, send_now.dpi_y
        ))?;
        socket.send_message(false, send_now.data.as_deref().ok_or(Error::NullReference)?)
    }

    /// `_onMessage?.Invoke(this, message)`. A handler that panics is the
    /// handler that throws of the original.
    fn raise_message(&self, message: &Message) -> Result<(), Error> {
        let handlers = lock(&self.lock).on_message.snapshot();
        let _raising = lock(&self.raising);
        catch_unwind(AssertUnwindSafe(|| {
            for handler in &handlers {
                handler(self, message);
            }
        }))
        .map_err(|payload| {
            let message = if let Some(text) = payload.downcast_ref::<&str>() {
                (*text).to_string()
            } else if let Some(text) = payload.downcast_ref::<String>() {
                text.clone()
            } else {
                "A handler panicked.".to_string()
            };
            Error::Handler(message)
        })
    }

    fn on_signal_transport_message(&self, message: &Message) {
        let handlers = lock(&self.lock).on_message.snapshot();
        let _raising = lock(&self.raising);
        for handler in &handlers {
            handler(self, message);
        }
    }

    fn on_signal_transport_exception(&self, ex: &Error) {
        let handlers = lock(&self.lock).on_exception.snapshot();
        let _raising = lock(&self.raising);
        for handler in &handlers {
            handler(self, ex);
        }
    }

    /// A message of the page as the message of the protocol it stands for;
    /// `None` for a message of a kind the transport does not know. The
    /// exceptions of the original (a part that is missing, a number or a
    /// name that is not one) are the errors.
    fn parse_message(message: &str) -> Result<Option<Message>, Error> {
        let parts: Vec<&str> = message.split(':').collect();
        let part = |index: usize| {
            parts
                .get(index)
                .copied()
                .ok_or_else(|| Error::Argument("Index was outside the bounds of the array.".to_string()))
        };
        let key = parts[0];
        if key.eq_ignore_ascii_case("frame-received") {
            let sequence_id = part(1)?
                .trim()
                .parse()
                .map_err(|_| Error::Format(format!("The input string '{}' was not in a correct format.", parts[1])))?;
            return Ok(Some(Arc::new(FrameReceivedMessage { sequence_id })));
        } else if key.eq_ignore_ascii_case("pointer-released") {
            return Ok(Some(Arc::new(PointerReleasedEventMessage {
                base: Self::parse_pointer_event(part(1)?, part(2)?, part(3)?)?,
                button: Self::parse_mouse_button(part(4)?)?,
            })));
        } else if key.eq_ignore_ascii_case("pointer-pressed") {
            return Ok(Some(Arc::new(PointerPressedEventMessage {
                base: Self::parse_pointer_event(part(1)?, part(2)?, part(3)?)?,
                button: Self::parse_mouse_button(part(4)?)?,
            })));
        } else if key.eq_ignore_ascii_case("pointer-moved") {
            return Ok(Some(Arc::new(PointerMovedEventMessage {
                base: Self::parse_pointer_event(part(1)?, part(2)?, part(3)?)?,
            })));
        } else if key.eq_ignore_ascii_case("scroll") {
            return Ok(Some(Arc::new(ScrollEventMessage {
                base: Self::parse_pointer_event(part(1)?, part(2)?, part(3)?)?,
                delta_x: Self::parse_double(part(4)?)?,
                delta_y: Self::parse_double(part(5)?)?,
            })));
        }

        Ok(None)
    }

    /// `Modifiers`, `X` and `Y`, which every pointer message starts with.
    fn parse_pointer_event(modifiers: &str, x: &str, y: &str) -> Result<PointerEventMessageBase, Error> {
        let mut base = PointerEventMessageBase::default();
        base.modifiers = Self::parse_input_modifiers(modifiers)?;
        base.x = Self::parse_double(x)?;
        base.y = Self::parse_double(y)?;
        Ok(base)
    }

    fn parse_input_modifiers(modifiers_text: &str) -> Result<Option<Vec<InputModifiers>>, Error> {
        if modifiers_text.trim().is_empty() {
            return Ok(None);
        }
        modifiers_text
            .split(',')
            .map(|x| {
                Self::enum_parse(
                    x,
                    InputModifiers::from_value,
                    &[
                        ("Alt", InputModifiers::Alt),
                        ("Control", InputModifiers::Control),
                        ("Shift", InputModifiers::Shift),
                        ("Windows", InputModifiers::Windows),
                        ("LeftMouseButton", InputModifiers::LeftMouseButton),
                        ("RightMouseButton", InputModifiers::RightMouseButton),
                        ("MiddleMouseButton", InputModifiers::MiddleMouseButton),
                    ],
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    fn parse_mouse_button(button_text: &str) -> Result<MouseButton, Error> {
        if button_text.trim().is_empty() {
            return Ok(MouseButton::None);
        }
        Self::enum_parse(
            button_text,
            MouseButton::from_value,
            &[
                ("None", MouseButton::None),
                ("Left", MouseButton::Left),
                ("Right", MouseButton::Right),
                ("Middle", MouseButton::Middle),
            ],
        )
    }

    /// `Enum.Parse<T>(value, ignoreCase: true)`: the number of a member, as
    /// the page writes it, or its name in any case.
    ///
    /// Deviation (DEVIATIONS.md, Remote protocol): the original accepts a
    /// number that no member has; an enumeration of Rust cannot hold it.
    fn enum_parse<T: Copy>(value: &str, from_value: fn(i32) -> Option<T>, names: &[(&str, T)]) -> Result<T, Error> {
        let value = value.trim();
        let not_found = || Error::Argument(format!("Requested value '{value}' was not found."));
        if value.starts_with(|first: char| first.is_ascii_digit() || first == '-' || first == '+') {
            if let Ok(number) = value.parse::<i32>() {
                return from_value(number).ok_or_else(not_found);
            }
        }
        names.iter().find(|(name, _)| name.eq_ignore_ascii_case(value)).map(|(_, member)| *member).ok_or_else(not_found)
    }

    /// `double.Parse(text, NumberStyles.Float, CultureInfo.InvariantCulture)`.
    fn parse_double(text: &str) -> Result<f64, Error> {
        let number = text.trim();
        // The names the runtime writes and reads for the values that are
        // not numbers, which a page may send (a division by a scale of
        // zero).
        match number {
            "NaN" => return Ok(f64::NAN),
            "Infinity" | "+Infinity" | "\u{221e}" => return Ok(f64::INFINITY),
            "-Infinity" | "-\u{221e}" => return Ok(f64::NEG_INFINITY),
            _ => {}
        }
        match number.parse::<f64>() {
            // The parser of the standard library also reads the names in
            // other spellings, which the original does not.
            Ok(value) if value.is_finite() || number.contains(|c: char| c.is_ascii_digit()) => Ok(value),
            _ => Err(Error::Format(format!("The input string '{text}' was not in a correct format."))),
        }
    }
}

impl IFerroRemoteTransportConnection for HtmlWebSocketTransport {
    /// Stops the server and closes the socket of the page. The signal
    /// transport is not disposed, as in the original.
    fn dispose(&self) {
        self.disposed.store(true, Ordering::SeqCst);
        let pending_socket = lock(&self.lock).pending_socket.clone();
        if let Some(pending_socket) = pending_socket {
            pending_socket.dispose();
        }
        self.simple_server.dispose();
    }

    fn send(&self, data: Message) -> Task {
        if data.is::<FrameMessage>() {
            lock(&self.lock).last_frame_message = Some(data);
            self.wakeup.set();
            return Task::completed();
        }
        if data.is::<RequestViewportResizeMessage>() {
            return Task::completed();
        }
        self.signal_transport.send(data)
    }

    fn start(&self) {
        // On the calling thread, as in the original.
        let message: Message = Arc::new(ClientSupportedPixelFormatsMessage { formats: Some(vec![PixelFormat::Rgba8888]) });
        self.on_signal_transport_message(&message);
        self.signal_transport.start();
    }

    fn on_message(&self, handler: MessageHandler) -> HandlerToken {
        let (token, subscribe_to_inner) = {
            let mut state = lock(&self.lock);
            let subscribe_to_inner = state.on_message.is_null();
            (state.on_message.add(handler), subscribe_to_inner)
        };

        if subscribe_to_inner {
            let this = self.this.clone();
            let inner = self.signal_transport.on_message(message_handler(move |_, message| {
                if let Some(this) = this.upgrade() {
                    this.on_signal_transport_message(message);
                }
            }));
            lock(&self.lock).signal_on_message = Some(inner);
        }
        token
    }

    fn remove_on_message(&self, token: HandlerToken) {
        let inner = {
            let mut state = lock(&self.lock);
            state.on_message.remove(token);
            if state.on_message.is_null() {
                state.signal_on_message.take()
            } else {
                None
            }
        };
        if let Some(inner) = inner {
            self.signal_transport.remove_on_message(inner);
        }
    }

    fn on_exception(&self, handler: ExceptionHandler) -> HandlerToken {
        // The original adds its handler to the signal transport inside the
        // lock. A connection may raise what it has stashed while a handler
        // is added, on the thread that adds it, and the handler takes the
        // lock: it is added outside the lock here, as for the messages.
        let (token, subscribe_to_inner) = {
            let mut state = lock(&self.lock);
            let subscribe_to_inner = state.on_exception.is_null();
            (state.on_exception.add(handler), subscribe_to_inner)
        };

        if subscribe_to_inner {
            let this = self.this.clone();
            let inner = self.signal_transport.on_exception(exception_handler(move |_, ex| {
                if let Some(this) = this.upgrade() {
                    this.on_signal_transport_exception(ex);
                }
            }));
            lock(&self.lock).signal_on_exception = Some(inner);
        }
        token
    }

    fn remove_on_exception(&self, token: HandlerToken) {
        let inner = {
            let mut state = lock(&self.lock);
            state.on_exception.remove(token);
            if state.on_exception.is_null() {
                state.signal_on_exception.take()
            } else {
                None
            }
        };
        if let Some(inner) = inner {
            self.signal_transport.remove_on_exception(inner);
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project tests the transport only in
    // the tests of the page (`webapp/tests`, whose messages
    // `the_messages_of_the_tests_of_the_page_are_parsed` reads here).
    // Nothing here can wait forever: the client of the tests has a time
    // limit on every read and write, and every wait for another thread has
    // one.
    use super::*;
    use crate::remote::html_transport::simple_web_socket_http_server::sha1;
    use crate::remote::html_transport::simple_web_socket_http_server::tests::{TestClient, TIMEOUT};
    use crate::remote::remote_rendering_tests::{is_red, red_server, Services};
    use crate::remote::test_connection::TestConnection;
    use ferroui_base::utilities::UriKind;
    use ferroui_remote_protocol::viewport::{ClientViewportAllocatedMessage, MeasureViewportMessage};
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::{Receiver, RecvTimeoutError};
    use std::time::Instant;

    /// The origin a page of [`start_transport`] states: the URI as it was given.
    const LISTEN_URI: &str = "http://127.0.0.1:0";

    /// A transport on a port of the loopback interface the system chose,
    /// over a signal transport of the tests.
    fn start_transport() -> (Arc<HtmlWebSocketTransport>, Arc<TestConnection>, SocketAddr) {
        let signal = TestConnection::new();
        let listen_uri = Uri::new(LISTEN_URI, UriKind::Absolute).unwrap();
        let transport = HtmlWebSocketTransport::new(signal.clone(), &listen_uri).expect("the loopback interface can be listened on");
        let address = transport.local_addr().expect("the address of the server");
        (transport, signal, address)
    }

    /// The cookie of the page, read from the script of the page as served.
    fn cookie_of(address: SocketAddr) -> String {
        let (_, page) = TestClient::get(address, "/");
        let page = String::from_utf8(page).unwrap();
        let assignment = "window[\"ferroPreviewerSecurityCookie\"] = \"";
        let start = page.find(assignment).expect("the script that states the cookie") + assignment.len();
        page[start..start + 36].to_string()
    }

    /// A page that has connected and stated its cookie, and its frames.
    fn page(address: SocketAddr) -> (TestClient, Receiver<(bool, u8, Vec<u8>)>) {
        let cookie = cookie_of(address);
        let mut client = TestClient::upgrade(address, "/ws", Some(LISTEN_URI)).expect("the handshake");
        client.send_text(&cookie);
        let frames = client.frames();
        (client, frames)
    }

    /// The messages a handler of the connection receives from now on.
    fn messages_of(transport: &HtmlWebSocketTransport) -> (Arc<Mutex<Vec<Message>>>, HandlerToken) {
        let messages = Arc::new(Mutex::new(Vec::new()));
        let received = messages.clone();
        let token = transport.on_message(message_handler(move |_, message| received.lock().unwrap().push(message.clone())));
        (messages, token)
    }

    fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
        let deadline = Instant::now() + TIMEOUT;
        while !done() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn frame_message(sequence_id: i64, data: Vec<u8>) -> Message {
        Arc::new(FrameMessage {
            sequence_id,
            format: PixelFormat::Rgba8888,
            data: Some(data),
            width: 2,
            height: 1,
            stride: 8,
            dpi_x: 96.0,
            dpi_y: 192.0,
        })
    }

    fn parse<T: Clone + 'static>(message: &str) -> T {
        let parsed = HtmlWebSocketTransport::parse_message(message).expect("a message that parses").expect("a known message");
        parsed.downcast_ref::<T>().expect("the class of the message").clone()
    }

    fn pointer(modifiers: Option<Vec<InputModifiers>>, x: f64, y: f64) -> PointerEventMessageBase {
        let mut base = PointerEventMessageBase::default();
        base.modifiers = modifiers;
        base.x = x;
        base.y = y;
        base
    }

    #[test]
    fn the_messages_of_the_page_are_parsed_into_the_messages_of_the_protocol() {
        assert_eq!(FrameReceivedMessage { sequence_id: 42 }, parse("frame-received:42"));
        assert_eq!(
            PointerPressedEventMessage {
                base: pointer(Some(vec![InputModifiers::Control, InputModifiers::LeftMouseButton]), 10.5, 20.0),
                button: MouseButton::Left,
            },
            parse("pointer-pressed:1,4:10.5:20:1")
        );
        assert_eq!(
            PointerReleasedEventMessage { base: pointer(None, 1.0, -2.0), button: MouseButton::Right },
            parse("pointer-released::1:-2:2")
        );
        assert_eq!(PointerMovedEventMessage { base: pointer(None, 0.0, 1e3) }, parse("pointer-moved: :0:1e3"));
        assert_eq!(
            ScrollEventMessage { base: pointer(Some(vec![InputModifiers::Shift]), 3.0, 4.0), delta_x: -0.0, delta_y: 120.0 },
            parse("scroll:2:3:4:-0:120")
        );

        // The key in any case; members by their names in any case, as
        // `Enum.Parse` reads them, and with white space around them; a
        // button that is not stated; parts after the last one that is read.
        assert_eq!(
            PointerPressedEventMessage {
                base: pointer(Some(vec![InputModifiers::Alt, InputModifiers::MiddleMouseButton]), 1.0, 2.0),
                button: MouseButton::Middle,
            },
            parse("Pointer-Pressed:alt, MIDDLEMOUSEBUTTON: 1 :2:middle:more")
        );
        assert_eq!(
            PointerReleasedEventMessage { base: pointer(None, 1.0, 2.0), button: MouseButton::None },
            parse("POINTER-RELEASED::1:2: ")
        );
        assert!(parse::<PointerMovedEventMessage>("pointer-moved::NaN:Infinity").x.is_nan());
        assert_eq!(f64::NEG_INFINITY, parse::<PointerMovedEventMessage>("pointer-moved::0:-Infinity").y);

        // A message of another kind is none.
        for message in ["", "frame", "key-down:1", "frame-received"] {
            let parsed = HtmlWebSocketTransport::parse_message(message);
            assert_eq!(message == "frame-received", parsed.is_err(), "{message}");
            assert!(parsed.map_or(true, |parsed| parsed.is_none()), "{message}");
        }
    }

    #[test]
    fn a_message_of_the_page_that_is_not_well_formed_is_an_error() {
        for message in [
            // A part that is missing.
            "pointer-moved::1",
            "pointer-pressed::1:2",
            "scroll::1:2:3",
            // What is not a number.
            "frame-received:x",
            "pointer-moved::x:2",
            "pointer-moved::1:",
            "pointer-moved::1:inf",
            "scroll::1:2:3:nan",
            // What is not a member.
            "pointer-moved:Hyper:1:2",
            "pointer-moved:7:1:2",
            "pointer-moved:1,:1:2",
            "pointer-pressed::1:2:4",
            "pointer-pressed::1:2:Fourth",
        ] {
            assert!(HtmlWebSocketTransport::parse_message(message).is_err(), "{message}");
        }
    }

    // The messages the tests of the page expect its classes to write
    // (`webapp/tests/Models/InputEventTests.ts`: every modifier, the left
    // button, the position and the deltas of its events).
    #[test]
    fn the_messages_of_the_tests_of_the_page_are_parsed() {
        let modifiers = Some(vec![
            InputModifiers::Alt,
            InputModifiers::Control,
            InputModifiers::Shift,
            InputModifiers::Windows,
            InputModifiers::LeftMouseButton,
            InputModifiers::RightMouseButton,
            InputModifiers::MiddleMouseButton,
        ]);
        let base = pointer(modifiers, 0.3, 0.42);
        assert_eq!(PointerMovedEventMessage { base: base.clone() }, parse("pointer-moved:0,1,2,3,4,5,6:0.3:0.42"));
        assert_eq!(
            PointerPressedEventMessage { base: base.clone(), button: MouseButton::Left },
            parse("pointer-pressed:0,1,2,3,4,5,6:0.3:0.42:1")
        );
        assert_eq!(
            PointerReleasedEventMessage { base: base.clone(), button: MouseButton::Left },
            parse("pointer-released:0,1,2,3,4,5,6:0.3:0.42:1")
        );
        assert_eq!(
            ScrollEventMessage { base, delta_x: -3.0, delta_y: -3.0 },
            parse("scroll:0,1,2,3,4,5,6:0.3:0.42:-3:-3")
        );
    }

    #[test]
    fn the_listen_uri_has_to_be_an_http_uri_with_an_ip_address() {
        let new = |uri: &str| {
            let signal = TestConnection::new();
            let result = HtmlWebSocketTransport::new(signal.clone(), &Uri::new(uri, UriKind::Absolute).unwrap());
            // Nothing was started.
            assert!(signal.sent().is_empty());
            result.err().expect("an error")
        };
        assert!(matches!(new("https://127.0.0.1:8081"), Error::Argument(_)));
        assert!(matches!(new("tcp-bson://127.0.0.1:8081"), Error::Argument(_)));
        // The default of the entry point, as in the original.
        assert!(matches!(new("http://localhost:5000"), Error::Format(_)));

        // A port that is taken.
        let (transport, _, address) = start_transport();
        assert!(matches!(new(&format!("http://{address}")), Error::Io(_)));
        transport.dispose();
    }

    #[test]
    fn the_page_is_served_with_its_cookie_and_anything_else_is_not_found() {
        let (transport, signal, address) = start_transport();
        // The other end of the signal transport is told where the page is:
        // the address and the port of the URI.
        assert_eq!(
            vec![HtmlTransportStartedMessage { uri: Some("http://127.0.0.1:0/".to_string()) }],
            signal.sent_of::<HtmlTransportStartedMessage>()
        );

        let (head, page) = TestClient::get(address, "/");
        let page = String::from_utf8(page).unwrap();
        assert_eq!(
            format!("HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: text/html\r\nContent-Length: {}", page.len()),
            head
        );
        assert!(!page.contains("PREVIEWER_SECURITY_COOKIE"));
        let cookie = cookie_of(address);
        assert!(Guid::parse(&cookie).is_ok());
        assert_eq!(cookie, transport.secret_cookie.to_string());
        assert!(page.contains("<script type=\"module\" src=\"index.js\"></script>"));
        assert_eq!((head, page.into_bytes()), TestClient::get(address, "/index.html"));

        let (head, script) = TestClient::get(address, "/index.js");
        assert!(head.contains("\r\nContent-Type: text/javascript\r\n"), "{head}");
        assert!(script == include_bytes!("webapp/build/index.js"));

        // The cookie of another transport is another one.
        let (other, _, other_address) = start_transport();
        assert_ne!(cookie, cookie_of(other_address));
        other.dispose();

        for path in ["/index.css", "/index.js.map", "/ws", "/?a=b", "/index.html/", "//index.html.gz"] {
            let (head, body) = TestClient::get(address, path);
            assert_eq!(
                ("HTTP/1.1 404 NotFound\r\nConnection: close\r\nContent-Type: text/plain\r\nContent-Length: 15", &b"404 - Not Found"[..]),
                (head.as_str(), &body[..]),
                "{path}"
            );
        }
        // A slash of the path is a dot of the name of a resource.
        assert!(TestClient::get(address, "/index/html").0.starts_with("HTTP/1.1 200 OK\r\n"));
        transport.dispose();
    }

    #[test]
    fn what_is_not_a_frame_is_sent_through_the_signal_transport_and_its_events_are_forwarded() {
        let (transport, signal, _) = start_transport();
        // A frame and a request to resize the viewport stay here.
        assert!(transport.send(frame_message(1, vec![0; 8])).is_completed());
        assert!(transport.send(Arc::new(RequestViewportResizeMessage { width: 1.0, height: 2.0 })).is_completed());
        assert_eq!(1, signal.sent().len());
        transport.send(Arc::new(MeasureViewportMessage { width: 1.0, height: 2.0 }));
        assert_eq!(vec![MeasureViewportMessage { width: 1.0, height: 2.0 }], signal.sent_of::<MeasureViewportMessage>());

        // The first handler subscribes to the signal transport, the last
        // one that is removed unsubscribes.
        assert_eq!((0, 0), (signal.message_handlers(), signal.exception_handlers()));
        let (first, first_token) = messages_of(&transport);
        let (second, second_token) = messages_of(&transport);
        assert_eq!(1, signal.message_handlers());
        signal.raise_message(Arc::new(MeasureViewportMessage { width: 3.0, height: 4.0 }));
        assert_eq!(1, first.lock().unwrap().len());
        assert_eq!(1, second.lock().unwrap().len());

        // Starting states the pixel format of the page to the handlers and
        // starts the signal transport.
        assert_eq!(0, signal.started());
        transport.start();
        assert_eq!(1, signal.started());
        assert_eq!(
            Some(&ClientSupportedPixelFormatsMessage { formats: Some(vec![PixelFormat::Rgba8888]) }),
            first.lock().unwrap()[1].downcast_ref()
        );

        transport.remove_on_message(first_token);
        assert_eq!(1, signal.message_handlers());
        signal.raise_message(Arc::new(MeasureViewportMessage { width: 5.0, height: 6.0 }));
        assert_eq!((2, 3), (first.lock().unwrap().len(), second.lock().unwrap().len()));
        transport.remove_on_message(second_token);
        assert_eq!(0, signal.message_handlers());

        // The exceptions of the signal transport, with this connection as
        // their sender.
        let exceptions = Arc::new(Mutex::new(Vec::new()));
        let received = exceptions.clone();
        let this = Arc::as_ptr(&transport) as *const () as usize;
        let token = transport.on_exception(exception_handler(move |connection, e| {
            let sender = connection as *const dyn IFerroRemoteTransportConnection as *const () as usize;
            received.lock().unwrap().push((sender == this, e.to_string()));
        }));
        let other = transport.on_exception(exception_handler(|_, _| {}));
        assert_eq!(1, signal.exception_handlers());
        signal.raise_exception(Error::EndOfStream);
        assert_eq!(vec![(true, Error::EndOfStream.to_string())], *exceptions.lock().unwrap());
        transport.remove_on_exception(token);
        assert_eq!(1, signal.exception_handlers());
        transport.remove_on_exception(other);
        assert_eq!(0, signal.exception_handlers());

        // The signal transport is not disposed with the connection.
        transport.dispose();
        assert_eq!(0, signal.disposed());
    }

    /// The next frame of the server as the page reads it: the parts of its
    /// header and its pixels.
    fn next_frame(frames: &Receiver<(bool, u8, Vec<u8>)>) -> (Vec<String>, Vec<u8>) {
        let (end_of_message, frame_type, header) = frames.recv_timeout(TIMEOUT).expect("the header of a frame");
        assert_eq!((true, 1), (end_of_message, frame_type));
        let (end_of_message, frame_type, pixels) = frames.recv_timeout(TIMEOUT).expect("the pixels of a frame");
        assert_eq!((true, 2), (end_of_message, frame_type));
        (String::from_utf8(header).unwrap().split(':').map(str::to_string).collect(), pixels)
    }

    #[test]
    fn a_frame_is_sent_to_the_page_as_its_header_and_its_pixels_and_the_messages_of_the_page_arrive() {
        let (transport, signal, address) = start_transport();
        let (messages, _) = messages_of(&transport);
        // A frame that was sent before a page connected is the first frame
        // of the page.
        let pixels: Vec<u8> = (0..8).collect();
        transport.send(frame_message(7, pixels.clone()));
        let (mut client, frames) = page(address);
        let (header, received) = next_frame(&frames);
        assert_eq!("frame:7:2:1:8:96:192", header.join(":"));
        assert_eq!(pixels, received);

        // The page acknowledges the frame and sends its input.
        client.send_text("frame-received:7");
        client.send_text("pointer-moved:4:1.5:2.5");
        // A binary message and a message of a kind that is not known are
        // nothing; a ping is answered.
        client.send_frame(true, 2, &[1, 2, 3]);
        client.send_text("unknown:1");
        client.send_frame(true, 9, b"ping");
        client.send_text("scroll::1:2:0:-120");
        wait_until("the messages of the page", || messages.lock().unwrap().len() == 3);
        {
            let messages = messages.lock().unwrap();
            assert_eq!(Some(&FrameReceivedMessage { sequence_id: 7 }), messages[0].downcast_ref());
            assert_eq!(
                Some(&PointerMovedEventMessage { base: pointer(Some(vec![InputModifiers::LeftMouseButton]), 1.5, 2.5) }),
                messages[1].downcast_ref()
            );
            assert_eq!(
                Some(&ScrollEventMessage { base: pointer(None, 1.0, 2.0), delta_x: 0.0, delta_y: -120.0 }),
                messages[2].downcast_ref()
            );
        }
        assert_eq!((true, 10, b"ping".to_vec()), frames.recv_timeout(TIMEOUT).expect("the pong"));

        // The next frame, and only the last of the frames that were sent
        // since: a frame is not sent twice.
        transport.send(frame_message(8, vec![8; 8]));
        assert_eq!("8", next_frame(&frames).0[1]);
        transport.send(frame_message(9, vec![9; 8]));
        transport.send(frame_message(10, vec![10; 300]));
        let mut last = next_frame(&frames);
        if last.0[1] == "9" {
            last = next_frame(&frames);
        }
        assert_eq!(("10", vec![10u8; 300]), (last.0[1].as_str(), last.1));
        assert!(matches!(frames.recv_timeout(Duration::from_millis(1500)), Err(RecvTimeoutError::Timeout)));

        // Nothing of it went through the signal transport.
        assert!(signal.sent_of::<FrameMessage>().is_empty());

        // A message that cannot be parsed ends the reading of the socket,
        // as in the original; frames are still sent to it.
        client.send_text("pointer-moved::x:y");
        client.send_text("frame-received:10");
        transport.send(frame_message(11, vec![11; 8]));
        assert_eq!("11", next_frame(&frames).0[1]);
        assert_eq!(3, messages.lock().unwrap().len());

        // Disposing closes the socket of the page, and nothing listens.
        transport.dispose();
        assert!(matches!(frames.recv_timeout(TIMEOUT), Err(RecvTimeoutError::Disconnected)));
        assert_eq!(None, transport.local_addr());
    }

    #[test]
    fn a_page_that_connects_takes_the_place_of_the_page_before_it() {
        let (transport, _, address) = start_transport();
        transport.send(frame_message(1, vec![1; 8]));
        let (_first, first_frames) = page(address);
        assert_eq!("1", next_frame(&first_frames).0[1]);

        // The second page is sent the last frame again, and the socket of
        // the first is closed.
        let (second, second_frames) = page(address);
        assert_eq!("1", next_frame(&second_frames).0[1]);
        assert!(matches!(first_frames.recv_timeout(TIMEOUT), Err(RecvTimeoutError::Disconnected)));

        transport.send(frame_message(2, vec![2; 8]));
        assert_eq!("2", next_frame(&second_frames).0[1]);

        // A page that went away without closing: the frame that cannot be
        // sent to it does not keep the next page from its frames.
        second.close();
        drop(second_frames);
        for sequence_id in 3..6 {
            transport.send(frame_message(sequence_id, vec![3; 70000]));
            std::thread::sleep(Duration::from_millis(50));
        }
        let (_third, third_frames) = page(address);
        let mut frame = next_frame(&third_frames);
        while frame.0[1] != "5" {
            frame = next_frame(&third_frames);
        }
        assert_eq!(70000, frame.1.len());
        transport.dispose();
        assert!(matches!(third_frames.recv_timeout(TIMEOUT), Err(RecvTimeoutError::Disconnected)));
    }

    #[test]
    fn a_web_socket_is_accepted_by_its_origin_and_kept_by_its_cookie() {
        let (transport, _, address) = start_transport();
        let (messages, _) = messages_of(&transport);
        transport.send(frame_message(1, vec![1; 8]));

        // No origin, the origin of another page, and the origin of the page
        // written another way: the connection is closed without an answer.
        for origin in [None, Some("http://example.com"), Some("http://127.0.0.1:0/"), Some("HTTP://127.0.0.1:0")] {
            assert_eq!(Some(String::new()), TestClient::upgrade(address, "/ws", origin).err(), "{origin:?}");
        }
        // The server still serves.
        assert!(TestClient::get(address, "/").0.starts_with("HTTP/1.1 200 OK\r\n"));

        // A socket that states another cookie is closed, and what it sends
        // is no message.
        let mut stranger = TestClient::upgrade(address, "/ws", Some(LISTEN_URI)).expect("the handshake");
        let stranger_frames = stranger.frames();
        stranger.send_text("00000000-0000-4000-8000-000000000000");
        stranger.send_text("pointer-moved::1:2");
        wait_until("the socket of the stranger to be closed", || {
            matches!(stranger_frames.recv_timeout(Duration::from_millis(10)), Err(RecvTimeoutError::Disconnected))
        });
        assert!(messages.lock().unwrap().is_empty());

        // The page, and a view of an editor that shows the page, are
        // served, whatever the path of the request.
        let (_page, frames) = page(address);
        assert_eq!("1", next_frame(&frames).0[1]);
        let cookie = cookie_of(address);
        let mut view = TestClient::upgrade(address, "/", Some("vscode-webview://0123abcd")).expect("the handshake");
        let view_frames = view.frames();
        view.send_text(&cookie);
        assert_eq!("1", next_frame(&view_frames).0[1]);
        view.send_text("pointer-moved::1:2");
        wait_until("the message of the view", || messages.lock().unwrap().len() == 1);
        transport.dispose();
    }

    // The path of a frame from a top-level that is rendered to the page,
    // and of the input of the page back: a server top-level over the
    // connection renders its scene with Skia, a client written here fetches
    // the page, upgrades to a web socket and reads the frames as the script
    // of the page does, and sends a pointer event.
    #[test]
    fn a_page_receives_the_frames_of_a_server_and_its_pointer_events_arrive_as_input_messages() {
        let services = Services::start();
        let (transport, signal, address) = start_transport();
        let (messages, _) = messages_of(&transport);
        let connection: Arc<dyn IFerroRemoteTransportConnection> = transport.clone();
        let server = red_server(&connection);

        // The pixel format of the page, stated by the connection itself,
        // and the viewport, allocated by the IDE over the signal transport.
        transport.start();
        signal.raise_message(Arc::new(ClientViewportAllocatedMessage { width: 8.0, height: 4.0, dpi_x: 96.0, dpi_y: 96.0 }));

        let (mut client, frames) = page(address);
        // The frames as the page reads them: the header of a frame, then
        // its pixels, which the page acknowledges. The first frames may be
        // rendered before the content is laid out.
        let mut header: Option<Vec<String>> = None;
        let mut scene: Option<(Vec<String>, Vec<u8>)> = None;
        services.pump_until("the frame of the scene", || {
            while let Ok((end_of_message, frame_type, data)) = frames.try_recv() {
                assert!(end_of_message);
                if frame_type == 1 {
                    header = Some(String::from_utf8(data).unwrap().split(':').map(str::to_string).collect());
                } else {
                    let header = header.take().expect("the header of the frame before its pixels");
                    client.send_text(&format!("frame-received:{}", header[1]));
                    let frame = FrameMessage {
                        width: header[2].parse().unwrap(),
                        height: header[3].parse().unwrap(),
                        data: Some(data.clone()),
                        ..FrameMessage::default()
                    };
                    if is_red(&frame, 8, 4) {
                        scene = Some((header, data));
                    }
                }
            }
            scene.is_some()
        });
        let (header, pixels) = scene.unwrap();
        assert_eq!("frame", header[0]);
        assert_eq!(["8", "4", "32", "96", "96"], header[2..]);
        assert_eq!(32 * 4, pixels.len());
        // The acknowledgements arrived as messages of the protocol.
        let sequence_id: i64 = header[1].parse().unwrap();
        services.pump_until("the acknowledgement of the frame", || {
            messages.lock().unwrap().iter().any(|message| {
                message.downcast_ref::<FrameReceivedMessage>().is_some_and(|received| received.sequence_id == sequence_id)
            })
        });

        // A press of the left button over the scene, with the control key.
        client.send_text("pointer-pressed:1,4:3:2:1");
        services.pump_until("the pointer event of the page", || {
            messages.lock().unwrap().iter().any(|message| message.is::<PointerPressedEventMessage>())
        });
        let pressed = messages
            .lock()
            .unwrap()
            .iter()
            .find_map(|message| message.downcast_ref::<PointerPressedEventMessage>().cloned())
            .unwrap();
        assert_eq!(
            PointerPressedEventMessage {
                base: pointer(Some(vec![InputModifiers::Control, InputModifiers::LeftMouseButton]), 3.0, 2.0),
                button: MouseButton::Left,
            },
            pressed
        );
        // The frames went to the page and not to the IDE.
        assert!(signal.sent_of::<FrameMessage>().is_empty());

        server.dispose();
        transport.dispose();
        services.end();
        let deadline = Instant::now() + TIMEOUT;
        loop {
            match frames.recv_timeout(TIMEOUT) {
                Ok(_) => assert!(Instant::now() < deadline, "the socket of the page is still open"),
                Err(e) => break assert_eq!(RecvTimeoutError::Disconnected, e),
            }
        }
    }

    fn sources(directory: &Path, relative: &str, files: &mut Vec<(String, PathBuf)>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            let relative = format!("{relative}/{name}");
            if entry.file_type().unwrap().is_dir() {
                sources(&entry.path(), &relative, files);
            } else if !name.starts_with('.') {
                files.push((relative, entry.path()));
            }
        }
    }

    // The files of `webapp/build` are checked in. The digest next to them
    // is the one of the sources they were built from, as
    // `scripts/build-designer-webapp.sh` computes it: for `tsconfig.json`
    // and then every file of `src` in the order of the bytes of its path,
    // the path, a line feed and the bytes of the file.
    #[test]
    fn the_built_page_is_built_from_its_sources() {
        let webapp = Path::new(env!("CARGO_MANIFEST_DIR")).join("remote/html_transport/webapp");
        let mut files = Vec::new();
        sources(&webapp.join("src"), "src", &mut files);
        files.sort();
        files.insert(0, ("tsconfig.json".to_string(), webapp.join("tsconfig.json")));
        let mut data = Vec::new();
        for (relative, path) in &files {
            data.extend_from_slice(relative.as_bytes());
            data.push(b'\n');
            data.extend_from_slice(&std::fs::read(path).unwrap());
        }
        let digest: String = sha1(&data).iter().map(|byte| format!("{byte:02x}")).collect();
        let built_from = std::fs::read_to_string(webapp.join("build/sources.sha1")).unwrap();
        assert_eq!(
            built_from.trim(),
            digest,
            "the sources of the page changed since it was built: run scripts/build-designer-webapp.sh"
        );

        // The page that is embedded is the host page of the sources, and it
        // loads the bundle.
        assert!(std::fs::read(webapp.join("src/index.html")).unwrap() == include_bytes!("webapp/build/index.html"));
        assert!(RESOURCES.iter().all(|(_, data)| !data.is_empty()));
    }
}
