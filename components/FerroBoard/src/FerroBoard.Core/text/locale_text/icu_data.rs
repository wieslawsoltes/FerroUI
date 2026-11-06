//! The selected Emscripten ICU port links stub data. Supply the matching, unmodified
//! ICU 68.2 package before opening a collator. See resources/native/icu/provenance.json.
use super::{CollationError, check, ffi};
use std::sync::OnceLock;

// ICU requires common data to be aligned and to outlive every ICU object using it.
// A static allocation provides both, including when separate collators are retired.
#[repr(align(16))]
struct CommonData([u8; 28_566_176]);
static DATA: CommonData = CommonData(*include_bytes!("../../resources/native/icu/icudt68l.dat"));
static INSTALLED: OnceLock<Result<(), CollationError>> = OnceLock::new();

pub(super) fn install() -> Result<(), CollationError> {
    INSTALLED
        .get_or_init(|| {
            let mut status = 0;
            // SAFETY: immutable ICU package, 16-byte alignment, process-long storage;
            // OnceLock serializes registration before any Core collator is opened.
            unsafe { ffi::set_common_data(DATA.0.as_ptr().cast(), &mut status) };
            check(status, "udata_setCommonData")
        })
        .clone()
}
