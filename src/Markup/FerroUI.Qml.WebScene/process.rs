//! The application thread remains the sole owner of the QML transaction. Only
//! framed bytes and completion notifications cross the worker process boundary.
use super::*;
use std::{
    io::{BufReader, BufWriter},
    process::{Child, Command, Stdio},
    sync::mpsc,
    thread,
    time::Instant,
};

pub(super) struct Worker {
    child: Child,
    input: mpsc::Sender<(Json, mpsc::Sender<std::io::Result<()>>)>,
    output: mpsc::Receiver<Result<Json, String>>,
}
impl Worker {
    pub fn spawn(executable: &Path, library: &Path) -> Result<Self, Error> {
        Self::spawn_mode(executable, library, false)
    }
    pub fn spawn_mode(
        executable: &Path,
        library: &Path,
        worker_script: bool,
    ) -> Result<Self, Error> {
        let mut command = Command::new(executable);
        command
            .arg(library)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        if worker_script {
            command.arg("--workerscript");
        }
        #[cfg(windows)]
        if let Some(root) = std::env::var_os("SystemRoot") {
            command.env("SystemRoot", root);
        }
        let mut child = command
            .spawn()
            .map_err(|e| Error::new(format!("launch WebScene worker: {e}")))?;
        let mut stream = BufWriter::new(child.stdin.take().expect("piped stdin"));
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
        // A guest can stop consuming input. Keep pipe writes off the UI
        // thread so large frames obey the same deadline as result reads.
        let (input, writes) = mpsc::channel::<(Json, mpsc::Sender<std::io::Result<()>>)>();
        let writer = thread::Builder::new()
            .name("qml-webscene-input".into())
            .spawn(move || {
                while let Ok((message, acknowledgement)) = writes.recv() {
                    let status = framing::send(&mut stream, &message);
                    let failed = status.is_err();
                    let _ = acknowledgement.send(status);
                    if failed {
                        break;
                    }
                }
            });
        if let Err(error) = writer {
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
    pub fn send(&self, msg: Json, deadline: Instant) -> Result<(), Error> {
        let (ack, written) = mpsc::channel();
        self.input
            .send((msg, ack))
            .map_err(|_| Error::new("WebScene worker disconnected"))?;
        written
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .map_err(protocol_timeout)?
            .map_err(|e| Error::new(format!("WebScene worker write: {e}")))
    }
    pub fn read_cancellable(
        &self,
        deadline: Instant,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Json, Error> {
        loop {
            if cancelled.load(std::sync::atomic::Ordering::Acquire) {
                return Err(Error::new("WorkerScript stopped"));
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self
                .output
                .recv_timeout(remaining.min(Duration::from_millis(25)))
            {
                Ok(message) => {
                    return message
                        .map_err(|e| Error::new(format!("WorkerScript disconnected: {e}")))
                }
                Err(mpsc::RecvTimeoutError::Timeout) if Instant::now() < deadline => continue,
                Err(e) => return Err(protocol_timeout(e)),
            }
        }
    }
    pub fn read(&self, deadline: Instant) -> Result<Json, Error> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        self.output
            .recv_timeout(remaining)
            .map_err(protocol_timeout)?
            .map_err(|e| Error::new(format!("WebScene worker stopped: {e}")))
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn protocol_timeout(error: mpsc::RecvTimeoutError) -> Error {
    match error {
        mpsc::RecvTimeoutError::Timeout => Error::new("WebScene execution deadline exceeded"),
        _ => Error::new("WebScene worker disconnected"),
    }
}
