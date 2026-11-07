//! The browser's locale service on Emscripten: `Intl.Collator` and `Intl.Segmenter` of the
//! JavaScript engine (`collation.js`), which carry the browser's own ICU and CLDR data. As on the
//! other platforms, the module links no ICU of its own and embeds no locale data.
use super::CollationError;
use std::{cmp::Ordering, marker::PhantomData, rc::Rc};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/text/locale_text/collation.js")]
extern "C" {
    type BrowserCollator;
    #[wasm_bindgen(catch, js_name = openCollator)]
    fn open_collator(locale: &str) -> Result<BrowserCollator, JsValue>;
    #[wasm_bindgen(method, catch)]
    fn compare(this: &BrowserCollator, left: &[u16], right: &[u16], ignore_case: bool) -> Result<i32, JsValue>;
    #[wasm_bindgen(method, catch)]
    fn contains(this: &BrowserCollator, text: &[u16], pattern: &[u16], ignore_case: bool) -> Result<bool, JsValue>;
}

/// An engine failure (an exception of `Intl`) as the error of a native call.
fn native(operation: &'static str) -> impl FnOnce(JsValue) -> CollationError {
    move |_| CollationError::Native { operation, code: -1 }
}

pub(super) struct Collator {
    inner: BrowserCollator,
    // A JavaScript object belongs to the realm of its thread.
    _owner_thread: PhantomData<Rc<()>>,
}
impl Collator {
    pub(super) fn new(locale: &str) -> Result<Self, CollationError> {
        Ok(Self {
            inner: open_collator(locale).map_err(native("Intl.Collator"))?,
            _owner_thread: PhantomData,
        })
    }
    pub(super) fn compare(
        &self,
        left: &[u16],
        right: &[u16],
        ignore_case: bool,
    ) -> Result<Ordering, CollationError> {
        let result = self
            .inner
            .compare(left, right, ignore_case)
            .map_err(native("Intl.Collator.compare"))?;
        Ok(result.cmp(&0))
    }
    pub(super) fn contains(
        &self,
        text: &[u16],
        pattern: &[u16],
        ignore_case: bool,
    ) -> Result<bool, CollationError> {
        if pattern.is_empty() || self.compare(pattern, &[], ignore_case)? == Ordering::Equal {
            return Ok(true);
        }
        if text.is_empty() {
            return Ok(false);
        }
        self.inner
            .contains(text, pattern, ignore_case)
            .map_err(native("Intl.Segmenter"))
    }
}
