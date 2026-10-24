use crate::{regex_source::Captures, Error, Result};
use wasm_bindgen::JsValue;

mod bindings {
    include!(concat!(env!("OUT_DIR"), "/browser_bindings.rs"));
}

/// Initializes the bundled official Oniguruma WebAssembly engine once per JS
/// realm. Await this before tokenizing. No network request or asset URL is used.
/// A worker initializes its own engine, independently of the window.
pub async fn initialize_browser_oniguruma() -> Result<()> {
    let promise =
        bindings::initialize(include_bytes!("browser/vendor/onig.wasm")).map_err(js_error)?;
    wasm_bindgen_futures::JsFuture::from(promise)
        .await
        .map_err(js_error)?;
    Ok(())
}

pub fn browser_oniguruma_ready() -> bool {
    bindings::ready()
}

pub(crate) struct BrowserRegex {
    scanner: JsValue,
}
impl BrowserRegex {
    pub fn new(pattern: &str) -> Result<Self> {
        if !bindings::ready() {
            return Err(Error::BackendNotInitialized);
        }
        Ok(Self {
            scanner: bindings::compile(pattern).map_err(|e| Error::Regex {
                pattern: pattern.into(),
                message: describe(e),
            })?,
        })
    }
    pub fn search(&self, text: &str, from: usize) -> Result<Option<Captures>> {
        let utf16_start = text[..from].encode_utf16().count();
        let captures = bindings::search(
            &self.scanner,
            text,
            u32::try_from(utf16_start)
                .map_err(|_| Error::RegexLimit("line exceeds JavaScript index range".into()))?,
        )
        .map_err(js_error)?;
        if captures.is_null() {
            return Ok(None);
        }
        let indices = js_sys::Array::from(&captures);
        let mut utf16_to_byte = Vec::with_capacity(text.len() + 1);
        for (byte, character) in text.char_indices() {
            for _ in 0..character.len_utf16() {
                utf16_to_byte.push(byte);
            }
        }
        utf16_to_byte.push(text.len());
        Ok(Some(
            indices
                .iter()
                .map(|capture| {
                    let capture = js_sys::Array::from(&capture);
                    let start = capture.get(0).as_f64()?;
                    let end = capture.get(1).as_f64()?;
                    if start < 0.0 || end < start {
                        return None;
                    }
                    Some((
                        *utf16_to_byte.get(start as usize)?,
                        *utf16_to_byte.get(end as usize)?,
                    ))
                })
                .collect(),
        ))
    }
}
impl Drop for BrowserRegex {
    fn drop(&mut self) {
        bindings::dispose(&self.scanner);
    }
}
fn describe(value: JsValue) -> String {
    value
        .as_string()
        .unwrap_or_else(|| js_sys::Error::from(value).message().into())
}
fn js_error(value: JsValue) -> Error {
    Error::RegexLimit(describe(value))
}
