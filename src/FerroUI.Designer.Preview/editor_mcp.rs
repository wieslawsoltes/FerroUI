//! The editor owns its buffers and UI. Existing MCP transport workers forward
//! validated requests to that window's authenticated private bridge.
use ferroui_mcp::{Options, Permissions, RequestContext, ServiceToolProvider, Transport};
use serde_json::{json, Value};
use std::{
    io::{self, BufRead, BufReader, Write},
    net::{SocketAddr, TcpStream},
    sync::Arc,
    time::{Duration, Instant},
};

const MAX_RESPONSE: usize = 16 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(35);

pub(crate) fn serve() -> io::Result<()> {
    let address = std::env::var("FERROUI_EDITOR_MCP_ADDRESS")
        .ok()
        .and_then(|address| address.parse::<SocketAddr>().ok())
        .filter(|address| address.ip().is_loopback() && address.port() != 0)
        .ok_or_else(|| io::Error::other("Expected a local VS Code bridge address"))?;
    let token = std::env::var("FERROUI_EDITOR_MCP_TOKEN")
        .ok()
        .filter(|token| token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| io::Error::other("Expected a VS Code bridge token"))?;
    let mut bridge = Bridge {
        address,
        token,
        tools: Vec::new(),
    };
    bridge.tools = serde_json::from_value(
        bridge
            .exchange(json!({"type":"catalog"}), None)
            .map_err(io::Error::other)?,
    )
    .map_err(io::Error::other)?;
    let server = ferroui_mcp::serve_service(
        Options {
            transport: Transport::Stdio,
            permissions: Permissions {
                allow_property_writes: true,
                allow_input: false,
            },
            capture_logs: false,
            dispatch_timeout: TIMEOUT,
            ..Options::default()
        },
        Arc::new(bridge),
    )?;
    while server.is_running() {
        std::thread::sleep(Duration::from_millis(30));
    }
    Ok(())
}

struct Bridge {
    address: SocketAddr,
    token: String,
    tools: Vec<Value>,
}

impl Bridge {
    fn exchange(
        &self,
        mut message: Value,
        context: Option<&RequestContext>,
    ) -> Result<Value, String> {
        let check = |start: Instant| {
            if let Some(context) = context {
                context.check_cancelled()
            } else if start.elapsed() >= Duration::from_secs(5) {
                Err("The VS Code bridge did not respond".into())
            } else {
                Ok(())
            }
        };
        let start = Instant::now();
        check(start)?;
        let mut stream = TcpStream::connect_timeout(&self.address, Duration::from_secs(2))
            .map_err(|_| "The FerroUI VS Code window is unavailable".to_owned())?;
        stream
            .set_read_timeout(Some(Duration::from_millis(150)))
            .map_err(|e| e.to_string())?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| e.to_string())?;
        message["token"] = json!(self.token);
        writeln!(stream, "{message}").map_err(|e| e.to_string())?;
        let mut reader = BufReader::new(stream);
        let mut bytes = Vec::new();
        loop {
            check(start)?;
            let chunk = match reader.fill_buf() {
                Ok([]) => return Err("The VS Code bridge closed the request".into()),
                Ok(chunk) => chunk,
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                    ) =>
                {
                    continue
                }
                Err(error) => return Err(error.to_string()),
            };
            let newline = chunk.iter().position(|byte| *byte == b'\n');
            let length = newline.unwrap_or(chunk.len());
            if bytes.len() + length > MAX_RESPONSE {
                return Err("The VS Code bridge response exceeded 16 MiB".into());
            }
            bytes.extend_from_slice(&chunk[..length]);
            if newline.is_some() {
                break;
            }
            reader.consume(length);
        }
        let response: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        match response["error"].as_str() {
            Some(error) => Err(error.to_owned()),
            None => Ok(response["result"].clone()),
        }
    }
}

impl ServiceToolProvider for Bridge {
    fn tools(&self) -> Vec<Value> {
        self.tools.clone()
    }

    fn validate(&self, _: &str, _: &Value) -> Result<(), String> {
        Ok(())
    }

    fn call(
        &self,
        name: &str,
        arguments: &Value,
        context: &RequestContext,
    ) -> Result<Value, String> {
        self.exchange(
            json!({"type":"call", "name":name, "arguments":arguments}),
            Some(context),
        )
    }
}
