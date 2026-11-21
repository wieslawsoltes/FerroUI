use crate::{
    LanguageError, Result,
    lsp::{read_frame, write_frame},
};
use ferroui_designer_project::{spawn_project_process, stop_project_process};
use serde_json::{Value, json};
use std::{
    io::{self, BufReader},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::{Duration, Instant},
};

/// Pumps bounded LSP frames off the requesting thread. Cancellation can close
/// the process tree even if a child stopped reading from its input pipe.
pub(super) struct Session<'a> {
    child: Child,
    outgoing: Sender<Value>,
    incoming: Receiver<io::Result<Value>>,
    cancelled: &'a AtomicBool,
    deadline: Instant,
    serial: u64,
    status: Value,
}

impl<'a> Session<'a> {
    pub(super) fn spawn(
        mut command: Command,
        cancelled: &'a AtomicBool,
        timeout: Duration,
    ) -> Result<Self> {
        if cancelled.load(Ordering::Acquire) {
            return Err(LanguageError("Rust rename review was cancelled".into()));
        }
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        let mut child = spawn_project_process(&mut command).map_err(|error| {
            LanguageError(format!(
                "Start rust-analyzer from PATH or set FERROUI_RUST_ANALYZER. {error}"
            ))
        })?;
        let mut input = child.stdin.take().expect("piped stdin");
        let mut output = BufReader::new(child.stdout.take().expect("piped stdout"));
        // The caller has a bounded draft snapshot. Queue its notifications
        // without waiting for the server to drain its own startup messages.
        let (send, outgoing) = mpsc::channel::<Value>();
        let (incoming, receive) = mpsc::sync_channel(16);
        let session = Self {
            child,
            outgoing: send,
            incoming: receive,
            cancelled,
            deadline: Instant::now() + timeout,
            serial: 0,
            status: Value::Null,
        };
        let writer_events = incoming.clone();
        thread::Builder::new()
            .spawn(move || {
                while let Ok(message) = outgoing.recv() {
                    if let Err(error) = write_frame(&mut input, &message) {
                        let _ = writer_events.send(Err(error));
                        break;
                    }
                }
            })
            .map_err(|error| LanguageError(format!("Open rust-analyzer input. {error}")))?;
        thread::Builder::new()
            .spawn(move || {
                loop {
                    match read_frame(&mut output) {
                        Ok(Some(message)) => {
                            if incoming.send(Ok(message)).is_err() {
                                break;
                            }
                        }
                        Ok(None) => {
                            let _ = incoming.send(Err(io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "rust-analyzer closed its output",
                            )));
                            break;
                        }
                        Err(error) => {
                            let _ = incoming.send(Err(error));
                            break;
                        }
                    }
                }
            })
            .map_err(|error| LanguageError(format!("Open rust-analyzer output. {error}")))?;
        Ok(session)
    }

    pub(super) fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.serial += 1;
        let id = self.serial;
        self.send(json!({"jsonrpc":"2.0", "id": id, "method":method, "params":params}))?;
        loop {
            let message = self.read()?;
            if message.get("method").is_none() && message["id"].as_u64() == Some(id) {
                if let Some(error) = message.pointer("/error/message").and_then(Value::as_str) {
                    return Err(LanguageError(error.to_owned()));
                }
                return Ok(message.get("result").cloned().unwrap_or(Value::Null));
            }
        }
    }

    pub(super) fn notify(&self, method: &str, params: Value) -> Result<()> {
        self.send(json!({"jsonrpc":"2.0", "method":method, "params":params}))
    }

    pub(super) fn wait_until_ready(&mut self) -> Result<()> {
        loop {
            if self.status["quiescent"] == true {
                return if self.status["health"] == "error" {
                    Err(LanguageError(format!(
                        "rust-analyzer could not load the Cargo project. {}",
                        self.status["message"]
                            .as_str()
                            .unwrap_or("Check its Cargo configuration.")
                    )))
                } else {
                    Ok(())
                };
            }
            self.read()?;
        }
    }

    pub(super) fn finish(&mut self) {
        self.deadline = self
            .deadline
            .min(Instant::now() + Duration::from_millis(500));
        if self.request("shutdown", Value::Null).is_ok() && self.notify("exit", Value::Null).is_ok()
        {
            while Instant::now() < self.deadline && matches!(self.child.try_wait(), Ok(None)) {
                thread::sleep(Duration::from_millis(10));
            }
        }
    }

    fn check(&self) -> Result<()> {
        if self.cancelled.load(Ordering::Acquire) {
            Err(LanguageError("Rust rename review was cancelled".into()))
        } else if Instant::now() >= self.deadline {
            Err(LanguageError("rust-analyzer did not finish loading and resolving the selected Cargo project in time".into()))
        } else {
            Ok(())
        }
    }

    fn send(&self, value: Value) -> Result<()> {
        self.check()?;
        self.outgoing.send(value).map_err(|_| disconnected())
    }

    fn read(&mut self) -> Result<Value> {
        loop {
            self.check()?;
            let message = match self.incoming.recv_timeout(Duration::from_millis(50)) {
                Ok(Ok(message)) => message,
                Ok(Err(error))
                    if error.kind() != io::ErrorKind::UnexpectedEof
                        && error.kind() != io::ErrorKind::BrokenPipe =>
                {
                    return Err(LanguageError(format!(
                        "Read rust-analyzer response. {error}"
                    )));
                }
                Ok(Err(_)) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(disconnected());
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
            };
            if message["method"] == "experimental/serverStatus" {
                self.status = message["params"].clone();
            }
            if let (Some(method), Some(id)) = (message["method"].as_str(), message.get("id")) {
                let response = match method {
                    "window/workDoneProgress/create"
                    | "client/registerCapability"
                    | "client/unregisterCapability" => json!({"result":null}),
                    "workspace/applyEdit" => json!({"result":{"applied":false}}),
                    _ => {
                        json!({"error":{"code":-32601,"message":"Unsupported rust-analyzer request"}})
                    }
                };
                let mut response = response;
                response["jsonrpc"] = "2.0".into();
                response["id"] = id.clone();
                self.send(response)?;
            }
            return Ok(message);
        }
    }
}

impl Drop for Session<'_> {
    fn drop(&mut self) {
        let exited = self.child.try_wait().is_ok_and(|result| result.is_some());
        let _ = stop_project_process(&mut self.child, exited);
    }
}

fn disconnected() -> LanguageError {
    LanguageError("rust-analyzer closed before returning a review. Check that the executable is available for this Rust toolchain".into())
}
