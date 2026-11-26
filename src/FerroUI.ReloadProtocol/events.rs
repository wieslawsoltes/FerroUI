use super::fingerprint;
use serde::Serialize;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const PREFIX: &str = "FERROUI_RELOAD_V1=";
const FRAME_PREFIX: &str = "FERROUI_RELOAD_FRAME_V1=";
static NEXT_FRAME: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Xaml,
    Rsx,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    Ready,
    Applied,
    Failed,
    Rebuild,
}

#[derive(Serialize)]
struct Event<'a> {
    language: Language,
    status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    file: Option<&'a Path>,
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frame: Option<u64>,
}

pub fn enabled() -> bool {
    cfg!(debug_assertions)
        && std::env::var_os("FERROUI_EDITOR_RELOAD").as_deref() == Some("1".as_ref())
}

pub fn ready(language: Language) {
    report_ready(language, None);
}

fn report_ready(language: Language, frame: Option<u64>) {
    if !enabled() {
        return;
    }
    emit(PREFIX, Event {
        language,
        status: Status::Ready,
        file: None,
        revision: None,
        message: None,
        frame,
    });
}

pub fn source(language: Language, status: Status, file: &Path, text: &str, message: Option<&str>) {
    let _ = report_source(language, status, file, text, message, false);
}

pub fn source_with_frame(language: Language, file: &Path, text: &str, message: Option<&str>) -> Option<FrameRequest> {
    report_source(language, Status::Applied, file, text, message, true)
}

fn report_source(language: Language, status: Status, file: &Path, text: &str, message: Option<&str>, observe_frame: bool) -> Option<FrameRequest> {
    if !enabled() {
        return None;
    }
    let frame = FrameRequest::new(observe_frame, Some(Instant::now()));
    emit(PREFIX, Event {
        language,
        status,
        file: Some(file),
        revision: Some(format!("{:016x}", fingerprint(text.as_bytes()))),
        message,
        frame: frame.as_ref().map(|request| request.id),
    });
    frame
}

/// Share a managed application's MCP listener with its supervising editor over stderr.
/// Returns false when the application was started outside a development editor.
pub fn mcp_endpoint(address: SocketAddr, token: &str) -> bool {
    if !enabled() || !address.ip().is_loopback() {
        return false;
    }
    #[derive(Serialize)]
    struct Endpoint<'a> {
        address: SocketAddr,
        token: &'a str,
    }
    match serde_json::to_string(&Endpoint { address, token }) {
        Ok(json) => {
            eprintln!("FERROUI_MCP_V1={json}");
            true
        }
        Err(_) => false,
    }
}

/// A within-process identity from an acknowledgement. The owning editor
/// associates its process generation and exact file revision with this ID.
#[derive(Clone, Copy)]
pub struct FrameRequest {
    id: u64,
    acknowledged_at: Option<Instant>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FrameStatus {
    Rendered,
    NoSceneUpdate,
    Discarded,
}

impl FrameRequest {
    fn new(observe: bool, acknowledged_at: Option<Instant>) -> Option<Self> {
        (observe && std::env::var_os("FERROUI_RELOAD_FRAMES").as_deref() == Some("1".as_ref()))
            .then(|| Self { id: NEXT_FRAME.fetch_add(1, Ordering::Relaxed), acknowledged_at })
    }

    /// Reserve a receipt before showing a window. The platform can paint
    /// during Show, before the application acknowledges that it opened.
    pub fn startup() -> Option<Self> {
        Self::new(enabled(), None)
    }

    pub fn ready(self, language: Language) {
        report_ready(language, Some(self.id));
    }

    /// Complete this acknowledgement with the observed compositor batch.
    pub fn complete(self, batch: i64, status: FrameStatus) {
        Self::complete_batch([self], batch, status);
    }

    /// Complete a group of acknowledgements with one compositor timestamp.
    pub fn complete_batch(requests: impl IntoIterator<Item = Self>, batch: i64, status: FrameStatus) {
        let now = Instant::now();
        let completed = SystemTime::now().duration_since(UNIX_EPOCH).ok();
        for request in requests {
            emit(FRAME_PREFIX, serde_json::json!({
                "frame": request.id,
                "batch": batch,
                "status": status,
                "unixMs": completed.map(|time| time.as_millis()),
                "afterAcknowledgementMs": request.acknowledged_at.map(|ack| now.saturating_duration_since(ack).as_secs_f64() * 1_000.0),
            }));
        }
    }
}

fn emit(prefix: &str, event: impl Serialize) {
    if let Ok(json) = serde_json::to_string(&event) {
        eprintln!("{prefix}{json}");
    }
}
