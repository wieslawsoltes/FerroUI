//! Development acknowledgements for a child started by an editor. The child
//! reports the exact source bytes it observed over its existing stderr pipe.
use serde::Serialize;
use std::path::Path;

const PREFIX: &str = "FERROUI_RELOAD_V1=";

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
}

pub fn enabled() -> bool {
    cfg!(debug_assertions)
        && std::env::var_os("FERROUI_EDITOR_RELOAD").as_deref() == Some("1".as_ref())
}

pub fn ready(language: Language) {
    emit(Event {
        language,
        status: Status::Ready,
        file: None,
        revision: None,
        message: None,
    });
}

pub fn source(language: Language, status: Status, file: &Path, text: &str, message: Option<&str>) {
    if !enabled() {
        return;
    }
    // FNV-1a identifies identical observed bytes across Rust and the VS Code host.
    let hash = text.bytes().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    });
    emit(Event {
        language,
        status,
        file: Some(file),
        revision: Some(format!("{hash:016x}")),
        message,
    });
}

fn emit(event: Event<'_>) {
    if enabled() {
        if let Ok(json) = serde_json::to_string(&event) {
            eprintln!("{PREFIX}{json}");
        }
    }
}
