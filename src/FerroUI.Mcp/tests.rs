use super::*;
use crate::protocol::Session;
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpStream,
};

fn options() -> Options {
    Options {
        capture_logs: false,
        ..Options::default()
    }
}
fn request(address: SocketAddr, method: &str, headers: &str, body: &str) -> (u16, String, String) {
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    write!(stream,"{method} /mcp HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n{body}",body.len()).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    let (head, body) = response.split_once("\r\n\r\n").unwrap();
    (
        head.split_whitespace().nth(1).unwrap().parse().unwrap(),
        head.into(),
        body.into(),
    )
}
fn headers(token: &str, session: Option<&str>) -> String {
    format!("Authorization: Bearer {token}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n{}",session.map(|s|format!("Mcp-Session-Id: {s}\r\nMCP-Protocol-Version: {}\r\n",protocol::VERSION)).unwrap_or_default())
}
#[test]
fn http_authentication_origin_sessions_and_protocol() {
    let _scope = Dispatcher::unit_test_scope();
    let server = serve(options()).unwrap();
    let address = server.address.unwrap();
    let token = server.token.as_ref().unwrap();
    assert_eq!(request(address, "POST", "", "{}").0, 401);
    assert_eq!(
        request(
            address,
            "POST",
            &format!(
                "{}Origin: https://attacker.example\r\n",
                headers(token, None)
            ),
            "{}"
        )
        .0,
        403
    );
    let init=json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":protocol::VERSION,"capabilities":{},"clientInfo":{"name":"test","version":"1"}}}).to_string();
    let (status, head, body) = request(address, "POST", &headers(token, None), &init);
    assert_eq!(status, 200);
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["result"]["protocolVersion"],
        protocol::VERSION
    );
    let session = head
        .lines()
        .find_map(|l| {
            let (k, v) = l.split_once(':')?;
            k.eq_ignore_ascii_case("Mcp-Session-Id").then(|| v.trim())
        })
        .unwrap();
    let auth = headers(token, Some(session));
    assert_eq!(
        request(
            address,
            "POST",
            &auth,
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#
        )
        .0,
        202
    );
    let (_, _, body) = request(
        address,
        "POST",
        &auth,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
    );
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["result"]["tools"]
            .as_array()
            .unwrap()
            .len(),
        16
    );
    assert_eq!(request(address, "POST", &auth, "{").0, 400);
    assert_eq!(
        request(
            address,
            "POST",
            &headers(token, Some("missing")),
            r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#
        )
        .0,
        404
    );
    // A real SSE client must receive small messages immediately, not after a chunk buffer fills.
    let capture = server.client.logs.install();
    request(
        address,
        "POST",
        &auth,
        r#"{"jsonrpc":"2.0","id":6,"method":"logging/setLevel","params":{"level":"warning"}}"#,
    );
    ferroui_base::logging::Logger::sink().unwrap().log(
        ferroui_base::logging::LogEventLevel::Warning,
        "MCP-test",
        None,
        "stream-test",
    );
    let mut stream = TcpStream::connect(address).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    write!(stream,"GET /mcp HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {token}\r\nMcp-Session-Id: {session}\r\nAccept: text/event-stream\r\n\r\n").unwrap();
    let mut reader = std::io::BufReader::new(stream);
    loop {
        let mut line = String::new();
        assert!(std::io::BufRead::read_line(&mut reader, &mut line).unwrap() > 0);
        if line.starts_with("data:") && line.contains("stream-test") {
            break;
        }
    }
    drop(reader);
    drop(capture);
    assert_eq!(request(address, "DELETE", &auth, "").0, 200);
    assert_eq!(
        request(
            address,
            "POST",
            &auth,
            r#"{"jsonrpc":"2.0","id":4,"method":"ping"}"#
        )
        .0,
        404
    );
}
#[test]
fn oversized_and_stalled_bodies_neither_allocate_nor_block_other_requests() {
    let _scope = Dispatcher::unit_test_scope();
    let server = serve(options()).unwrap();
    let address = server.address.unwrap();
    let token = server.token.as_ref().unwrap();
    // A length beyond the message limit is refused from the head alone, before authentication.
    let mut huge = TcpStream::connect(address).unwrap();
    huge.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    write!(
        huge,
        "POST /mcp HTTP/1.1\r\nHost: {address}\r\nContent-Length: 100000000000000\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    huge.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 413"), "{response}");
    // A body that never arrives holds only its own connection.
    let mut stalled = TcpStream::connect(address).unwrap();
    write!(
        stalled,
        "POST /mcp HTTP/1.1\r\nHost: {address}\r\nContent-Length: 2000\r\n\r\n"
    )
    .unwrap();
    let started = Instant::now();
    assert_eq!(request(address, "POST", "", "{}").0, 401);
    let init=json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":protocol::VERSION,"capabilities":{},"clientInfo":{"name":"test","version":"1"}}}).to_string();
    assert_eq!(
        request(address, "POST", &headers(token, None), &init).0,
        200
    );
    assert!(started.elapsed() < Duration::from_secs(5));
    drop(stalled);
}
#[test]
fn rejects_non_loopback_and_invalid_configuration() {
    let _scope = Dispatcher::unit_test_scope();
    assert!(serve(Options {
        transport: Transport::Http(([0, 0, 0, 0], 0).into()),
        ..options()
    })
    .is_err());
    assert!(serve(Options {
        dispatch_timeout: Duration::ZERO,
        ..options()
    })
    .is_err());
}
#[test]
fn dispatcher_executes_and_timeout_does_not_leave_queued_work() {
    let _scope = Dispatcher::unit_test_scope();
    let server = serve(Options {
        dispatch_timeout: Duration::from_millis(100),
        ..options()
    })
    .unwrap();
    let client = server.client.clone();
    let worker = std::thread::spawn(move || {
        client.call("get_tree", json!({}), &Arc::new(AtomicBool::new(false)))
    });
    while !worker.is_finished() {
        Dispatcher::ui_thread().run_jobs(None);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(worker.join().unwrap().unwrap(), json!([]));
    let client = server.client.clone();
    let worker = std::thread::spawn(move || {
        client.call("get_tree", json!({}), &Arc::new(AtomicBool::new(false)))
    });
    // Deliberately do not pump this dispatcher.
    assert!(worker.join().unwrap().unwrap_err().contains("timed out"));
    Dispatcher::ui_thread().run_jobs(None);
}
#[test]
fn waits_yield_to_dispatcher_and_cancel() {
    let _scope = Dispatcher::unit_test_scope();
    let server = serve(options()).unwrap();
    let client = server.client.clone();
    let flag = Arc::new(AtomicBool::new(false));
    let cancelled = flag.clone();
    let worker = std::thread::spawn(move || {
        client.call(
            "wait_for",
            json!({"name":"missing","timeout_ms":5000}),
            &cancelled,
        )
    });
    for _ in 0..20 {
        Dispatcher::ui_thread().run_jobs(None);
        std::thread::sleep(Duration::from_millis(5));
    }
    flag.store(true, Ordering::Release);
    while !worker.is_finished() {
        Dispatcher::ui_thread().run_jobs(None);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(worker.join().unwrap().unwrap_err().contains("cancelled"));
    let client = server.client.clone();
    let worker = std::thread::spawn(move || {
        client.call(
            "wait_for",
            json!({"name":"missing","timeout_ms":0}),
            &Arc::new(AtomicBool::new(false)),
        )
    });
    while !worker.is_finished() {
        Dispatcher::ui_thread().run_jobs(None);
        std::thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(worker.join().unwrap().unwrap_err(), "Condition timed out");
}
#[test]
fn protocol_validation_resources_and_notifications() {
    let _scope = Dispatcher::unit_test_scope();
    let server = serve(options()).unwrap();
    let session = Session::default();
    assert_eq!(
        session.handle(&server.client, json!([])).unwrap()["error"]["code"],
        -32600
    );
    assert!(session
        .handle(
            &server.client,
            json!({"jsonrpc":"2.0","method":"tools/call","params":{"name":"set_property"}})
        )
        .is_none());
    assert_eq!(
        session
            .handle(
                &server.client,
                json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})
            )
            .unwrap()["error"]["code"],
        -32000
    );
    session.handle(&server.client,json!({"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"unknown"}}));
    session.handle(
        &server.client,
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    );
    assert_eq!(
        session
            .handle(
                &server.client,
                json!({"jsonrpc":"2.0","id":3,"method":"resources/list"})
            )
            .unwrap()["result"]["resources"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(session.handle(&server.client,json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"screenshot","arguments":{"scale":999}}})).unwrap()["error"]["code"],-32602);
    assert_eq!(
        session
            .handle(
                &server.client,
                json!({"jsonrpc":"2.0","id":5,"method":"unknown"})
            )
            .unwrap()["error"]["code"],
        -32601
    );
}
