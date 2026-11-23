//! The table of shader libraries made ahead, in front of the shader compiler of a Metal device
//! (`shader_libraries.m`; the table is `crate::metal_libraries`).

use std::ffi::c_void;

unsafe extern "C" {
    fn ferroui_skia_metal_libraries_install(device: *mut c_void) -> bool;
}

/// Puts the table in front of the shader compiler of `device` (an `id<MTLDevice>`), when
/// there is a table or a folder to record to. Before the device makes its first pipeline.
pub(crate) fn install(device: *mut c_void) -> bool {
    if device.is_null() || !crate::metal_libraries::wanted() {
        return false;
    }
    // SAFETY: `device` is a live `id<MTLDevice>`; the call changes one method of its class once.
    unsafe { ferroui_skia_metal_libraries_install(device) }
}

/// The library of a shader text, for `shader_libraries.m`: null when the table has none.
///
/// # Safety
/// `source` is `length` readable bytes and `library_length` a writable `usize`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ferroui_skia_metal_library_for(
    source: *const u8,
    length: usize,
    language_version: u64,
    plain_options: bool,
    library_length: *mut usize,
) -> *const u8 {
    if source.is_null() || library_length.is_null() {
        return std::ptr::null();
    }
    // SAFETY: the caller's contract.
    let text = unsafe { std::slice::from_raw_parts(source, length) };
    match crate::metal_libraries::library_for(text, language_version, plain_options) {
        Some(library) => {
            // SAFETY: the caller's contract.
            unsafe { *library_length = library.len() };
            library.as_ptr()
        }
        None => std::ptr::null(),
    }
}
