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

pub(crate) struct BrowserLine(JsValue);

impl BrowserLine {
    pub fn new(text: &str) -> Result<Self> {
        bindings::create_line(text).map(Self).map_err(js_error)
    }
}

impl Drop for BrowserLine {
    fn drop(&mut self) {
        bindings::dispose_line(&self.0);
    }
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

    #[cfg(not(feature = "portable-regex"))]
    pub fn group(patterns: &[std::borrow::Cow<'_, str>]) -> Option<Self> {
        let patterns: js_sys::Array = patterns.iter().map(|p| JsValue::from_str(p)).collect();
        bindings::compile_group(&patterns)
            .ok()
            .map(|scanner| Self { scanner })
    }

    pub fn search(&self, line: &BrowserLine, from: usize) -> Result<Option<Captures>> {
        Ok(self.search_group(line, from)?.map(|(_, captures)| captures))
    }

    pub fn search_group(
        &self,
        line: &BrowserLine,
        from: usize,
    ) -> Result<Option<(usize, Captures)>> {
        let Some(captures) =
            bindings::search(&self.scanner, &line.0, from as u32).map_err(js_error)?
        else {
            return Ok(None);
        };
        let indices = captures.to_vec();
        let (pattern, ranges) = indices
            .split_first()
            .expect("scanner returns a pattern index");
        Ok(Some((
            *pattern as usize,
            ranges
                .chunks_exact(2)
                .map(|pair| (pair[0] != u32::MAX).then_some((pair[0] as usize, pair[1] as usize)))
                .collect(),
        )))
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
