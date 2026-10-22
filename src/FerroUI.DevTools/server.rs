use crate::Browser;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Cursor, Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tungstenite::protocol::WebSocketConfig;
use tungstenite::{Message, WebSocket};

const MAX_CLIENTS: usize = 16;
const MAX_MESSAGE: usize = 1024 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
enum Operation {
    List,
    Connect(Option<String>),
    Message(u64, String),
    Poll(u64),
    Disconnect(u64),
}
struct Request {
    operation: Operation,
    reply: mpsc::SyncSender<Value>,
    deadline: Instant,
}
#[derive(Clone)]
struct Transport {
    sender: mpsc::SyncSender<Request>,
    stop: Arc<AtomicBool>,
}
impl Transport {
    fn request(&self, operation: Operation) -> io::Result<Value> {
        let (reply, receiver) = mpsc::sync_channel(1);
        let deadline = Instant::now() + REQUEST_TIMEOUT;
        self.sender
            .try_send(Request {
                operation,
                reply,
                deadline,
            })
            .map_err(|_| io::Error::other("UI request queue is full or closed"))?;
        loop {
            if self.stop.load(Ordering::Relaxed) || Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "UI dispatcher unavailable",
                ));
            }
            match receiver.recv_timeout(Duration::from_millis(50)) {
                Ok(v) => return Ok(v),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => return Err(io::Error::other("UI dispatcher closed")),
            }
        }
    }
}

/// An explicitly started localhost listener. Keep the handle on the UI thread for the lifetime
/// of the endpoint. Dropping it stops the listener, closes clients and removes highlights/logging.
pub struct DevToolsServer {
    address: SocketAddr,
    browser: Rc<RefCell<Browser>>,
    timer: Rc<dyn IDisposable>,
    stop: Arc<AtomicBool>,
    sockets: Arc<Mutex<HashMap<u64, TcpStream>>>,
    worker: Option<JoinHandle<()>>,
}
impl DevToolsServer {
    /// Start on 127.0.0.1. Port zero requests an available port. Must run on the UI thread after
    /// platform setup; windows opened later are discovered automatically.
    pub fn start(port: u16) -> io::Result<Self> {
        Self::with_browser(port, Browser::new())
    }
    pub fn with_browser(port: u16, browser: Browser) -> io::Result<Self> {
        Dispatcher::ui_thread().verify_access();
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let browser = Rc::new(RefCell::new(browser));
        let weak = Rc::downgrade(&browser);
        let (sender, receiver) = mpsc::sync_channel::<Request>(128);
        let timer = DispatcherTimer::run(
            move || {
                let Some(browser) = weak.upgrade() else {
                    return false;
                };
                // A command that runs a nested dispatcher frame (a modal drag loop, a synchronous
                // wait) lets this timer tick again inside it: the requests then stay queued.
                let Ok(mut browser) = browser.try_borrow_mut() else {
                    return true;
                };
                // Bound work per tick so requests cannot starve layout and input.
                for request in receiver.try_iter().take(32) {
                    if Instant::now() >= request.deadline {
                        continue;
                    }
                    let value = match request.operation {
                        Operation::List => browser.discovery(address.port()),
                        Operation::Connect(target) => match browser.connect(target.as_deref()) {
                            Ok(id) => json!({"connection":id}),
                            Err(e) => json!({"error":e}),
                        },
                        Operation::Message(id, message) => json!(browser.handle(id, &message)),
                        Operation::Poll(id) => json!(browser.poll(id)),
                        Operation::Disconnect(id) => {
                            browser.disconnect(id);
                            Value::Null
                        }
                    };
                    let _ = request.reply.send(value);
                }
                true
            },
            Duration::from_millis(10),
            DispatcherPriority::BACKGROUND,
        );
        let stop = Arc::new(AtomicBool::new(false));
        let transport = Transport {
            sender,
            stop: stop.clone(),
        };
        let sockets = Arc::new(Mutex::new(HashMap::new()));
        let active = sockets.clone();
        let worker = thread::Builder::new()
            .name("ferroui-cdp".into())
            .spawn(move || {
                let mut workers: Vec<JoinHandle<()>> = Vec::new();
                let mut next = 1;
                while !transport.stop.load(Ordering::Relaxed) {
                    let mut index = 0;
                    while index < workers.len() {
                        if workers[index].is_finished() {
                            let _ = workers.swap_remove(index).join();
                        } else {
                            index += 1;
                        }
                    }
                    match listener.accept() {
                        Ok((stream, _)) => {
                            if workers.len() >= MAX_CLIENTS {
                                let _ = stream.shutdown(Shutdown::Both);
                                continue;
                            }
                            let id = next;
                            next += 1;
                            if let Ok(clone) = stream.try_clone() {
                                active.lock().unwrap().insert(id, clone);
                            }
                            let active = active.clone();
                            let transport = transport.clone();
                            workers.push(thread::spawn(move || {
                                if let Err(error) = serve(stream, address.port(), &transport) {
                                    if std::env::var_os("FERROUI_CDP_TRACE").is_some() {
                                        eprintln!("CDP connection: {error}");
                                    }
                                }
                                active.lock().unwrap().remove(&id);
                            }));
                        }
                        // A connection reset before it was accepted, or descriptors running out,
                        // fails this accept only; the listener stays.
                        Err(_) => thread::sleep(Duration::from_millis(10)),
                    }
                }
                for worker in workers {
                    let _ = worker.join();
                }
            })?;
        Ok(Self {
            address,
            browser,
            timer,
            stop,
            sockets,
            worker: Some(worker),
        })
    }
    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }
    /// Register a window from a custom lifetime, on the UI thread.
    pub fn register(&self, root: &ferroui_base::Ref<ferroui_controls::TopLevel>) -> String {
        self.browser.borrow_mut().register(root)
    }
}
impl Drop for DevToolsServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.timer.dispose();
        for socket in self.sockets.lock().unwrap().values() {
            let _ = socket.shutdown(Shutdown::Both);
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

struct PrefixedStream {
    prefix: Cursor<Vec<u8>>,
    stream: TcpStream,
}
impl Read for PrefixedStream {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.prefix.position() < (self.prefix.get_ref().len() as u64) {
            self.prefix.read(buffer)
        } else {
            self.stream.read(buffer)
        }
    }
}
impl Write for PrefixedStream {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.stream.write(buffer)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

fn serve(mut stream: TcpStream, port: u16, transport: &Transport) -> io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut header = Vec::new();
    let mut byte = [0];
    // The whole head within a deadline, not only each byte, so slow clients cannot hold the slots.
    let started = Instant::now();
    while !header.ends_with(b"\r\n\r\n") {
        if header.len() >= 16 * 1024 {
            return http(&mut stream, 431, json!({"error":"Headers too large"}));
        }
        if started.elapsed() > Duration::from_secs(5) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "Request head too slow",
            ));
        }
        stream.read_exact(&mut byte)?;
        header.push(byte[0]);
    }
    let text = std::str::from_utf8(&header).map_err(io::Error::other)?;
    let mut lines = text.split("\r\n");
    let mut start = lines.next().unwrap_or("").split_whitespace();
    let method = start.next().unwrap_or("");
    let path = start.next().unwrap_or("").trim_end_matches('/');
    if method != "GET" {
        return http(&mut stream, 405, json!({"error":"GET required"}));
    }
    let mut headers = HashMap::new();
    for line in lines.filter(|l| !l.is_empty()) {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| io::Error::other("Malformed header"))?;
        if headers
            .insert(name.trim().to_ascii_lowercase(), value.trim().to_owned())
            .is_some()
        {
            return http(&mut stream, 400, json!({"error":"Duplicate header"}));
        }
    }
    // Reject DNS rebinding and browser pages attempting to operate a local UI.
    let allowed_hosts = [format!("127.0.0.1:{port}"), format!("localhost:{port}")];
    if !headers
        .get("host")
        .is_some_and(|h| allowed_hosts.contains(h))
    {
        return http(&mut stream, 403, json!({"error":"Invalid Host"}));
    }
    if headers
        .get("origin")
        .is_some_and(|o| o != "devtools://devtools" && o != "chrome-devtools://devtools")
    {
        return http(&mut stream, 403, json!({"error":"Origin is not allowed"}));
    }
    if headers
        .get("upgrade")
        .is_some_and(|v| v.eq_ignore_ascii_case("websocket"))
    {
        let target = if path == "/devtools/browser" {
            None
        } else if let Some(id) = path
            .strip_prefix("/devtools/page/")
            .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        {
            Some(id.to_owned())
        } else {
            return http(
                &mut stream,
                404,
                json!({"error":"Unknown WebSocket target"}),
            );
        };
        let result = transport.request(Operation::Connect(target))?;
        let Some(connection) = result["connection"].as_u64() else {
            return http(&mut stream, 404, result);
        };
        let stream = PrefixedStream {
            prefix: Cursor::new(header),
            stream,
        };
        let config = WebSocketConfig::default()
            .max_message_size(Some(MAX_MESSAGE))
            .max_frame_size(Some(MAX_MESSAGE));
        let result = match tungstenite::accept_with_config(stream, Some(config)) {
            Ok(mut socket) => {
                socket
                    .get_mut()
                    .stream
                    .set_read_timeout(Some(Duration::from_millis(100)))?;
                websocket(&mut socket, connection, transport)
            }
            Err(e) => Err(io::Error::other(e.to_string())),
        };
        // Always release UI-side session state, even after an unsuccessful handshake.
        let _ = transport.request(Operation::Disconnect(connection));
        result
    } else {
        match path {
            "/json" | "/json/" | "/json/list" => {
                http(&mut stream, 200, transport.request(Operation::List)?)
            }
            "/json/version" => http(&mut stream, 200, Browser::version(port)),
            "/json/protocol" => http(
                &mut stream,
                200,
                serde_json::from_str(include_str!("protocol.json"))
                    .expect("embedded protocol is valid"),
            ),
            _ => http(&mut stream, 404, json!({"error":"Unknown endpoint"})),
        }
    }
}
fn http(stream: &mut TcpStream, status: u16, body: Value) -> io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        431 => "Request Header Fields Too Large",
        _ => "Error",
    };
    let body = body.to_string();
    write!(stream,"HTTP/1.1 {status} {reason}\r\nContent-Type: application/json; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n{body}",body.len())?;
    stream.flush()
}
fn send(socket: &mut WebSocket<PrefixedStream>, messages: Value) -> io::Result<()> {
    if let Some(messages) = messages.as_array() {
        for message in messages {
            socket
                .send(Message::Text(message.to_string().into()))
                .map_err(|e| io::Error::other(e.to_string()))?;
        }
    }
    Ok(())
}
fn websocket(
    socket: &mut WebSocket<PrefixedStream>,
    connection: u64,
    transport: &Transport,
) -> io::Result<()> {
    let mut last_poll = Instant::now();
    while !transport.stop.load(Ordering::Relaxed) {
        match socket.read() {
            Ok(Message::Text(message)) => send(
                socket,
                transport.request(Operation::Message(connection, message.to_string()))?,
            )?,
            Ok(Message::Close(_)) => {
                let _ = socket.flush();
                break;
            }
            Ok(Message::Ping(_)) => {
                socket
                    .flush()
                    .map_err(|e| io::Error::other(e.to_string()))?;
            }
            Ok(Message::Binary(_)) => return Err(io::Error::other("CDP expects text frames")),
            Ok(_) => {}
            Err(tungstenite::Error::Io(e))
                if matches!(
                    e.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(io::Error::other(e.to_string())),
        }
        if last_poll.elapsed() >= Duration::from_millis(250) {
            send(socket, transport.request(Operation::Poll(connection))?)?;
            last_poll = Instant::now();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn endpoint() -> (
        SocketAddr,
        JoinHandle<()>,
        Arc<AtomicBool>,
        mpsc::Receiver<Request>,
    ) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::sync_channel(128);
        let stop = Arc::new(AtomicBool::new(false));
        let transport = Transport {
            sender,
            stop: stop.clone(),
        };
        let worker = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let _ = serve(stream, address.port(), &transport);
        });
        (address, worker, stop, receiver)
    }
    fn get(path: &str, headers: &str) -> String {
        let (address, worker, _, _receiver) = endpoint();
        let mut stream = TcpStream::connect(address).unwrap();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: {address}\r\n{headers}\r\n"
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        worker.join().unwrap();
        response
    }
    #[test]
    fn http_discovery_protocol_and_origin_rejection() {
        let version = get("/json/version", "");
        assert!(version.starts_with("HTTP/1.1 200"));
        assert!(version.contains("webSocketDebuggerUrl"));
        let schema = get("/json/protocol", "");
        assert!(schema.contains("getDocument"));
        assert!(schema.contains("dispatchMouseEvent"));
        assert!(get("/missing", "").starts_with("HTTP/1.1 404"));
        assert!(get("/json/version", "Origin: https://example.org\r\n").starts_with("HTTP/1.1 403"));
        let (address, worker, _, _receiver) = endpoint();
        let mut stream = TcpStream::connect(address).unwrap();
        write!(
            stream,
            "GET /json/version HTTP/1.1\r\nHost: attacker.example:{}\r\n\r\n",
            address.port()
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 403"));
        worker.join().unwrap();
    }
    #[test]
    fn websocket_round_trip_and_disconnect_release_ui_session() {
        let (address, worker, stop, receiver) = endpoint();
        let ui = thread::spawn(move || {
            let request = receiver.recv().unwrap();
            assert!(matches!(request.operation, Operation::Connect(None)));
            request.reply.send(json!({"connection":7})).unwrap();
            let request = receiver.recv().unwrap();
            let Operation::Message(id, message) = request.operation else {
                panic!("expected command")
            };
            assert_eq!(id, 7);
            assert_eq!(
                serde_json::from_str::<Value>(&message).unwrap()["method"],
                "DOM.getDocument"
            );
            request
                .reply
                .send(json!([{"id":1,"result":{"root":{"nodeId":1}}}]))
                .unwrap();
            loop {
                let request = receiver.recv().unwrap();
                match request.operation {
                    Operation::Disconnect(7) => {
                        request.reply.send(Value::Null).unwrap();
                        break;
                    }
                    Operation::Poll(7) => request.reply.send(json!([])).unwrap(),
                    _ => panic!("unexpected operation"),
                }
            }
        });
        let (mut ws, _) = tungstenite::connect(format!("ws://{address}/devtools/browser")).unwrap();
        ws.send(Message::Text(
            r#"{"id":1,"method":"DOM.getDocument"}"#.into(),
        ))
        .unwrap();
        let response = ws.read().unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(response.to_text().unwrap()).unwrap()["result"]["root"]
                ["nodeId"],
            1
        );
        ws.close(None).unwrap();
        let _ = ws.read();
        worker.join().unwrap();
        ui.join().unwrap();
        stop.store(true, Ordering::Relaxed);
    }
}
