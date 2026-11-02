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
    input: mpsc::SyncSender<Json>,
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
        // The protocol alternates one host frame with one worker frame. Keep
        // both pipe operations off the owner thread, so a blocked write and
        // a stalled guest share the same cancellable response wait.
        let (input, requests) = mpsc::sync_channel::<Json>(1);
        let (sender, output) = mpsc::sync_channel(1);
        let io = thread::Builder::new()
            .name("qml-webscene-protocol".into())
            .spawn(move || {
                // A realm announces readiness before its first command.
                // A WorkerScript starts by waiting for its initialization.
                if !worker_script {
                    let hello = framing::receive(&mut stdout).map_err(|e| e.to_string());
                    let failed = hello.is_err();
                    if sender.send(hello).is_err() || failed {
                        return;
                    }
                }
                while let Ok(message) = requests.recv() {
                    let reply = framing::send(&mut stream, &message)
                        .map_err(|e| format!("write: {e}"))
                        .and_then(|()| framing::receive(&mut stdout).map_err(|e| e.to_string()));
                    let failed = reply.is_err();
                    if sender.send(reply).is_err() || failed {
                        break;
                    }
                }
            });
        if let Err(error) = io {
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
    fn enqueue(&self, msg: Json, deadline: Instant) -> Result<(), Error> {
        if Instant::now() >= deadline {
            return Err(protocol_timeout(mpsc::RecvTimeoutError::Timeout));
        }
        // A previous exchange has already received its frame before another
        // can start, including when a host handler reenters the realm.
        self.input.try_send(msg).map_err(|e| match e {
            mpsc::TrySendError::Disconnected(_) => Error::new("WebScene worker disconnected"),
            mpsc::TrySendError::Full(_) => Error::new("WebScene request already pending"),
        })
    }
    pub fn exchange(&self, msg: Json, deadline: Instant) -> Result<Json, Error> {
        self.enqueue(msg, deadline)?;
        self.read(deadline)
    }
    pub fn exchange_cancellable(
        &self,
        msg: Json,
        deadline: Instant,
        cancelled: &std::sync::atomic::AtomicBool,
    ) -> Result<Json, Error> {
        if cancelled.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::new("WorkerScript stopped"));
        }
        self.enqueue(msg, deadline)?;
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

#[cfg(all(test, unix))]
#[test]
fn workerscript_cancels_while_a_guest_does_not_read_its_input() {
    use std::io::Write;
    use std::sync::atomic::{AtomicBool, Ordering};

    let mut script = tempfile::NamedTempFile::new().unwrap();
    writeln!(script, "exec sleep 30").unwrap();
    let child = Worker::spawn_mode(Path::new("/bin/sh"), script.path(), true).unwrap();
    let cancelled = AtomicBool::new(false);
    thread::scope(|scope| {
        scope.spawn(|| {
            thread::sleep(Duration::from_millis(100));
            cancelled.store(true, Ordering::Release);
        });
        let message = Json::String("x".repeat(2 * 1024 * 1024));
        let deadline = Instant::now() + Duration::from_secs(10);
        let error = child
            .exchange_cancellable(message, deadline, &cancelled)
            .unwrap_err();
        assert_eq!(error.message, "WorkerScript stopped");
    });
}
