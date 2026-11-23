//! Opt-in Live App endpoint for a debuggable Android application.
//!
//! The inspector listens on Android loopback and writes a private discovery file
//! that the app or an authorized Android Debug Bridge `run-as` session can read.

use crate::interop::java::{call_object, string_of};
use crate::FerroAndroidApplication;
use ferroui_mcp::{Options, ServerHandle, Transport};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::net::Ipv4Addr;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

const RECEIPT: &str = "ferroui-live-app.json";
static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Keep this handle for the lifetime of the application.
pub struct LiveAppInspector {
    _server: ServerHandle,
    _receipt: Receipt,
}

struct Receipt(Option<PathBuf>);

impl Drop for Receipt {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = fs::remove_file(path);
        }
        ACTIVE.store(false, Ordering::Release);
    }
}

/// Start on the UI thread once the Android application has initialized its platform.
/// The endpoint is available to VS Code through the debuggable application's sandbox.
pub fn serve(options: Options) -> io::Result<LiveAppInspector> {
    if !cfg!(debug_assertions) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Live App requires a debug build",
        ));
    }
    if !matches!(&options.transport, Transport::Http(address) if address.ip() == Ipv4Addr::LOCALHOST)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Live App requires IPv4 loopback HTTP",
        ));
    }
    let destination = files_dir()?.join(RECEIPT);
    if ACTIVE.swap(true, Ordering::AcqRel) {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "The Live App inspector is already active",
        ));
    }
    let mut receipt = Receipt(None);
    let server = ferroui_mcp::serve_private(options)?;
    write_receipt(&destination, &server)?;
    receipt.0 = Some(destination);
    Ok(LiveAppInspector {
        _server: server,
        _receipt: receipt,
    })
}

fn files_dir() -> io::Result<PathBuf> {
    let application = FerroAndroidApplication::try_context()
        .ok_or_else(|| io::Error::other("The Android application has not initialized"))?;
    let directory = call_object(&application, "getFilesDir", "()Ljava/io/File;", &[])
        .and_then(|directory| {
            call_object(&directory, "getAbsolutePath", "()Ljava/lang/String;", &[])
        })
        .ok_or_else(|| {
            io::Error::other("Android could not open the application's files directory")
        })?;
    Ok(PathBuf::from(string_of(&directory)))
}

fn write_receipt(destination: &Path, server: &ServerHandle) -> io::Result<()> {
    let (address, token) = server
        .address
        .zip(server.token.as_deref())
        .ok_or_else(|| io::Error::other("Live App could not create its HTTP endpoint"))?;
    let pid = std::process::id();
    let payload = serde_json::json!({"version":1,"pid":pid,"port":address.port(),"token":token});
    let temporary = destination.with_extension(format!("{pid}.{}.tmp", address.port()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    let result = file
        .write_all(payload.to_string().as_bytes())
        .and_then(|_| fs::rename(&temporary, destination));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
