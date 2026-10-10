//! The functions of the ICE library the platform calls (the port of
//! `ICELib.cs`): the transport of the session management protocol.
//! `libICE.so.6` is opened at run time.

use crate::interop::native_library::{native_functions, NativeLibrary, NativeLibraryError};
use crate::sm_lib::{IceWatchProc, SmcErrorHandler};
use std::ffi::{c_int, c_void, CStr};
use std::sync::OnceLock;

const LIB_ICE: &CStr = c"libICE.so.6";

/// `IceProcessMessagesStatus`: the one value that is looked at.
pub const ICE_PROCESS_MESSAGES_IO_ERROR: c_int = 1;

/// `IceErrorHandler`: the shape of the error handler of the session
/// management library.
pub type IceErrorHandler = SmcErrorHandler;
/// `IceIOErrorHandler`.
pub type IceIOErrorHandler = unsafe extern "C" fn(ice_conn: *mut c_void);

native_functions! {
    /// The functions, as C declares them.
    pub struct IceLibApi(ice) {
        ice::IceAddConnectionWatch: unsafe extern "C" fn(IceWatchProc, *mut c_void) -> c_int;
        ice::IceRemoveConnectionWatch: unsafe extern "C" fn(IceWatchProc, *mut c_void);
        ice::IceProcessMessages: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_int) -> c_int;
        ice::IceSetErrorHandler: unsafe extern "C" fn(Option<IceErrorHandler>) -> Option<IceErrorHandler>;
        ice::IceSetIOErrorHandler: unsafe extern "C" fn(Option<IceIOErrorHandler>) -> Option<IceIOErrorHandler>;
    }
}

/// The library, opened at the first call.
pub fn ice_lib() -> Result<&'static IceLibApi, NativeLibraryError> {
    static API: OnceLock<Result<IceLibApi, NativeLibraryError>> = OnceLock::new();
    API.get_or_init(|| IceLibApi::load(&NativeLibrary::open(LIB_ICE)?)).as_ref().map_err(Clone::clone)
}

#[cfg(test)]
mod tests {
    // Not from the reference.
    use super::*;

    #[test]
    fn the_library_loads_or_says_that_it_is_missing() {
        if let Err(error) = ice_lib() {
            assert!(error.to_string().contains("libICE.so.6"), "{error}");
        }
    }
}
