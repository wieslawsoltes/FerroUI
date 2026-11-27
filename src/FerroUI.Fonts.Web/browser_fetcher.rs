//! Fonts over the network in the browser: `fetch()`, with the HTTP cache of the browser.

use crate::web_font_collection::{FontFetchError, FontFetchResult};
use wasm_bindgen::prelude::*;

// The mode and credentials of a font request of CSS. The body is read as a stream so that a
// response over the limit is cancelled, whatever its headers say.
#[wasm_bindgen(inline_js = r#"
export function ferroFetchFont(url, maxSize, done) {
    let finished = false;
    const finish = (bytes, code) => { if (!finished) { finished = true; done(bytes, code); } };
    (async () => {
        const response = await fetch(url, { mode: "cors", credentials: "same-origin" });
        if (!response.ok) {
            finish(undefined, response.status === 404 || response.status === 410 ? 1 : 2);
            return;
        }
        if (Number(response.headers.get("Content-Length")) > maxSize) {
            await response.body?.cancel();
            finish(undefined, 3);
            return;
        }
        if (!response.body) {
            const whole = new Uint8Array(await response.arrayBuffer());
            finish(whole.length > maxSize ? undefined : whole, whole.length > maxSize ? 3 : 0);
            return;
        }
        const reader = response.body.getReader();
        const chunks = [];
        let total = 0;
        for (;;) {
            const { done: end, value } = await reader.read();
            if (end) break;
            total += value.length;
            if (total > maxSize) {
                await reader.cancel();
                finish(undefined, 3);
                return;
            }
            chunks.push(value);
        }
        const bytes = new Uint8Array(total);
        let at = 0;
        for (const chunk of chunks) {
            bytes.set(chunk, at);
            at += chunk.length;
        }
        finish(bytes, 0);
    })().catch(() => finish(undefined, 2));
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = ferroFetchFont)]
    fn fetch_font(url: &str, max_size: f64, done: &JsValue);
}

// The Local Font Access API. It asks for a permission, so it runs only when the application asks,
// which has to be from a user gesture.
#[wasm_bindgen(inline_js = r#"
let localFonts = null;
export function ferroQueryLocalFonts(done) {
    if (typeof globalThis.queryLocalFonts !== "function") { done(false); return; }
    globalThis.queryLocalFonts().then((fonts) => {
        localFonts = new Map();
        for (const font of fonts) {
            localFonts.set(font.fullName.toLowerCase(), font);
            localFonts.set(font.postscriptName.toLowerCase(), font);
        }
        done(true);
    }).catch(() => done(false));
}
export function ferroHasLocalFont(name) {
    return localFonts !== null && localFonts.has(name.toLowerCase());
}
export function ferroReadLocalFont(name, maxSize, done) {
    const font = localFonts?.get(name.toLowerCase());
    if (!font) { done(undefined, 1); return; }
    font.blob().then(async (blob) => {
        if (blob.size > maxSize) { done(undefined, 3); return; }
        done(new Uint8Array(await blob.arrayBuffer()), 0);
    }).catch(() => done(undefined, 2));
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = ferroQueryLocalFonts)]
    fn query_local_fonts(done: &JsValue);
    #[wasm_bindgen(js_name = ferroHasLocalFont)]
    pub(crate) fn has_local_font(name: &str) -> bool;
    #[wasm_bindgen(js_name = ferroReadLocalFont)]
    fn read_local_font_raw(name: &str, max_size: f64, done: &JsValue);
}

/// Asks the page for the installed fonts. `done` says whether they can be read.
pub(crate) fn request_local_fonts(done: Box<dyn FnOnce(bool)>) {
    let done = std::panic::AssertUnwindSafe(done);
    let done = Closure::once_into_js(move |granted: bool| {
        let done = done;
        (done.0)(granted);
    });
    query_local_fonts(&done);
}

/// The bytes of an installed font the page has been given access to.
pub(crate) fn read_local_font(name: &str, max_size: usize, completion: Box<dyn FnOnce(FontFetchResult)>) {
    read_local_font_raw(name, max_size as f64, &completion_callback(completion));
}

fn completion_callback(completion: Box<dyn FnOnce(FontFetchResult)>) -> JsValue {
    // A panic in a completion aborts the module, as for any callback of the page.
    let completion = std::panic::AssertUnwindSafe(completion);
    Closure::once_into_js(move |bytes: JsValue, code: u32| {
        let completion = completion;
        (completion.0)(match code {
            0 => Ok(js_sys::Uint8Array::new(&bytes).to_vec()),
            1 => Err(FontFetchError::NotFound),
            3 => Err(FontFetchError::Forbidden("the font is over the size limit".into())),
            _ => Err(FontFetchError::Network("the fetch failed".into())),
        });
    })
}

/// Fetches a font with the `fetch()` of the page. A cross-origin font needs the server's CORS
/// headers, and a Content Security Policy governs this request through `connect-src`.
pub(crate) fn fetch(url: &str, max_size: usize, completion: Box<dyn FnOnce(FontFetchResult)>) {
    fetch_font(url, max_size as f64, &completion_callback(completion));
}
