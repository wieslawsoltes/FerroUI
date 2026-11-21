//! Common data for the ICU port in the pinned Emscripten SDK.
use std::{ffi::c_void, sync::OnceLock};

const DATA_LENGTH: usize = include_bytes!(env!("FERRORIBBON_BROWSER_ICU_DATA")).len();

#[repr(C, align(16))]
struct CommonData([u8; DATA_LENGTH]);

static DATA: CommonData = CommonData(*include_bytes!(env!("FERRORIBBON_BROWSER_ICU_DATA")));
static STATUS: OnceLock<i32> = OnceLock::new();

unsafe extern "C" {
    fn udata_setCommonData_68(data: *const c_void, status: *mut i32);
}

pub(super) fn initialize() -> i32 {
    *STATUS.get_or_init(|| {
        let mut status = 0;
        // The ICU port supplies this little-endian data image. Its aligned
        // storage outlives every collation and number-formatting operation.
        unsafe { udata_setCommonData_68(DATA.0.as_ptr().cast(), &mut status) };
        status
    })
}
