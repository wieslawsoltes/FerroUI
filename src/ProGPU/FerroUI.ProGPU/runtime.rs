//! The ProGPU runtime libraries, loaded once per process (docs/porting/progpu-backend.md, section 2.2).
//!
//! The device-independent parts of the engine (geometry queries, path measures) are called from any
//! thread without a device; they share the one loaded runtime.

use progpu_native_sys::{LoadError, Runtime};
use std::sync::OnceLock;

static RUNTIME: OnceLock<Result<Runtime, String>> = OnceLock::new();

/// The loaded runtime of the process, loaded on first use.
pub fn runtime() -> Result<&'static Runtime, &'static str> {
    RUNTIME.get_or_init(|| Runtime::load().map_err(|error: LoadError| error.to_string())).as_ref().map_err(String::as_str)
}
