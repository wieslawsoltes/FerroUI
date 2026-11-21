use super::{MAX_IMAGE_BYTES, TIMEOUT, image_too_large};
use ferroui_base::utilities::UriRef;
use js_sys::{Reflect, Uint8Array};
use std::io;
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use web_sys::{AbortController, ReadableStreamDefaultReader, RequestInit, Response, Window};

/// Dropping an icon cancels its fetch and clears the timeout, including while
/// the response body is streaming.
struct FetchRequest {
    window: Window,
    controller: AbortController,
    timer: i32,
    _on_timeout: Closure<dyn FnMut()>,
}

impl Drop for FetchRequest {
    fn drop(&mut self) {
        self.window.clear_timeout_with_handle(self.timer);
        self.controller.abort();
    }
}

pub(in crate::primitives) async fn read_http_image(uri: &UriRef) -> io::Result<Vec<u8>> {
    let url = uri.as_url().expect("resolved HTTP URI");
    let window = web_sys::window().ok_or_else(|| io::Error::other("Missing browser window"))?;
    let controller = AbortController::new().map_err(browser_error)?;
    let on_timeout = {
        let controller = controller.clone();
        Closure::once(move || controller.abort())
    };
    let timer = window
        .set_timeout_with_callback_and_timeout_and_arguments_0(
            on_timeout.as_ref().unchecked_ref(),
            TIMEOUT.as_millis() as i32,
        )
        .map_err(browser_error)?;
    let request = FetchRequest {
        window,
        controller,
        timer,
        _on_timeout: on_timeout,
    };
    let result = download(&request, url.as_str()).await;
    if request.controller.signal().aborted() {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "HTTP image timed out",
        ));
    }
    result
}

async fn download(request: &FetchRequest, url: &str) -> io::Result<Vec<u8>> {
    let init = RequestInit::new();
    init.set_signal(Some(&request.controller.signal()));
    let response = request
        .window
        .fetch_with_str_and_init(url, &init)
        .await
        .and_then(JsCast::dyn_into::<Response>)
        .map_err(browser_error)?;
    if !response.ok() {
        return Err(io::Error::other(format!("HTTP {}", response.status())));
    }
    if response
        .headers()
        .get("content-length")
        .map_err(browser_error)?
        .and_then(|length| length.parse::<u64>().ok())
        .is_some_and(|length| length > MAX_IMAGE_BYTES as u64)
    {
        return Err(image_too_large());
    }

    let mut bytes = Vec::new();
    let Some(body) = response.body() else {
        return Ok(bytes);
    };
    let reader = ReadableStreamDefaultReader::new(&body).map_err(browser_error)?;
    loop {
        let next = reader.read().await.map_err(browser_error)?;
        let done = Reflect::get(&next, &JsValue::from_str("done")).map_err(browser_error)?;
        if done.is_truthy() {
            reader.release_lock();
            return Ok(bytes);
        }
        let chunk = Reflect::get(&next, &JsValue::from_str("value"))
            .and_then(JsCast::dyn_into::<Uint8Array>)
            .map_err(browser_error)?;
        let length = chunk.length() as usize;
        let start = bytes.len();
        if length > MAX_IMAGE_BYTES - start {
            return Err(image_too_large());
        }
        bytes.resize(start + length, 0);
        chunk.copy_to(&mut bytes[start..]);
    }
}

fn browser_error(error: JsValue) -> io::Error {
    let detail = error.as_string().or_else(|| {
        Reflect::get(&error, &JsValue::from_str("message"))
            .ok()?
            .as_string()
    });
    io::Error::other(detail.unwrap_or_else(|| "Browser HTTP image request failed".into()))
}
