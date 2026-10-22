use crate::{
    protocol::{self, Session},
    Client,
};
use serde_json::Value;
use std::{
    collections::HashMap,
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
const MAX_MESSAGE: usize = 1024 * 1024;
const MAX_WORKERS: usize = 16;

fn spawn_request(
    client: Client,
    session: Arc<Session>,
    message: Value,
    writer: Arc<Mutex<std::fs::File>>,
    workers: Arc<AtomicUsize>,
) {
    if workers.fetch_add(1, Ordering::AcqRel) >= MAX_WORKERS {
        workers.fetch_sub(1, Ordering::Release);
        if let Some(id) = message.get("id") {
            let _ = writeln!(
                writer.lock().unwrap(),
                "{}",
                protocol::error(id.clone(), -32000, "Too many requests")
            );
        }
        return;
    }
    std::thread::spawn(move || {
        if let Some(response) = session.handle(&client, message) {
            let mut output = writer.lock().unwrap();
            let _ = writeln!(output, "{response}");
            let _ = output.flush();
        }
        workers.fetch_sub(1, Ordering::Release);
    });
}
pub fn stdio(client: Client, protocol_output: std::fs::File) -> io::Result<()> {
    let session = Arc::new(Session::default());
    let output = Arc::new(Mutex::new(protocol_output));
    let workers = Arc::new(AtomicUsize::new(0));
    let logger_client = client.clone();
    let logger_session = session.clone();
    let logger_output = output.clone();
    std::thread::spawn(move || {
        let mut cursor = 0;
        while !logger_client.stop.load(Ordering::Acquire) {
            for message in logger_session.logs(&logger_client, &mut cursor) {
                let mut out = logger_output.lock().unwrap();
                let _ = writeln!(out, "{message}");
                let _ = out.flush();
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    let mut input = io::stdin().lock();
    while !client.stop.load(Ordering::Acquire) {
        let mut bytes = Vec::new();
        let n = input
            .by_ref()
            .take((MAX_MESSAGE + 1) as u64)
            .read_until(b'\n', &mut bytes)?;
        if n == 0 {
            break;
        }
        if n > MAX_MESSAGE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "MCP message exceeds 1 MiB",
            ));
        }
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(message) if message.get("id").is_none() => {
                if let Some(response) = session.handle(&client, message) {
                    let mut out = output.lock().unwrap();
                    writeln!(out, "{response}")?;
                    out.flush()?;
                }
            }
            Ok(message) => spawn_request(
                client.clone(),
                session.clone(),
                message,
                output.clone(),
                workers.clone(),
            ),
            Err(_) => {
                let mut out = output.lock().unwrap();
                writeln!(
                    out,
                    "{}",
                    protocol::error(Value::Null, -32700, "Parse error")
                )?;
                out.flush()?;
            }
        }
    }
    client.stop.store(true, Ordering::Release);
    Ok(())
}
// HTTP/1.1 on the standard library: one request per connection (`Connection: close`). The head is
// read with a size limit and the body only after its Content-Length has been checked against the
// message limit, on a worker thread with read timeouts, so no client can stall the accept loop or
// make the server allocate more than a message.
const MAX_HEAD: u64 = 16 * 1024;
const MAX_SESSIONS: usize = 64;
const SESSION_IDLE: Duration = Duration::from_secs(1800);
const IO_TIMEOUT: Duration = Duration::from_secs(10);

struct Request {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}
impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(field, _)| field.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}
/// A request that cannot be read is answered with the status of the error, if any (an I/O failure
/// has none: the connection is closed).
fn read_request(stream: &mut TcpStream) -> Result<Request, Option<u16>> {
    let mut reader = BufReader::new(stream.try_clone().map_err(|_| None)?);
    let mut head = (&mut reader).take(MAX_HEAD);
    let mut line = Vec::new();
    let mut read_line = |line: &mut Vec<u8>| -> Result<String, Option<u16>> {
        line.clear();
        match head.read_until(b'\n', line) {
            Ok(_) if line.ends_with(b"\n") => String::from_utf8(std::mem::take(line))
                .map(|text| text.trim_end_matches(['\r', '\n']).to_owned())
                .map_err(|_| Some(400)),
            // The limit ended the head before its end.
            Ok(n) if n > 0 => Err(Some(431)),
            _ => Err(None),
        }
    };
    let request_line = read_line(&mut line)?;
    let mut parts = request_line.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(Some(400));
    };
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(Some(400));
    }
    let mut headers = Vec::new();
    loop {
        let field = read_line(&mut line)?;
        if field.is_empty() {
            break;
        }
        let Some((name, value)) = field.split_once(':') else {
            return Err(Some(400));
        };
        headers.push((name.trim().to_owned(), value.trim().to_owned()));
    }
    let mut request = Request {
        method: method.to_owned(),
        target: target.to_owned(),
        headers,
        body: Vec::new(),
    };
    if request.header("Transfer-Encoding").is_some() {
        return Err(Some(411));
    }
    let length = match request.header("Content-Length") {
        None => 0,
        Some(value) => value.parse::<usize>().map_err(|_| Some(400))?,
    };
    if length > MAX_MESSAGE {
        return Err(Some(413));
    }
    if length > 0 {
        if request
            .header("Expect")
            .is_some_and(|e| e.eq_ignore_ascii_case("100-continue"))
        {
            let _ = stream.write_all(b"HTTP/1.1 100 Continue\r\n\r\n");
        }
        request.body = vec![0; length];
        reader.read_exact(&mut request.body).map_err(|_| None)?;
    }
    Ok(request)
}
fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        409 => "Conflict",
        411 => "Length Required",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        431 => "Request Header Fields Too Large",
        503 => "Service Unavailable",
        _ => "Internal Server Error",
    }
}
fn respond_with(stream: &mut TcpStream, status: u16, extra: &[(&str, &str)], body: &str) {
    let mut head = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        reason(status),
        body.len()
    );
    for (name, value) in extra {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("\r\n");
    let _ = stream
        .write_all(head.as_bytes())
        .and_then(|_| stream.write_all(body.as_bytes()))
        .and_then(|_| stream.flush());
}
fn respond(stream: &mut TcpStream, status: u16, body: String) {
    if status == 401 {
        respond_with(stream, status, &[("WWW-Authenticate", "Bearer")], &body);
    } else {
        respond_with(stream, status, &[], &body);
    }
}
/// Compares the whole of both strings, so the time does not tell how much of a token matched.
fn same_secret(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}
struct Policy {
    token: String,
    hosts: Vec<String>,
    origins: Vec<String>,
}
type Sessions = Arc<Mutex<HashMap<String, (Arc<Session>, Instant)>>>;
pub fn http(listener: TcpListener, client: Client, token: String) {
    let sessions: Sessions = Arc::new(Mutex::new(HashMap::new()));
    let workers = Arc::new(AtomicUsize::new(0));
    let Ok(address) = listener.local_addr() else {
        return;
    };
    let port = address.port();
    let mut hosts = vec![address.to_string(), format!("localhost:{port}")];
    if port == 80 {
        hosts.push(address.ip().to_string());
        hosts.push("localhost".into());
    }
    let origins = hosts.iter().map(|host| format!("http://{host}")).collect();
    let policy = Arc::new(Policy {
        token,
        hosts,
        origins,
    });
    if listener.set_nonblocking(true).is_err() {
        return;
    }
    while !client.stop.load(Ordering::Acquire) {
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(25));
                continue;
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        };
        // An accepted socket inherits the non-blocking mode of the listener on some systems.
        if stream.set_nonblocking(false).is_err()
            || stream.set_read_timeout(Some(IO_TIMEOUT)).is_err()
            || stream.set_write_timeout(Some(IO_TIMEOUT)).is_err()
        {
            continue;
        }
        if workers.fetch_add(1, Ordering::AcqRel) >= MAX_WORKERS {
            workers.fetch_sub(1, Ordering::Release);
            respond(&mut stream, 503, String::new());
            continue;
        }
        let client = client.clone();
        let sessions = sessions.clone();
        let workers = workers.clone();
        let policy = policy.clone();
        std::thread::spawn(move || {
            handle_connection(stream, &client, &sessions, &policy);
            workers.fetch_sub(1, Ordering::Release);
        });
    }
}
fn handle_connection(mut stream: TcpStream, client: &Client, sessions: &Sessions, policy: &Policy) {
    let request = match read_request(&mut stream) {
        Ok(request) => request,
        Err(status) => {
            if let Some(status) = status {
                respond(&mut stream, status, String::new());
            }
            return;
        }
    };
    if request.target != "/mcp" {
        respond(&mut stream, 404, String::new());
        return;
    }
    if !request
        .header("Host")
        .is_some_and(|h| policy.hosts.iter().any(|a| a == h))
        || request
            .header("Origin")
            .is_some_and(|o| !policy.origins.iter().any(|a| a == o))
    {
        respond(&mut stream, 403, String::new());
        return;
    }
    let authorized = request
        .header("Authorization")
        .and_then(|value| value.split_once(' '))
        .is_some_and(|(scheme, token)| {
            scheme.eq_ignore_ascii_case("Bearer") && same_secret(token.trim(), &policy.token)
        });
    if !authorized {
        respond(&mut stream, 401, String::new());
        return;
    }
    if request
        .header("MCP-Protocol-Version")
        .is_some_and(|v| !protocol::VERSIONS.contains(&v))
    {
        respond(&mut stream, 400, String::new());
        return;
    }
    if !matches!(request.method.as_str(), "POST" | "GET" | "DELETE") {
        respond(&mut stream, 405, String::new());
        return;
    }
    handle_http(stream, request, client, sessions);
}
fn handle_http(mut stream: TcpStream, request: Request, client: &Client, sessions: &Sessions) {
    let session_id = request.header("Mcp-Session-Id").map(str::to_owned);
    let session = {
        let mut sessions = sessions.lock().unwrap();
        sessions.retain(|_, (session, used)| {
            let keep = used.elapsed() < SESSION_IDLE;
            if !keep {
                session.close();
            }
            keep
        });
        session_id
            .as_ref()
            .and_then(|id| sessions.get_mut(id))
            .map(|(session, used)| {
                *used = Instant::now();
                session.clone()
            })
    };
    if session_id.is_some() && session.is_none() {
        respond(&mut stream, 404, String::new());
        return;
    }
    if request.method == "DELETE" {
        if let Some(id) = session_id {
            if let Some((session, _)) = sessions.lock().unwrap().remove(&id) {
                session.close();
            }
            respond(&mut stream, 200, String::new());
        } else {
            respond(&mut stream, 400, String::new());
        }
        return;
    }
    if request.method == "GET" {
        let Some(session) = session else {
            respond(&mut stream, 400, String::new());
            return;
        };
        if !request
            .header("Accept")
            .is_some_and(|a| a.contains("text/event-stream"))
        {
            respond(&mut stream, 406, String::new());
            return;
        }
        if session.stream_open.swap(true, Ordering::AcqRel) {
            respond(&mut stream, 409, String::new());
            return;
        }
        let _ = stream_logs(stream, client, &session);
        session.stream_open.store(false, Ordering::Release);
        return;
    }
    if !request
        .header("Content-Type")
        .and_then(|s| s.split(';').next())
        .is_some_and(|media| media.trim().eq_ignore_ascii_case("application/json"))
    {
        respond(&mut stream, 415, String::new());
        return;
    }
    if !request
        .header("Accept")
        .is_some_and(|s| s.contains("application/json") && s.contains("text/event-stream"))
    {
        respond(&mut stream, 406, String::new());
        return;
    }
    let message: Value = match serde_json::from_slice(&request.body) {
        Ok(message) => message,
        Err(_) => {
            respond(
                &mut stream,
                400,
                protocol::error(Value::Null, -32700, "Parse error").to_string(),
            );
            return;
        }
    };
    if message["method"] == "initialize" && session.is_none() {
        let mut bytes = [0u8; 24];
        if getrandom::fill(&mut bytes).is_err() {
            respond(&mut stream, 500, String::new());
            return;
        }
        let id = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let session = Arc::new(Session::default());
        let Some(response) = session.handle(client, message) else {
            respond(&mut stream, 400, String::new());
            return;
        };
        if response.get("error").is_some() {
            respond(&mut stream, 400, response.to_string());
            return;
        }
        {
            let mut sessions = sessions.lock().unwrap();
            if sessions.len() >= MAX_SESSIONS {
                respond(&mut stream, 503, String::new());
                return;
            }
            sessions.insert(id.clone(), (session, Instant::now()));
        }
        respond_with(
            &mut stream,
            200,
            &[("Mcp-Session-Id", &id)],
            &response.to_string(),
        );
    } else if let Some(session) = session {
        match session.handle(client, message) {
            Some(response) => respond(&mut stream, 200, response.to_string()),
            None => respond(&mut stream, 202, String::new()),
        }
    } else {
        respond(&mut stream, 400, String::new());
    }
}
// Every event is written as its own chunk and flushed, so that small notifications arrive at once.
fn stream_logs(mut stream: TcpStream, client: &Client, session: &Session) -> io::Result<()> {
    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\nTransfer-Encoding: chunked\r\n\r\n")?;
    stream.flush()?;
    let mut cursor = 0;
    while !client.stop.load(Ordering::Acquire) && !session.is_closed() {
        let messages = session.logs(client, &mut cursor);
        let text = if messages.is_empty() {
            ": keepalive\n\n".into()
        } else {
            messages
                .into_iter()
                .map(|m| format!("event: message\ndata: {m}\n\n"))
                .collect::<String>()
        };
        write!(stream, "{:x}\r\n{}\r\n", text.len(), text)?;
        stream.flush()?;
        std::thread::sleep(Duration::from_millis(250));
    }
    stream.write_all(b"0\r\n\r\n")?;
    stream.flush()
}
