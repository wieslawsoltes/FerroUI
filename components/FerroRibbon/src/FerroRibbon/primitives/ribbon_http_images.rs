//! HTTP transport shared by the bitmap and SVG icon paths.
use std::io;
#[cfg(any(feature = "http-images", target_os = "emscripten"))]
use std::time::Duration;

#[cfg(target_os = "emscripten")]
mod browser;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-images"))]
mod native;

#[cfg(target_os = "emscripten")]
pub(super) use browser::read_http_image;
#[cfg(all(not(target_arch = "wasm32"), feature = "http-images"))]
pub(super) use native::read_http_image;

/// Without the `http-images` feature a remote icon fails to load like any unreachable one.
#[cfg(all(not(target_arch = "wasm32"), not(feature = "http-images")))]
pub(super) async fn read_http_image(_uri: &ferroui_base::utilities::UriRef) -> io::Result<Vec<u8>> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "HTTP icons need the `http-images` feature of ferroribbon",
    ))
}

#[cfg(any(feature = "http-images", target_os = "emscripten"))]
const MAX_IMAGE_BYTES: usize = 16 * 1024 * 1024;
#[cfg(any(feature = "http-images", target_os = "emscripten"))]
const TIMEOUT: Duration = Duration::from_secs(30);

#[cfg(any(feature = "http-images", target_os = "emscripten"))]
fn image_too_large() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "HTTP image exceeds 16 MiB")
}
