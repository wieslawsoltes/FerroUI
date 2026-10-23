//! Opt-in Model Context Protocol server for FerroUI. Call [`serve`] on the UI thread after
//! application setup and keep the returned handle alive until shutdown. No server starts merely
//! by linking the crate. Property writes and input are independently disabled by default.
//!
//! The tools run on the shared inspector of the UI thread (`ferroui_inspect::Inspector::shared`),
//! the one the Chrome DevTools Protocol endpoint uses too: node IDs are the same for both, and a
//! custom lifetime registers its top levels there (`register_root`).
mod protocol;
mod transport;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_inspect::LogCapture;
pub use ferroui_inspect::Permissions;
use serde_json::Value;
use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};

/// HTTP listens only on a loopback address. Port zero chooses an unused port.
#[derive(Clone, Debug)]
pub enum Transport {
    Stdio,
    Http(SocketAddr),
}
pub struct Options {
    pub transport: Transport,
    pub permissions: Permissions,
    /// Tee the application's diagnostic log into the bounded ring of `ferroui-inspect`, shared
    /// with the other servers of the process, while the server is alive.
    pub capture_logs: bool,
    /// Previously reserved protocol output; otherwise stdio reserves stdout when serve is called.
    pub stdio_output: Option<std::fs::File>,
    /// Maximum time a queued UI operation may wait for the dispatcher.
    pub dispatch_timeout: Duration,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            transport: Transport::Http(SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)),
            permissions: Permissions::default(),
            capture_logs: true,
            stdio_output: None,
            dispatch_timeout: Duration::from_secs(10),
        }
    }
}
static STDIO_STARTED: AtomicBool = AtomicBool::new(false);

/// Dropping this handle stops accepting requests and invalidates queued work. A stdio reader
/// blocked in the OS remains parked until EOF; it cannot execute more UI operations.
pub struct ServerHandle {
    client: Client,
    log_guard: Option<ferroui_inspect::LogGuard>,
    pub address: Option<SocketAddr>,
    /// Bearer token for HTTP; printed to stderr at startup, never included in tool responses.
    pub token: Option<String>,
}
impl ServerHandle {
    pub fn is_running(&self) -> bool {
        !self.client.stop.load(Ordering::Acquire)
    }
}
impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.client.stop.store(true, Ordering::Release);
        self.log_guard.take();
    }
}

/// Start the server on the UI thread. The returned handle is the lifetime of the service.
pub fn serve(options: Options) -> io::Result<ServerHandle> {
    let dispatcher = Dispatcher::ui_thread();
    dispatcher.verify_access();
    if options.dispatch_timeout.is_zero() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "dispatch_timeout must be positive",
        ));
    }
    let stdio = matches!(options.transport, Transport::Stdio);
    if stdio && STDIO_STARTED.swap(true, Ordering::AcqRel) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Only one stdio server may start per process",
        ));
    }
    // A server that fails to start leaves stdio free for another attempt.
    let result = start(options, dispatcher);
    if stdio && result.is_err() {
        STDIO_STARTED.store(false, Ordering::Release);
    }
    result
}

fn start(options: Options, dispatcher: Arc<Dispatcher>) -> io::Result<ServerHandle> {
    let (http, address, token) = match &options.transport {
        Transport::Http(address) => {
            if !address.ip().is_loopback() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "MCP HTTP must bind to loopback",
                ));
            }
            let server = std::net::TcpListener::bind(address)?;
            let address = server.local_addr()?;
            let mut bytes = [0u8; 32];
            getrandom::fill(&mut bytes).map_err(|e| io::Error::other(e.to_string()))?;
            let token = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
            (Some(server), Some(address), Some(token))
        }
        Transport::Stdio => (None, None, None),
    };
    let stdio_output = if http.is_none() {
        Some(match options.stdio_output {
            Some(file) => file,
            None => reserve_stdout()?,
        })
    } else {
        None
    };
    let log_guard = options.capture_logs.then(LogCapture::install);
    let client = Client {
        dispatcher,
        permissions: options.permissions,
        timeout: options.dispatch_timeout,
        stop: Arc::new(AtomicBool::new(false)),
    };
    let handle = ServerHandle {
        client: client.clone(),
        log_guard,
        address,
        token: token.clone(),
    };
    let task = move || {
        let stop = client.stop.clone();
        if let Some(server) = http {
            transport::http(server, client, token.unwrap());
        } else if let Err(e) = transport::stdio(client, stdio_output.unwrap()) {
            eprintln!("FerroUI MCP stdio: {e}");
        }
        stop.store(true, Ordering::Release);
    };
    std::thread::Builder::new()
        .name("FerroUI MCP".into())
        .spawn(task)?;
    if let (Some(address), Some(token)) = (handle.address, &handle.token) {
        eprintln!("FerroUI MCP: http://{address}/mcp\nAuthorization: Bearer {token}");
    }
    Ok(handle)
}

#[derive(Clone)]
pub(crate) struct Client {
    dispatcher: Arc<Dispatcher>,
    permissions: Permissions,
    timeout: Duration,
    stop: Arc<AtomicBool>,
}
impl Client {
    fn call(&self, name: &str, args: Value, cancel: &Arc<AtomicBool>) -> Result<Value, String> {
        ferroui_inspect::validate(name, &args)?;
        let deadline =
            Instant::now() + Duration::from_millis(args["timeout_ms"].as_u64().unwrap_or(5000));
        loop {
            if self.stop.load(Ordering::Acquire) || cancel.load(Ordering::Acquire) {
                return Err("Request cancelled or server stopped".into());
            }
            let (tx, rx) = mpsc::sync_channel(1);
            let permissions = self.permissions;
            let tool = name.to_owned();
            let args = args.clone();
            let cancelled = cancel.clone();
            let stop = self.stop.clone();
            let pending = Arc::new(AtomicBool::new(true));
            let queued = pending.clone();
            self.dispatcher.post(
                move || {
                    if !queued.swap(false, Ordering::AcqRel)
                        || cancelled.load(Ordering::Acquire)
                        || stop.load(Ordering::Acquire)
                    {
                        return;
                    }
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        ferroui_inspect::execute(&tool, &args, permissions)
                    }))
                    .unwrap_or_else(|_| Err("UI operation panicked".into()));
                    let _ = tx.send(result);
                },
                DispatcherPriority::BACKGROUND,
            );
            let queued_at = Instant::now();
            let result = loop {
                if self.stop.load(Ordering::Acquire) || cancel.load(Ordering::Acquire) {
                    pending.store(false, Ordering::Release);
                    return Err("Request cancelled or server stopped".into());
                }
                let remaining = self.timeout.saturating_sub(queued_at.elapsed());
                if remaining.is_zero() {
                    pending.store(false, Ordering::Release);
                    return Err("UI dispatcher timed out".into());
                }
                match rx.recv_timeout(remaining.min(Duration::from_millis(25))) {
                    Ok(result) => break result?,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        return Err("UI operation cancelled".into())
                    }
                }
            };
            if name != "wait_for" || result["matched"] == true {
                return Ok(result);
            }
            if Instant::now() >= deadline {
                return Err("Condition timed out".into());
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}

/// Reserve stdout for MCP before application startup. Ordinary stdout is redirected to stderr
/// for the rest of the process, including native/library output on Unix (on Windows, output
/// through the process's standard handle). Pass the returned file in Options.
pub fn reserve_stdout() -> io::Result<std::fs::File> {
    use std::io::Write;
    std::io::stdout().flush()?;
    let original = filedescriptor::FileDescriptor::redirect_stdio(
        &std::io::stderr(),
        filedescriptor::StdioDescriptor::Stdout,
    )
    .map_err(io::Error::other)?;
    // The original stdout changes owner instead of being duplicated: dropping it would close the
    // handle that the C runtime of Windows still holds as its descriptor 1.
    #[cfg(unix)]
    {
        use std::os::fd::{FromRawFd, IntoRawFd};
        // SAFETY: `into_raw_fd` hands over the descriptor, which nothing else owns.
        Ok(unsafe { std::fs::File::from_raw_fd(original.into_raw_fd()) })
    }
    #[cfg(windows)]
    {
        use std::os::windows::io::{FromRawHandle, IntoRawHandle};
        // SAFETY: `into_raw_handle` hands over the handle, which nothing else owns.
        Ok(unsafe { std::fs::File::from_raw_handle(original.into_raw_handle()) })
    }
}

#[cfg(test)]
mod tests;
