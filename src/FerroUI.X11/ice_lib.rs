//! The functions of the ICE library the platform calls (the port of
//! `ICELib.cs`): the transport of the session management protocol.
//! `libICE.so.6` is opened at run time.

use crate::interop::native_library::{native_functions, NativeLibrary, NativeLibraryError};
use crate::sm_lib::{IceWatchProc, SmcErrorHandler};
use std::ffi::{c_int, c_void, CStr};
use std::sync::OnceLock;

const LIB_ICE: &CStr = c"libICE.so.6";

/// `IceProcessMessagesStatus`: the one value that is looked at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum IceProcessMessagesStatus {
    IceProcessMessagesIoError = 1,
}

/// `IceErrorHandler`: the shape of the error handler of the session
/// management library.
pub type IceErrorHandler = SmcErrorHandler;
/// `IceIOErrorHandler`.
pub type IceIOErrorHandler = unsafe extern "C" fn(ice_conn: *mut c_void);

native_functions! {
    /// The functions, as C declares them.
    pub struct ICELib(ice) {
        ice::ice_add_connection_watch as "IceAddConnectionWatch": unsafe extern "C" fn(IceWatchProc, *mut c_void) -> c_int;
        ice::ice_remove_connection_watch as "IceRemoveConnectionWatch": unsafe extern "C" fn(IceWatchProc, *mut c_void);
        ice::ice_process_messages as "IceProcessMessages": unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_int) -> c_int;
        ice::ice_set_error_handler as "IceSetErrorHandler": unsafe extern "C" fn(Option<IceErrorHandler>) -> Option<IceErrorHandler>;
        ice::ice_set_io_error_handler as "IceSetIOErrorHandler": unsafe extern "C" fn(Option<IceIOErrorHandler>) -> Option<IceIOErrorHandler>;
    }
}

/// The library, opened at the first call.
pub fn ice_lib() -> Result<&'static ICELib, NativeLibraryError> {
    static API: OnceLock<Result<ICELib, NativeLibraryError>> = OnceLock::new();
    API.get_or_init(|| ICELib::load(&NativeLibrary::open(LIB_ICE)?)).as_ref().map_err(Clone::clone)
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
