use super::fingerprint;
use serde::Serialize;
use std::net::SocketAddr;
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
    emit(Event {
        language,
        status,
        file: Some(file),
        revision: Some(format!("{:016x}", fingerprint(text.as_bytes()))),
        message,
    });
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

fn emit(event: Event<'_>) {
    if enabled() {
        if let Ok(json) = serde_json::to_string(&event) {
            eprintln!("{PREFIX}{json}");
        }
    }
}
