//! The application thread remains the sole owner of the QML transaction. Only
//! framed bytes and completion notifications cross the worker process boundary.
use super::*;
use std::{
    io::{BufReader, BufWriter},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    thread,
    time::Instant,
};

pub(super) struct Worker {
    child: Child,
    input: BufWriter<ChildStdin>,
    output: mpsc::Receiver<Result<Json, String>>,
}
impl Worker {
    pub fn spawn(executable: &Path, library: &Path) -> Result<Self, Error> {
        let mut command = Command::new(executable);
        command
            .arg(library)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        #[cfg(windows)]
        if let Some(root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", root);
        }
        let mut child = command
            .spawn()
            .map_err(|e| Error::new(format!("launch WebScene worker: {e}")))?;
        let input = BufWriter::new(child.stdin.take().expect("piped stdin"));
        let mut stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
        let (sender, output) = mpsc::sync_channel(1);
        let reader = thread::Builder::new()
            .name("qml-webscene-protocol".into())
            .spawn(move || loop {
                match framing::receive(&mut stdout) {
                    Ok(json) => {
                        if sender.send(Ok(json)).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(Err(e.to_string()));
                        break;
                    }
                }
            });
        if let Err(error) = reader {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::new(error.to_string()));
        }
        Ok(Self {
            child,
            input,
            output,
        })
    }
    pub fn send(&mut self, msg: &Json) -> Result<(), Error> {
        framing::send(&mut self.input, msg)
            .map_err(|e| Error::new(format!("WebScene worker write: {e}")))
    }
    pub fn read(&self, deadline: Instant) -> Result<Json, Error> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        self.output
            .recv_timeout(remaining)
            .map_err(|e| match e {
                mpsc::RecvTimeoutError::Timeout => {
                    Error::new("WebScene execution deadline exceeded")
                }
                _ => Error::new("WebScene worker disconnected"),
            })?
            .map_err(|e| Error::new(format!("WebScene worker stopped: {e}")))
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
