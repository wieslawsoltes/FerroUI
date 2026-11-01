//! Supervise a single isolated JS process from a dedicated Rust thread.
//! Enqueuing from QML is bounded and never waits for JavaScript to return.
use super::*;
use ferroui_qml::workers::{WorkerDelivery, WorkerEndpoint, WorkerEvent, WorkerValue};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc, Arc,
    },
    thread,
};

const INPUT_BYTES: usize = 32 * 1024 * 1024;
struct Actor {
    input: mpsc::SyncSender<(WorkerValue, usize)>,
    bytes: Arc<AtomicUsize>,
    cancelled: Arc<AtomicBool>,
}
impl WorkerEndpoint for Actor {
    fn send(&self, value: WorkerValue) -> Result<(), Error> {
        let bytes = value.estimated_bytes();
        if bytes > INPUT_BYTES {
            return Err(Error::new("WorkerScript input byte limit exceeded"));
        }
        if self.bytes.fetch_add(bytes, Ordering::AcqRel) > INPUT_BYTES - bytes {
            self.bytes.fetch_sub(bytes, Ordering::Release);
            return Err(Error::new("WorkerScript input byte limit exceeded"));
        }
        self.input.try_send((value, bytes)).map_err(|e| {
            self.bytes.fetch_sub(bytes, Ordering::Release);
            Error::new(match e {
                mpsc::TrySendError::Full(_) => "WorkerScript input queue is full",
                mpsc::TrySendError::Disconnected(_) => "WorkerScript is no longer running",
            })
        })
    }
}
impl Drop for Actor {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

struct Supervisor {
    worker: PathBuf,
    library: PathBuf,
    timeout: Duration,
    cancelled: Arc<AtomicBool>,
    delivery: WorkerDelivery,
}
pub(super) fn start(
    engine: &WebSceneEngine,
    resources: Arc<BTreeMap<String, ir::ScriptResource>>,
    origin: &str,
    source: &str,
    delivery: WorkerDelivery,
) -> Result<Box<dyn WorkerEndpoint>, Error> {
    let url = worker_program::entry_url(&resources, origin, source)?;
    let (input, rx) = mpsc::sync_channel(256);
    let bytes = Arc::new(AtomicUsize::new(0));
    let cancelled = Arc::new(AtomicBool::new(false));
    let actor = Actor {
        input,
        bytes: bytes.clone(),
        cancelled: cancelled.clone(),
    };
    let supervisor = Supervisor {
        worker: engine.executable.clone(),
        library: engine.library.clone(),
        timeout: engine.timeout,
        cancelled,
        delivery,
    };
    thread::Builder::new()
        .name("qml-workerscript".into())
        .spawn(move || {
            if let Err(error) = supervisor.run(resources, url, rx, &bytes) {
                if !supervisor.cancelled.load(Ordering::Acquire) {
                    supervisor
                        .delivery
                        .publish(WorkerEvent::Stopped(error.message));
                }
            }
        })
        .map_err(|e| Error::new(e.to_string()))?;
    Ok(Box::new(actor))
}
impl Supervisor {
    fn run(
        &self,
        resources: Arc<BTreeMap<String, ir::ScriptResource>>,
        url: String,
        inbox: mpsc::Receiver<(WorkerValue, usize)>,
        bytes: &AtomicUsize,
    ) -> Result<(), Error> {
        let init = worker_program::initialization(&resources, &url)?;
        let child = process::Worker::spawn_mode(&self.worker, &self.library, true)?;
        self.exchange(&child, init)?;
        while let Ok((value, size)) = inbox.recv() {
            bytes.fetch_sub(size, Ordering::Release);
            if self.cancelled.load(Ordering::Acquire) {
                break;
            }
            self.exchange(&child, json!({"message":value}))?;
        }
        Ok(())
    }

    fn exchange(&self, child: &process::Worker, message: Json) -> Result<(), Error> {
        let deadline = Instant::now()
            .checked_add(self.timeout)
            .ok_or_else(|| Error::new("invalid worker deadline"))?;
        child.send(message, deadline)?;
        loop {
            let reply = child.read_cancellable(deadline, &self.cancelled)?;
            if let Some(reason) = reply["failure"].as_str() {
                return Err(Error::new(reason));
            }
            if reply["done"] == true {
                return Ok(());
            }
            let wire = reply
                .get("host")
                .ok_or_else(|| Error::new("invalid WorkerScript frame"))?;
            let event: WorkerEvent = serde_json::from_value(wire.clone())
                .map_err(|e| Error::new(format!("invalid WorkerScript event: {e}")))?;
            let sync = match event {
                WorkerEvent::Sync {
                    id,
                    revision,
                    edits,
                } => Some((self.delivery.synchronize(id, revision, edits)?, true)),
                WorkerEvent::SyncTree(models) => {
                    Some((self.delivery.synchronize_tree(models)?, false))
                }
                event => {
                    if !self.delivery.publish(event) {
                        return Err(Error::new("WorkerScript receiver unavailable"));
                    }
                    None
                }
            };
            let reply = match sync {
                Some((ack, single)) => loop {
                    if self.cancelled.load(Ordering::Acquire) {
                        return Err(Error::new("WorkerScript stopped"));
                    }
                    match ack.recv_revisions(
                        deadline
                            .saturating_duration_since(Instant::now())
                            .min(Duration::from_millis(25)),
                    ) {
                        Ok(Ok(revisions)) => {
                            break if single {
                                json!({"ok":revisions[0]})
                            } else {
                                json!({"ok":revisions})
                            }
                        }
                        Ok(Err(e)) => break json!({"error":e.message}),
                        Err(mpsc::RecvTimeoutError::Timeout) if Instant::now() < deadline => {
                            continue
                        }
                        Err(_) => {
                            return Err(Error::new("WorkerScript.sync owner delivery timed out"))
                        }
                    }
                },
                None => json!({"ok":true}),
            };
            child.send(reply, deadline)?;
        }
    }
}
