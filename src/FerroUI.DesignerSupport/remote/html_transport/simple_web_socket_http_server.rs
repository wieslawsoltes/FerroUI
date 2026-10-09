//! A small HTTP server that answers `GET` requests and upgrades a request to
//! a web socket, with the framing of the web socket protocol (RFC 6455)
//! written by hand, as in the original.
//!
//! Deviation (DEVIATIONS.md, Designer support): the original is asynchronous
//! (`AcceptAsync`, `RespondAsync`, `SendMessage` and `ReceiveMessage` return
//! tasks, and an `AsyncLock` puts the senders and the receivers of a socket
//! in a queue). Here every call blocks the calling thread, as the transports
//! of the protocol library do, and the two locks are mutexes. What throws in
//! the original returns an error of the protocol library.

use ferroui_remote_protocol::{DisposeCallback, Error};
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `Stream.ReadAsync`: one read, repeated when a signal interrupted it.
fn read_some(from: &mut dyn Read, to: &mut [u8]) -> Result<usize, Error> {
    loop {
        match from.read(to) {
            Ok(read) => return Ok(read),
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(e) => return Err(e.into()),
        }
    }
}

pub struct SimpleWebSocketHttpServer {
    address: IpAddr,
    port: u16,
    listener: Mutex<Option<Arc<TcpListener>>>,
}

impl SimpleWebSocketHttpServer {
    pub fn new(address: IpAddr, port: u16) -> SimpleWebSocketHttpServer {
        SimpleWebSocketHttpServer { address, port, listener: Mutex::new(None) }
    }

    fn listener(&self) -> Option<Arc<TcpListener>> {
        lock(&self.listener).clone()
    }

    /// `AcceptAsync`: blocks until a client has sent a `GET` request, and
    /// returns it. A client that sends anything else is dropped. An error
    /// when the server does not listen (any more).
    pub fn accept(&self) -> Result<SimpleWebSocketHttpRequest, Error> {
        loop {
            let res = self.accept_impl()?;
            if let Some(res) = res {
                return Ok(res);
            }
        }
    }

    fn accept_impl(&self) -> Result<Option<SimpleWebSocketHttpRequest>, Error> {
        let not_listening = || Error::InvalidOperation("Currently not listening".to_string());
        let Some(listener) = self.listener() else {
            return Err(not_listening());
        };
        let (mut stream, _) = listener.accept()?;
        // `Dispose` while the accept was waiting: see `dispose`.
        if !self.listener().is_some_and(|current| Arc::ptr_eq(&current, &listener)) {
            return Err(not_listening());
        }

        fn read_line(stream: &mut TcpStream) -> Result<String, Error> {
            let mut read_buffer = [0u8; 1];
            let mut line_buffer = [0u8; 1024];
            let mut c = 0;
            while c < 1024 {
                if read_some(stream, &mut read_buffer)? == 0 {
                    return Err(Error::EndOfStream);
                }
                if read_buffer[0] == 10 {
                    if c == 0 {
                        return Ok(String::new());
                    }
                    if line_buffer[c - 1] == 13 {
                        c -= 1;
                    }
                    if c == 0 {
                        return Ok(String::new());
                    }

                    return Ok(String::from_utf8_lossy(&line_buffer[..c]).into_owned());
                }
                line_buffer[c] = read_buffer[0];
                c += 1;
            }

            // `InvalidDataException`.
            Err(Error::InvalidOperation("Header is too large".to_string()))
        }

        // `catch { return null; }` with the stream disposed: every error of
        // the request is a client that is dropped.
        let mut read_request = || -> Result<Option<(String, HashMap<String, String>)>, Error> {
            let mut headers = HashMap::new();
            let line = read_line(&mut stream)?;
            let sp: Vec<&str> = line.split(' ').collect();
            if sp.len() != 3 || !sp[2].starts_with("HTTP") || sp[0] != "GET" {
                return Ok(None);
            }
            let path = sp[1].to_string();

            loop {
                let line = read_line(&mut stream)?;
                if line.is_empty() {
                    break;
                }
                // A line without a colon is the `IndexOutOfRangeException`
                // of the original.
                let Some((name, value)) = line.split_once(':') else {
                    return Ok(None);
                };
                headers.insert(name.to_string(), value.trim_start().to_string());
            }
            Ok(Some((path, headers)))
        };

        match read_request() {
            Ok(Some((path, headers))) => Ok(Some(SimpleWebSocketHttpRequest::new(stream, path, headers))),
            Ok(None) | Err(_) => Ok(None),
        }
    }

    /// Starts listening. A port of zero lets the system choose
    /// ([`local_addr`](Self::local_addr)). An address that cannot be bound is
    /// an error, as `TcpListener.Start` throws.
    pub fn listen(&self) -> Result<(), Error> {
        let listener = TcpListener::bind(SocketAddr::new(self.address, self.port))?;
        *lock(&self.listener) = Some(Arc::new(listener));
        Ok(())
    }

    /// The address the server listens on, with the port the system assigned
    /// when the port asked for was zero. (An addition of the port, as
    /// `DisposableServer::local_addr` of the protocol library is.)
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.listener().and_then(|listener| listener.local_addr().ok())
    }

    pub fn dispose(&self) {
        let Some(listener) = lock(&self.listener).take() else {
            return;
        };
        // `_listener.Stop()`: a blocking accept of the standard library
        // cannot be stopped from another thread, so a thread that waits in
        // `accept` is woken with a connection and ends with the error of a
        // server that does not listen. The port is closed when that thread
        // has let go of the listener.
        if Arc::strong_count(&listener) > 1 {
            if let Ok(mut address) = listener.local_addr() {
                if address.ip().is_unspecified() {
                    let loopback = match address {
                        SocketAddr::V4(_) => IpAddr::V4(Ipv4Addr::LOCALHOST),
                        SocketAddr::V6(_) => IpAddr::V6(Ipv6Addr::LOCALHOST),
                    };
                    address.set_ip(loopback);
                }
                //Ignore
                let _ = TcpStream::connect_timeout(&address, Duration::from_secs(1));
            }
        }
    }
}

/// The name of a member of `HttpStatusCode`, which the status line of the
/// original carries in place of a reason phrase (`{(HttpStatusCode)code}`);
/// the number for a code the enumeration does not have. Of two names with
/// one number, the one the runtime prints.
fn http_status_code(code: i32) -> String {
    let name = match code {
        100 => "Continue",
        101 => "SwitchingProtocols",
        102 => "Processing",
        103 => "EarlyHints",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "NonAuthoritativeInformation",
        204 => "NoContent",
        205 => "ResetContent",
        206 => "PartialContent",
        207 => "MultiStatus",
        208 => "AlreadyReported",
        226 => "IMUsed",
        300 => "MultipleChoices",
        301 => "MovedPermanently",
        302 => "Found",
        303 => "SeeOther",
        304 => "NotModified",
        305 => "UseProxy",
        306 => "Unused",
        307 => "TemporaryRedirect",
        308 => "PermanentRedirect",
        400 => "BadRequest",
        401 => "Unauthorized",
        402 => "PaymentRequired",
        403 => "Forbidden",
        404 => "NotFound",
        405 => "MethodNotAllowed",
        406 => "NotAcceptable",
        407 => "ProxyAuthenticationRequired",
        408 => "RequestTimeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "LengthRequired",
        412 => "PreconditionFailed",
        413 => "RequestEntityTooLarge",
        414 => "RequestUriTooLong",
        415 => "UnsupportedMediaType",
        416 => "RequestedRangeNotSatisfiable",
        417 => "ExpectationFailed",
        421 => "MisdirectedRequest",
        422 => "UnprocessableEntity",
        423 => "Locked",
        424 => "FailedDependency",
        426 => "UpgradeRequired",
        428 => "PreconditionRequired",
        429 => "TooManyRequests",
        431 => "RequestHeaderFieldsTooLarge",
        451 => "UnavailableForLegalReasons",
        500 => "InternalServerError",
        501 => "NotImplemented",
        502 => "BadGateway",
        503 => "ServiceUnavailable",
        504 => "GatewayTimeout",
        505 => "HttpVersionNotSupported",
        506 => "VariantAlsoNegotiates",
        507 => "InsufficientStorage",
        508 => "LoopDetected",
        510 => "NotExtended",
        511 => "NetworkAuthenticationRequired",
        _ => return code.to_string(),
    };
    name.to_string()
}

/// `SHA1.ComputeHash`: the SHA-1 digest of RFC 3174, which the handshake of
/// a web socket is defined with.
///
/// Deviation (DEVIATIONS.md, Designer support): the original calls the
/// runtime library. The workspace has no crate with the digest, and the
/// handshake is its only use.
pub(crate) fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());

    for block in message.chunks_exact(64) {
        let mut w = [0u32; 80];
        for (t, word) in block.chunks_exact(4).enumerate() {
            w[t] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for t in 16..80 {
            w[t] = (w[t - 3] ^ w[t - 8] ^ w[t - 14] ^ w[t - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = h;
        for (t, word) in w.iter().enumerate() {
            let (f, k) = match t {
                0..=19 => ((b & c) | (!b & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let temp = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        for (state, value) in h.iter_mut().zip([a, b, c, d, e]) {
            *state = state.wrapping_add(value);
        }
    }

    let mut digest = [0u8; 20];
    for (bytes, word) in digest.chunks_exact_mut(4).zip(h) {
        bytes.copy_from_slice(&word.to_be_bytes());
    }
    digest
}

/// `Convert.ToBase64String`: the Base64 encoding of RFC 4648, with padding.
fn to_base64_string(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut text = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let group = (u32::from(chunk[0]) << 16)
            | (u32::from(chunk.get(1).copied().unwrap_or(0)) << 8)
            | u32::from(chunk.get(2).copied().unwrap_or(0));
        text.push(ALPHABET[(group >> 18) as usize & 63] as char);
        text.push(ALPHABET[(group >> 12) as usize & 63] as char);
        text.push(if chunk.len() > 1 { ALPHABET[(group >> 6) as usize & 63] as char } else { '=' });
        text.push(if chunk.len() > 2 { ALPHABET[group as usize & 63] as char } else { '=' });
    }
    text
}

/// The value of `Sec-WebSocket-Accept` for the key of a request.
fn handshake(websocket_key: &str) -> String {
    let handshake_source = format!("{websocket_key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    to_base64_string(&sha1(handshake_source.as_bytes()))
}

pub struct SimpleWebSocketHttpRequest {
    headers: HashMap<String, String>,
    path: String,
    stream: Mutex<Option<TcpStream>>,
    is_websocket_request: bool,
    web_socket_protocols: Option<Vec<String>>,
    websocket_key: Option<String>,
}

impl SimpleWebSocketHttpRequest {
    pub fn new(stream: TcpStream, path: String, headers: HashMap<String, String>) -> SimpleWebSocketHttpRequest {
        let mut is_websocket_request = false;
        let mut web_socket_protocols = None;
        let mut websocket_key = None;
        if headers.get("Connection").is_some_and(|h| h.contains("Upgrade"))
            && headers.get("Upgrade").is_some_and(|h| h == "websocket")
        {
            if let Some(key) = headers.get("Sec-WebSocket-Key") {
                websocket_key = Some(key.clone());
                is_websocket_request = true;
                web_socket_protocols = Some(match headers.get("Sec-WebSocket-Protocol") {
                    Some(h) => h.split(',').map(|x| x.trim().to_string()).collect(),
                    None => Vec::new(),
                });
            }
        }
        SimpleWebSocketHttpRequest {
            headers,
            path,
            stream: Mutex::new(Some(stream)),
            is_websocket_request,
            web_socket_protocols,
            websocket_key,
        }
    }

    /// The headers of the request, by their names as the client wrote them.
    pub fn headers(&self) -> &HashMap<String, String> {
        &self.headers
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn is_websocket_request(&self) -> bool {
        self.is_websocket_request
    }

    /// The protocols the client offers; `None` for a request that is not a
    /// web socket request.
    pub fn web_socket_protocols(&self) -> Option<&[String]> {
        self.web_socket_protocols.as_deref()
    }

    /// Answers the request and closes the connection. A request that was
    /// answered, upgraded or disposed is the null reference of the original.
    pub fn respond(&self, code: i32, data: &[u8], content_type: &str) -> Result<(), Error> {
        let headers = format!(
            "HTTP/1.1 {code} {}\r\nConnection: close\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\n\r\n",
            http_status_code(code),
            data.len()
        );
        let mut stream = lock(&self.stream);
        let Some(connection) = stream.as_mut() else {
            return Err(Error::NullReference);
        };
        connection.write_all(headers.as_bytes())?;
        connection.write_all(data)?;
        *stream = None;
        Ok(())
    }

    /// Answers the handshake of a web socket request, and returns the
    /// socket, which owns the connection from then on.
    pub fn accept_web_socket(&self, protocol: Option<&str>) -> Result<SimpleWebSocket, Error> {
        let handshake = handshake(self.websocket_key.as_deref().unwrap_or(""));
        let mut headers = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {handshake}\r\n"
        );
        if let Some(protocol) = protocol {
            headers.push_str(protocol);
            headers.push_str("\r\n");
        }
        headers.push_str("\r\n");
        let mut stream = lock(&self.stream);
        let Some(connection) = stream.as_mut() else {
            return Err(Error::NullReference);
        };
        connection.write_all(headers.as_bytes())?;

        let Some(s) = stream.take() else {
            return Err(Error::NullReference);
        };
        SimpleWebSocket::of_stream(s)
    }

    pub fn dispose(&self) {
        *lock(&self.stream) = None;
    }
}

const WEBSOCKET_INITIAL_HEADER_LENGTH: usize = 2;
const WEBSOCKET_LEN16_LENGTH: usize = 4;
const WEBSOCKET_LEN64_LENGTH: usize = 10;

const WEBSOCKET_LEN16_CODE: u8 = 126;
const WEBSOCKET_LEN64_CODE: u8 = 127;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum FrameType {
    Continue = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

struct Frame {
    data: Vec<u8>,
    end_of_message: bool,
    /// The four bits of the frame type, which may be none of [`FrameType`].
    frame_type: u8,
}

struct SendState {
    stream: Box<dyn Write + Send>,
    send_header_buffer: [u8; 10],
}

struct ReceiveState {
    stream: Box<dyn Read + Send>,
    receive_frame_stream: Vec<u8>,
    receive_message_stream: Vec<u8>,
    current_message_frame_type: u8,
    recv_header_buffer: [u8; 8],
    mask_buffer: [u8; 4],
    read_exact_buffer: [u8; 4096],
}

/// The server end of a web socket: messages as frames over a stream. A
/// frame that is sent is never masked and never fragmented; a received
/// frame may be both.
pub struct SimpleWebSocket {
    // `_stream = null`.
    disposed: AtomicBool,
    dispose_callback: DisposeCallback,
    // `_sendLock` and what it guards.
    send: Mutex<SendState>,
    // `_recvLock` and what it guards.
    recv: Mutex<ReceiveState>,
}

impl SimpleWebSocket {
    /// The socket over the two directions of a stream. The original takes
    /// the one `Stream` of the connection, which is read and written and
    /// which `Dispose` closes; here the directions are two handles and the
    /// callback closes the stream, as for a connection of the protocol
    /// library. A read that waits in another thread ends when the callback
    /// has closed the stream.
    pub(crate) fn new(
        input_stream: Box<dyn Read + Send>,
        output_stream: Box<dyn Write + Send>,
        dispose_callback: DisposeCallback,
    ) -> SimpleWebSocket {
        SimpleWebSocket {
            disposed: AtomicBool::new(false),
            dispose_callback,
            send: Mutex::new(SendState { stream: output_stream, send_header_buffer: [0; 10] }),
            recv: Mutex::new(ReceiveState {
                stream: input_stream,
                receive_frame_stream: Vec::new(),
                receive_message_stream: Vec::new(),
                current_message_frame_type: FrameType::Continue as u8,
                recv_header_buffer: [0; 8],
                mask_buffer: [0; 4],
                read_exact_buffer: [0; 4096],
            }),
        }
    }

    fn of_stream(stream: TcpStream) -> Result<SimpleWebSocket, Error> {
        let input = stream.try_clone()?;
        let output = stream.try_clone()?;
        Ok(SimpleWebSocket::new(
            Box::new(input),
            Box::new(output),
            Box::new(move || {
                let _ = stream.shutdown(Shutdown::Both);
            }),
        ))
    }

    pub fn dispose(&self) {
        if !self.disposed.swap(true, Ordering::SeqCst) {
            (self.dispose_callback)();
        }
    }

    /// `SendMessage(string text)`.
    pub fn send_message_text(&self, text: &str) -> Result<(), Error> {
        let data = text.as_bytes();
        self.send_message(true, data)
    }

    /// `SendMessage(bool isText, byte[] data)` and the overload with an
    /// offset and a length, which a slice is.
    pub fn send_message(&self, is_text: bool, data: &[u8]) -> Result<(), Error> {
        self.send_frame(if is_text { FrameType::Text } else { FrameType::Binary }, data)
    }

    fn send_frame(&self, frame_type: FrameType, data: &[u8]) -> Result<(), Error> {
        let mut l = lock(&self.send);
        if self.disposed.load(Ordering::SeqCst) {
            return Err(Error::NullReference);
        }
        let length = data.len();

        // The header of the original is a structure with the two lengths at
        // the same offset, copied to the buffer; the bytes are written here.
        let header_length;
        if length <= 125 {
            header_length = WEBSOCKET_INITIAL_HEADER_LENGTH;
            l.send_header_buffer[1] = length as u8;
        } else if length <= 0xffff {
            header_length = WEBSOCKET_LEN16_LENGTH;
            l.send_header_buffer[1] = WEBSOCKET_LEN16_CODE;
            l.send_header_buffer[2..4].copy_from_slice(&(length as u16).to_be_bytes());
        } else {
            header_length = WEBSOCKET_LEN64_LENGTH;
            l.send_header_buffer[1] = WEBSOCKET_LEN64_CODE;
            l.send_header_buffer[2..10].copy_from_slice(&(length as u64).to_be_bytes());
        }

        const END_OF_MESSAGE_BIT: u8 = 1 << 7;
        l.send_header_buffer[0] = END_OF_MESSAGE_BIT | (frame_type as u8 & 0xf);

        let SendState { stream, send_header_buffer } = &mut *l;
        stream.write_all(&send_header_buffer[..header_length])?;
        stream.write_all(data)?;
        stream.flush()?;
        Ok(())
    }

    fn read_frame(state: &mut ReceiveState) -> Result<Frame, Error> {
        state.receive_frame_stream.clear();
        Self::read_exact(&mut *state.stream, &mut state.recv_header_buffer[..2])?;
        let masked = (state.recv_header_buffer[1] & 0x80) != 0;
        let len0 = state.recv_header_buffer[1] & 0x7F;
        let end_of_message = (state.recv_header_buffer[0] & 0x80) != 0;
        let frame_type = state.recv_header_buffer[0] & 0xf;
        let length: i32;
        if len0 <= 125 {
            length = i32::from(len0);
        } else if len0 == WEBSOCKET_LEN16_CODE {
            Self::read_exact(&mut *state.stream, &mut state.recv_header_buffer[..2])?;
            length = i32::from(u16::from_be_bytes([state.recv_header_buffer[0], state.recv_header_buffer[1]]));
        } else {
            Self::read_exact(&mut *state.stream, &mut state.recv_header_buffer[..8])?;
            // `(int)(ulong)`: the low 32 bits, and a length that is negative
            // as an `int` reads nothing.
            length = u64::from_be_bytes(state.recv_header_buffer) as i32;
        }

        if masked {
            Self::read_exact(&mut *state.stream, &mut state.mask_buffer)?;
        }
        Self::read_exact_to_stream(state, length)?;
        let mut data = state.receive_frame_stream.clone();
        if masked {
            for (c, byte) in data.iter_mut().enumerate() {
                *byte ^= state.mask_buffer[c % 4];
            }
        }

        Ok(Frame { data, end_of_message, frame_type })
    }

    /// The next message of the client: the frames of a fragmented message
    /// put together, a ping answered with a pong on the way. `None` for a
    /// close frame (which is not answered).
    pub fn receive_message(&self) -> Result<Option<SimpleWebSocketMessage>, Error> {
        let mut state = lock(&self.recv);
        if self.disposed.load(Ordering::SeqCst) {
            return Err(Error::NullReference);
        }
        loop {
            let frame = Self::read_frame(&mut state)?;

            if frame.frame_type == FrameType::Close as u8 {
                return Ok(None);
            }
            if frame.frame_type == FrameType::Ping as u8 {
                self.send_frame(FrameType::Pong, &frame.data)?;
            }
            if frame.frame_type == FrameType::Text as u8 || frame.frame_type == FrameType::Binary as u8 {
                let is_text = frame.frame_type == FrameType::Text as u8;
                if state.receive_message_stream.is_empty() && frame.end_of_message {
                    return Ok(Some(SimpleWebSocketMessage { is_text, data: frame.data }));
                }

                state.receive_message_stream.extend_from_slice(&frame.data);
                state.current_message_frame_type = frame.frame_type;
            }
            if frame.frame_type == FrameType::Continue as u8 {
                let frame_type = state.current_message_frame_type;
                state.receive_message_stream.extend_from_slice(&frame.data);
                if frame.end_of_message {
                    let is_text = frame_type == FrameType::Text as u8;
                    let data = std::mem::take(&mut state.receive_message_stream);
                    return Ok(Some(SimpleWebSocketMessage { is_text, data }));
                }
            }
        }
    }

    /// `ReadExact(Stream from, MemoryStream to, int length)`.
    fn read_exact_to_stream(state: &mut ReceiveState, length: i32) -> Result<(), Error> {
        let mut length = length;
        while length > 0 {
            let to_read = (length as usize).min(state.read_exact_buffer.len());
            let read = read_some(&mut *state.stream, &mut state.read_exact_buffer[..to_read])?;
            state.receive_frame_stream.extend_from_slice(&state.read_exact_buffer[..read]);
            if read == 0 {
                return Err(Error::EndOfStream);
            }
            length -= read as i32;
        }
        Ok(())
    }

    /// `ReadExact(Stream from, byte[] to, int offset, int length)`.
    fn read_exact(from: &mut dyn Read, to: &mut [u8]) -> Result<(), Error> {
        let mut offset = 0;
        while offset < to.len() {
            let read = read_some(from, &mut to[offset..])?;
            if read == 0 {
                return Err(Error::EndOfStream);
            }
            offset += read;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SimpleWebSocketMessage {
    pub is_text: bool,
    pub data: Vec<u8>,
}

impl SimpleWebSocketMessage {
    pub fn as_string(&self) -> String {
        String::from_utf8_lossy(&self.data).into_owned()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    // Not from upstream: the upstream project has no test of the server. The
    // framing is tested against the examples of RFC 6455 and the digest and
    // the encoding of the handshake against those of RFC 3174 and RFC 4648.
    // Nothing here can wait forever: a stream of the tests is a buffer, or a
    // socket of the loopback interface with a time limit on every read and
    // write, and a result of another thread is waited for with a time limit.
    use super::*;
    use std::io::Cursor;
    use std::sync::mpsc::{channel, Receiver};

    pub(crate) const TIMEOUT: Duration = Duration::from_secs(10);

    /// The key of the example handshake of RFC 6455, section 1.3.
    const SAMPLE_KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";
    const SAMPLE_ACCEPT: &str = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";

    /// A client written for the tests: HTTP, then the client end of a web
    /// socket, over a socket with time limits.
    pub(crate) struct TestClient {
        stream: TcpStream,
    }

    impl TestClient {
        fn connect(address: SocketAddr) -> TestClient {
            let stream = TcpStream::connect_timeout(&address, TIMEOUT).expect("the server accepts a connection");
            stream.set_read_timeout(Some(TIMEOUT)).unwrap();
            stream.set_write_timeout(Some(TIMEOUT)).unwrap();
            TestClient { stream }
        }

        /// The head of a response: up to and without the empty line.
        fn read_head(&mut self) -> std::io::Result<String> {
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") {
                if self.stream.read(&mut byte)? == 0 {
                    break;
                }
                head.push(byte[0]);
            }
            Ok(String::from_utf8_lossy(&head).trim_end().to_string())
        }

        /// Sends a request as it is and returns the head and the body of the
        /// response, read until the server closes the connection.
        pub(crate) fn request(address: SocketAddr, request: &str) -> (String, Vec<u8>) {
            let mut client = TestClient::connect(address);
            client.stream.write_all(request.as_bytes()).unwrap();
            let head = client.read_head().unwrap_or_default();
            let mut body = Vec::new();
            let _ = client.stream.read_to_end(&mut body);
            (head, body)
        }

        /// `GET path`: the head and the body of the response.
        pub(crate) fn get(address: SocketAddr, path: &str) -> (String, Vec<u8>) {
            TestClient::request(address, &format!("GET {path} HTTP/1.1\r\nHost: {address}\r\n\r\n"))
        }

        /// The handshake of a web socket with the key of the example of the
        /// RFC. The error is the head of a response that is not the
        /// handshake (empty when the server closed the connection).
        pub(crate) fn upgrade(address: SocketAddr, path: &str, origin: Option<&str>) -> Result<TestClient, String> {
            let mut client = TestClient::connect(address);
            let origin = origin.map(|origin| format!("Origin: {origin}\r\n")).unwrap_or_default();
            let request = format!(
                "GET {path} HTTP/1.1\r\nHost: {address}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
                 Sec-WebSocket-Key: {SAMPLE_KEY}\r\n{origin}Sec-WebSocket-Version: 13\r\n\r\n"
            );
            client.stream.write_all(request.as_bytes()).unwrap();
            let head = client.read_head().unwrap_or_default();
            let expected = format!(
                "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {SAMPLE_ACCEPT}"
            );
            if head == expected {
                Ok(client)
            } else {
                Err(head)
            }
        }

        /// One frame, masked as a client has to.
        pub(crate) fn send_frame(&mut self, end_of_message: bool, frame_type: u8, data: &[u8]) {
            self.stream.write_all(&frame(end_of_message, frame_type, data, Some([0x37, 0xfa, 0x21, 0x3d]))).unwrap();
        }

        pub(crate) fn send_text(&mut self, text: &str) {
            self.send_frame(true, 1, text.as_bytes());
        }

        /// Closes the connection, as a page that goes away does.
        pub(crate) fn close(&self) {
            let _ = self.stream.shutdown(Shutdown::Both);
        }

        /// The frames of the server from now on (whether a frame ends its
        /// message, its type and its data), read by a thread that ends with
        /// the connection or when the server has sent nothing within the
        /// time limit; the receiver is disconnected then.
        pub(crate) fn frames(&self) -> Receiver<(bool, u8, Vec<u8>)> {
            let mut stream = self.stream.try_clone().unwrap();
            let (sender, frames) = channel();
            std::thread::spawn(move || {
                while let Ok(frame) = read_frame_of(&mut stream) {
                    if sender.send(frame).is_err() {
                        break;
                    }
                }
            });
            frames
        }
    }

    fn read_frame_of(stream: &mut TcpStream) -> std::io::Result<(bool, u8, Vec<u8>)> {
        let mut header = [0u8; 2];
        stream.read_exact(&mut header)?;
        // A server does not mask.
        assert_eq!(0, header[1] & 0x80);
        let length = match header[1] {
            126 => {
                let mut length = [0u8; 2];
                stream.read_exact(&mut length)?;
                u64::from(u16::from_be_bytes(length))
            }
            127 => {
                let mut length = [0u8; 8];
                stream.read_exact(&mut length)?;
                u64::from_be_bytes(length)
            }
            length => u64::from(length),
        };
        let mut data = vec![0u8; length as usize];
        stream.read_exact(&mut data)?;
        Ok((header[0] & 0x80 != 0, header[0] & 0xf, data))
    }

    /// The bytes of a frame, with the shortest form of its length.
    fn frame(end_of_message: bool, frame_type: u8, data: &[u8], mask: Option<[u8; 4]>) -> Vec<u8> {
        let mut bytes = vec![(if end_of_message { 0x80 } else { 0 }) | frame_type];
        let masked = if mask.is_some() { 0x80 } else { 0 };
        if data.len() <= 125 {
            bytes.push(masked | data.len() as u8);
        } else if data.len() <= 0xffff {
            bytes.push(masked | 126);
            bytes.extend_from_slice(&(data.len() as u16).to_be_bytes());
        } else {
            bytes.push(masked | 127);
            bytes.extend_from_slice(&(data.len() as u64).to_be_bytes());
        }
        match mask {
            Some(mask) => {
                bytes.extend_from_slice(&mask);
                bytes.extend(data.iter().enumerate().map(|(index, byte)| byte ^ mask[index % 4]));
            }
            None => bytes.extend_from_slice(data),
        }
        bytes
    }

    /// What a socket of the tests has written.
    #[derive(Clone, Default)]
    struct Written(Arc<Mutex<Vec<u8>>>);

    impl Write for Written {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Written {
        fn take(&self) -> Vec<u8> {
            std::mem::take(&mut *self.0.lock().unwrap())
        }
    }

    /// A socket that reads the given bytes and then the end of the stream.
    fn socket_over(received: Vec<u8>) -> (SimpleWebSocket, Written, Arc<AtomicBool>) {
        let written = Written::default();
        let closed = Arc::new(AtomicBool::new(false));
        let flag = closed.clone();
        let socket = SimpleWebSocket::new(
            Box::new(Cursor::new(received)),
            Box::new(written.clone()),
            Box::new(move || flag.store(true, Ordering::SeqCst)),
        );
        (socket, written, closed)
    }

    fn text(text: &str) -> Option<SimpleWebSocketMessage> {
        Some(SimpleWebSocketMessage { is_text: true, data: text.as_bytes().to_vec() })
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn the_digest_of_the_handshake_is_sha_1() {
        // RFC 3174, section 7.3, and the digest of no data.
        assert_eq!("da39a3ee5e6b4b0d3255bfef95601890afd80709", hex(&sha1(b"")));
        assert_eq!("a9993e364706816aba3e25717850c26c9cd0d89d", hex(&sha1(b"abc")));
        assert_eq!(
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
            hex(&sha1(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"))
        );
        // More than one block, and a length that leaves no room for the
        // length in its block.
        assert_eq!("34aa973cd4c4daa4f61eeb2bdbad27316534016f", hex(&sha1(&vec![b'a'; 1_000_000])));
        assert_eq!("c2db330f6083854c99d4b5bfb6e8f29f201be699", hex(&sha1(&[b'a'; 56])));
    }

    #[test]
    fn the_encoding_of_the_handshake_is_base64() {
        // RFC 4648, section 10.
        for (data, encoded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encoded, to_base64_string(data.as_bytes()));
        }
        assert_eq!("+/8=", to_base64_string(&[0xfb, 0xff]));
    }

    #[test]
    fn the_accept_key_of_the_example_handshake_of_the_rfc() {
        // RFC 6455, section 1.3.
        assert_eq!(SAMPLE_ACCEPT, handshake(SAMPLE_KEY));
    }

    #[test]
    fn the_example_frames_of_the_rfc_are_received() {
        // RFC 6455, section 5.7: a single-frame unmasked text message, a
        // single-frame masked text message, and a fragmented unmasked text
        // message.
        let mut received = vec![0x81, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f];
        received.extend_from_slice(&[0x81, 0x85, 0x37, 0xfa, 0x21, 0x3d, 0x7f, 0x9f, 0x4d, 0x51, 0x58]);
        received.extend_from_slice(&[0x01, 0x03, 0x48, 0x65, 0x6c]);
        received.extend_from_slice(&[0x80, 0x02, 0x6c, 0x6f]);
        let (socket, written, _) = socket_over(received);
        for _ in 0..3 {
            assert_eq!(text("Hello"), socket.receive_message().unwrap());
        }
        // The end of the stream is the exception of the original.
        assert!(matches!(socket.receive_message(), Err(Error::EndOfStream)));
        assert!(written.take().is_empty());
    }

    #[test]
    fn frames_of_the_three_length_forms_are_received_masked_and_unmasked() {
        // RFC 6455, section 5.7: 256 bytes and 64 KiB of binary data in one
        // unmasked frame each (`82 7E 0100`, `82 7F 0000000000010000`); and
        // every form with a mask, and at the ends of its range.
        for length in [0usize, 1, 125, 126, 256, 65535, 65536, 70000] {
            let data: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
            for mask in [None, Some([0x37, 0xfa, 0x21, 0x3d])] {
                let bytes = frame(true, 2, &data, mask);
                match length {
                    256 if mask.is_none() => assert_eq!([0x82, 0x7e, 0x01, 0x00], bytes[..4]),
                    65536 if mask.is_none() => assert_eq!([0x82, 0x7f, 0, 0, 0, 0, 0, 1, 0, 0], bytes[..10]),
                    _ => {}
                }
                let (socket, _, _) = socket_over(bytes);
                let message = socket.receive_message().unwrap().expect("a message");
                assert!(!message.is_text);
                assert!(message.data == data, "{length} bytes, mask {mask:?}");
            }
        }
    }

    #[test]
    fn messages_are_sent_as_single_unmasked_frames_of_the_three_length_forms() {
        let (socket, written, _) = socket_over(Vec::new());
        // RFC 6455, section 5.7: a single-frame unmasked text message.
        socket.send_message_text("Hello").unwrap();
        assert_eq!(vec![0x81, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f], written.take());

        for (length, header) in [
            (0usize, vec![0x82, 0x00]),
            (125, vec![0x82, 125]),
            (126, vec![0x82, 0x7e, 0x00, 0x7e]),
            (256, vec![0x82, 0x7e, 0x01, 0x00]),
            (65535, vec![0x82, 0x7e, 0xff, 0xff]),
            (65536, vec![0x82, 0x7f, 0, 0, 0, 0, 0, 1, 0, 0]),
        ] {
            let data: Vec<u8> = (0..length).map(|index| (index % 251) as u8).collect();
            socket.send_message(false, &data).unwrap();
            let bytes = written.take();
            assert_eq!(header, bytes[..header.len()]);
            assert!(bytes[header.len()..] == data, "{length} bytes");
        }
    }

    #[test]
    fn a_fragmented_message_is_put_together() {
        // A binary message in three frames, the last one without data, and
        // a text message in two masked frames.
        let mut received = frame(false, 2, &[1, 2, 3], None);
        received.extend(frame(false, 0, &[4, 5], None));
        received.extend(frame(true, 0, &[], None));
        received.extend(frame(false, 1, "Hel".as_bytes(), Some([1, 2, 3, 4])));
        received.extend(frame(true, 0, "lo".as_bytes(), Some([5, 6, 7, 8])));
        let (socket, _, _) = socket_over(received);
        assert_eq!(
            Some(SimpleWebSocketMessage { is_text: false, data: vec![1, 2, 3, 4, 5] }),
            socket.receive_message().unwrap()
        );
        assert_eq!(text("Hello"), socket.receive_message().unwrap());
    }

    #[test]
    fn a_ping_is_answered_by_a_pong_with_its_data() {
        // RFC 6455, section 5.7: an unmasked ping request and its response;
        // a masked ping between the frames of a fragmented message; a pong
        // of the client is ignored.
        let mut received = vec![0x89, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f];
        received.extend(frame(false, 1, "Hel".as_bytes(), None));
        received.extend(frame(true, 9, &[7, 8], Some([9, 9, 9, 9])));
        received.extend(frame(true, 10, &[1], None));
        received.extend(frame(true, 0, "lo".as_bytes(), None));
        let (socket, written, _) = socket_over(received);
        assert_eq!(text("Hello"), socket.receive_message().unwrap());
        assert_eq!(vec![0x8a, 0x05, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x8a, 0x02, 7, 8], written.take());
    }

    #[test]
    fn a_close_frame_is_no_message_and_is_not_answered() {
        // A close frame with a status code (RFC 6455, section 5.5.1), and
        // one after the frames of a message that is never finished.
        let mut received = frame(true, 8, &[0x03, 0xe8], Some([1, 2, 3, 4]));
        received.extend(frame(false, 1, "Hel".as_bytes(), None));
        received.extend(frame(true, 8, &[], None));
        let (socket, written, closed) = socket_over(received);
        assert_eq!(None, socket.receive_message().unwrap());
        assert_eq!(None, socket.receive_message().unwrap());
        assert!(written.take().is_empty());
        assert!(!closed.load(Ordering::SeqCst));
    }

    #[test]
    fn a_frame_of_an_unknown_type_is_skipped_and_a_frame_that_ends_early_is_the_end_of_the_stream() {
        let mut received = frame(true, 3, &[1, 2, 3], None);
        received.extend(frame(true, 1, "next".as_bytes(), None));
        // A frame that says five bytes and has two.
        received.extend_from_slice(&[0x81, 0x05, 0x48, 0x65]);
        let (socket, _, _) = socket_over(received);
        assert_eq!(text("next"), socket.receive_message().unwrap());
        assert!(matches!(socket.receive_message(), Err(Error::EndOfStream)));

        // A header that ends early.
        let (socket, _, _) = socket_over(vec![0x81]);
        assert!(matches!(socket.receive_message(), Err(Error::EndOfStream)));
        // A length of 64 bits that is negative as the `int` of the original
        // reads no data.
        let mut received = vec![0x82, 0x7f, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff];
        received.extend(frame(true, 1, "next".as_bytes(), None));
        let (socket, _, _) = socket_over(received);
        assert_eq!(Some(SimpleWebSocketMessage { is_text: false, data: Vec::new() }), socket.receive_message().unwrap());
        assert_eq!(text("next"), socket.receive_message().unwrap());
    }

    #[test]
    fn a_disposed_socket_closes_its_stream_once_and_neither_sends_nor_receives() {
        let (socket, written, closed) = socket_over(frame(true, 1, "Hello".as_bytes(), None));
        socket.dispose();
        assert!(closed.swap(false, Ordering::SeqCst));
        socket.dispose();
        assert!(!closed.load(Ordering::SeqCst));
        assert!(matches!(socket.send_message_text("Hello"), Err(Error::NullReference)));
        assert!(matches!(socket.receive_message(), Err(Error::NullReference)));
        assert!(written.take().is_empty());
    }

    #[test]
    fn a_message_as_a_string_is_its_data_as_utf_8() {
        let message = SimpleWebSocketMessage { is_text: true, data: "za\u{17c}\u{f3}\u{142}\u{107}".as_bytes().to_vec() };
        assert_eq!("za\u{17c}\u{f3}\u{142}\u{107}", message.as_string());
    }

    #[test]
    fn the_status_line_names_the_member_of_the_status_code() {
        assert_eq!("OK", http_status_code(200));
        assert_eq!("NotFound", http_status_code(404));
        assert_eq!("299", http_status_code(299));
    }

    /// A server on a port of the loopback interface the system chose.
    fn server() -> (Arc<SimpleWebSocketHttpServer>, SocketAddr) {
        let server = Arc::new(SimpleWebSocketHttpServer::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0));
        assert_eq!(None, server.local_addr());
        assert!(matches!(server.accept(), Err(Error::InvalidOperation(_))));
        server.listen().unwrap();
        let address = server.local_addr().expect("the address of a server that listens");
        (server, address)
    }

    /// Runs `client` on a thread of its own and returns its result, which
    /// is waited for with a time limit.
    fn client<T: Send + 'static>(client: impl FnOnce() -> T + Send + 'static) -> Receiver<T> {
        let (sender, result) = channel();
        std::thread::spawn(move || {
            let _ = sender.send(client());
        });
        result
    }

    #[test]
    fn a_get_request_is_accepted_and_answered() {
        let (server, address) = server();
        let response = client(move || {
            TestClient::request(address, "GET /a/b.html HTTP/1.1\r\nHost: here\r\nX-Empty:\r\nX-Colon:  a:b\r\n\r\n")
        });
        let request = server.accept().unwrap();
        assert_eq!("/a/b.html", request.path());
        assert!(!request.is_websocket_request());
        assert_eq!(None, request.web_socket_protocols());
        assert_eq!(3, request.headers().len());
        assert_eq!(Some("here"), request.headers().get("Host").map(String::as_str));
        assert_eq!(Some(""), request.headers().get("X-Empty").map(String::as_str));
        assert_eq!(Some("a:b"), request.headers().get("X-Colon").map(String::as_str));
        request.respond(200, b"<p>page</p>", "text/html").unwrap();
        // The request is answered once.
        assert!(matches!(request.respond(200, b"", "text/html"), Err(Error::NullReference)));
        assert!(matches!(request.accept_web_socket(None), Err(Error::NullReference)));
        request.dispose();

        let (head, body) = response.recv_timeout(TIMEOUT).expect("the response");
        assert_eq!("HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: text/html\r\nContent-Length: 11", head);
        assert_eq!(b"<p>page</p>".to_vec(), body);

        // Lines that end with a line feed only, and a status that is not 200.
        let response = client(move || TestClient::request(address, "GET / HTTP/1.0\nA: b\n\n"));
        let request = server.accept().unwrap();
        assert_eq!("/", request.path());
        assert_eq!(Some("b"), request.headers().get("A").map(String::as_str));
        request.respond(404, b"404 - Not Found", "text/plain").unwrap();
        let (head, body) = response.recv_timeout(TIMEOUT).expect("the response");
        assert_eq!("HTTP/1.1 404 NotFound\r\nConnection: close\r\nContent-Type: text/plain\r\nContent-Length: 15", head);
        assert_eq!(b"404 - Not Found".to_vec(), body);
        server.dispose();
    }

    #[test]
    fn a_client_that_sends_no_get_request_is_dropped_and_the_next_one_is_accepted() {
        let (server, address) = server();
        let responses = client(move || {
            let long_line = format!("GET / HTTP/1.1\r\nX: {}\r\n\r\n", "x".repeat(2000));
            let dropped = [
                // Another method, a request line that is not one, a header
                // without a colon, a line of more than 1024 bytes, and a
                // client that closes before the end of the headers.
                "POST / HTTP/1.1\r\n\r\n",
                "GET /\r\n\r\n",
                "GET / FTP/1.1\r\n\r\n",
                "GET / HTTP/1.1\r\nno colon\r\n\r\n",
                long_line.as_str(),
            ]
            .map(|request| TestClient::request(address, request));
            drop(TestClient::connect(address));
            (dropped, TestClient::get(address, "/next"))
        });
        let request = server.accept().unwrap();
        assert_eq!("/next", request.path());
        request.respond(200, b"next", "text/plain").unwrap();
        let (dropped, (head, body)) = responses.recv_timeout(TIMEOUT).expect("the responses");
        for (head, body) in dropped {
            assert_eq!((String::new(), Vec::new()), (head, body));
        }
        assert!(head.starts_with("HTTP/1.1 200 OK\r\n"));
        assert_eq!(b"next".to_vec(), body);
        server.dispose();
    }

    #[test]
    fn a_web_socket_request_is_upgraded_and_messages_pass_both_ways() {
        let (server, address) = server();
        let exchange = client(move || {
            let mut client = TestClient::upgrade(address, "/ws", None).expect("the handshake of the example of the RFC");
            let frames = client.frames();
            client.send_text("Hello");
            client.send_frame(true, 9, b"ping");
            client.send_frame(false, 2, &[1, 2]);
            client.send_frame(true, 0, &[3]);
            let received: Vec<_> = (0..3).map(|_| frames.recv_timeout(TIMEOUT).expect("a frame of the server")).collect();
            client.send_frame(true, 8, &[]);
            // The server disposes its socket: the frames end.
            (received, frames.recv_timeout(TIMEOUT).is_err())
        });

        let request = server.accept().unwrap();
        assert_eq!("/ws", request.path());
        assert!(request.is_websocket_request());
        assert_eq!(Some(&[][..]), request.web_socket_protocols());
        let socket = request.accept_web_socket(None).unwrap();
        // The connection belongs to the socket now.
        assert!(matches!(request.respond(200, b"", "text/plain"), Err(Error::NullReference)));
        request.dispose();

        assert_eq!(text("Hello"), socket.receive_message().unwrap());
        socket.send_message_text("frame:1").unwrap();
        // The ping is answered while the next message is received.
        assert_eq!(Some(SimpleWebSocketMessage { is_text: false, data: vec![1, 2, 3] }), socket.receive_message().unwrap());
        socket.send_message(false, &vec![7u8; 300]).unwrap();
        assert_eq!(None, socket.receive_message().unwrap());
        socket.dispose();

        let (received, ended) = exchange.recv_timeout(TIMEOUT).expect("the frames the client received");
        assert_eq!((true, 1, b"frame:1".to_vec()), received[0]);
        assert_eq!((true, 10, b"ping".to_vec()), received[1]);
        assert_eq!((true, 2, vec![7u8; 300]), received[2]);
        assert!(ended);
        server.dispose();
    }

    #[test]
    fn a_request_is_a_web_socket_request_by_its_headers() {
        let (server, address) = server();
        let requests = [
            // The protocols of the client, and a connection header with
            // more than the upgrade.
            "GET / HTTP/1.1\r\nConnection: keep-alive, Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Key: k\r\nSec-WebSocket-Protocol: a, b\r\n\r\n",
            // No key; another upgrade; no connection header; header names
            // in another case.
            "GET / HTTP/1.1\r\nConnection: Upgrade\r\nUpgrade: websocket\r\n\r\n",
            "GET / HTTP/1.1\r\nConnection: Upgrade\r\nUpgrade: h2c\r\nSec-WebSocket-Key: k\r\n\r\n",
            "GET / HTTP/1.1\r\nUpgrade: websocket\r\nSec-WebSocket-Key: k\r\n\r\n",
            "GET / HTTP/1.1\r\nconnection: Upgrade\r\nupgrade: websocket\r\nsec-websocket-key: k\r\n\r\n",
        ];
        let responses = client(move || requests.map(|request| TestClient::request(address, request)));
        let request = server.accept().unwrap();
        assert!(request.is_websocket_request());
        assert_eq!(Some(&["a".to_string(), "b".to_string()][..]), request.web_socket_protocols());
        // The protocol is written as a line of its own, as it is.
        request.accept_web_socket(Some("Sec-WebSocket-Protocol: a")).unwrap().dispose();
        for _ in 1..requests.len() {
            let request = server.accept().unwrap();
            assert!(!request.is_websocket_request());
            assert_eq!(None, request.web_socket_protocols());
            request.dispose();
        }
        let responses = responses.recv_timeout(TIMEOUT).expect("the responses");
        assert_eq!(
            format!(
                "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\nSec-WebSocket-Protocol: a",
                handshake("k")
            ),
            responses[0].0
        );
        server.dispose();
    }

    #[test]
    fn a_disposed_server_ends_a_waiting_accept_and_closes_its_port() {
        let (server, address) = server();
        let waiting = server.clone();
        let accepted = client(move || waiting.accept().map(|_| ()));
        // Let the thread reach its accept; a dispose before it does ends it
        // the same way.
        std::thread::sleep(Duration::from_millis(100));
        server.dispose();
        assert!(matches!(
            accepted.recv_timeout(TIMEOUT).expect("the accept ends"),
            Err(Error::InvalidOperation(_))
        ));
        assert_eq!(None, server.local_addr());
        assert!(matches!(server.accept(), Err(Error::InvalidOperation(_))));
        server.dispose();

        // Nothing listens on the port any more.
        let deadline = std::time::Instant::now() + TIMEOUT;
        while TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok() {
            assert!(std::time::Instant::now() < deadline, "the port is still open");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
