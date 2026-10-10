//! The functions of the session management library the platform calls
//! (the port of `SMLib.cs`). `libSM.so.6` is opened at run time.

use crate::interop::native_library::{native_functions, NativeLibrary, NativeLibraryError};
use std::ffi::{c_char, c_int, c_ulong, c_void, CStr};
use std::sync::OnceLock;

const LIB_SM: &CStr = c"libSM.so.6";

/// `SmDialogValue`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum SmDialogValue {
    SmDialogError = 0,
}

/// `SmcSaveYourselfProc`.
pub type SmcSaveYourselfProc = unsafe extern "C" fn(
    smc_conn: *mut c_void,
    client_data: *mut c_void,
    save_type: c_int,
    shutdown: c_int,
    interact_style: c_int,
    fast: c_int,
);
/// A connection and the client data: the shape of the four callbacks
/// below.
pub type SmcProc = unsafe extern "C" fn(smc_conn: *mut c_void, client_data: *mut c_void);
pub type SmcDieProc = SmcProc;
pub type SmcInteractProc = SmcProc;
pub type SmcSaveCompleteProc = SmcProc;
pub type SmcShutdownCancelledProc = SmcProc;
/// `SmcErrorHandler` (and `IceErrorHandler`, which has the same shape).
pub type SmcErrorHandler = unsafe extern "C" fn(
    conn: *mut c_void,
    swap: c_int,
    offending_minor_opcode: c_int,
    offending_sequence: c_ulong,
    error_class: c_int,
    severity: c_int,
    values: *mut c_void,
);
/// `IceWatchProc`.
pub type IceWatchProc =
    unsafe extern "C" fn(ice_conn: *mut c_void, client_data: *mut c_void, opening: c_int, watch_data: *mut *mut c_void);

/// `SmcCallbacks`: each callback with its client data.
#[repr(C)]
pub struct SmcCallbacks {
    pub save_yourself: Option<SmcSaveYourselfProc>,
    pub save_yourself_client_data: *mut c_void,
    pub die: Option<SmcDieProc>,
    pub die_client_data: *mut c_void,
    pub save_complete: Option<SmcSaveCompleteProc>,
    pub save_complete_client_data: *mut c_void,
    pub shutdown_cancelled: Option<SmcShutdownCancelledProc>,
    pub shutdown_cancelled_client_data: *mut c_void,
}

native_functions! {
    /// The functions, as C declares them (a `Bool` is an `int`).
    pub struct SMLib(sm) {
        sm::smc_open_connection as "SmcOpenConnection": unsafe extern "C" fn(
            *const c_char,
            *mut c_void,
            c_int,
            c_int,
            c_ulong,
            *mut SmcCallbacks,
            *const c_char,
            *mut *mut c_char,
            c_int,
            *mut c_char,
        ) -> *mut c_void;
        sm::smc_close_connection as "SmcCloseConnection": unsafe extern "C" fn(*mut c_void, c_int, *mut *mut c_char) -> c_int;
        sm::smc_save_yourself_done as "SmcSaveYourselfDone": unsafe extern "C" fn(*mut c_void, c_int);
        sm::smc_interact_request as "SmcInteractRequest": unsafe extern "C" fn(*mut c_void, c_int, SmcInteractProc, *mut c_void) -> c_int;
        sm::smc_interact_done as "SmcInteractDone": unsafe extern "C" fn(*mut c_void, c_int);
        sm::smc_get_ice_connection as "SmcGetIceConnection": unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        sm::smc_set_error_handler as "SmcSetErrorHandler": unsafe extern "C" fn(Option<SmcErrorHandler>) -> Option<SmcErrorHandler>;
    }
}

/// The library, opened at the first call.
pub fn sm_lib() -> Result<&'static SMLib, NativeLibraryError> {
    static API: OnceLock<Result<SMLib, NativeLibraryError>> = OnceLock::new();
    API.get_or_init(|| SMLib::load(&NativeLibrary::open(LIB_SM)?)).as_ref().map_err(Clone::clone)
}

#[cfg(test)]
mod tests {
    // Not from the reference.
    use super::*;

    #[test]
    fn the_callbacks_have_the_layout_of_the_library() {
        // Four pairs of a function and its client data.
        assert_eq!(std::mem::size_of::<SmcCallbacks>(), 8 * std::mem::size_of::<usize>());
        assert_eq!(std::mem::offset_of!(SmcCallbacks, die), 2 * std::mem::size_of::<usize>());
        assert_eq!(std::mem::offset_of!(SmcCallbacks, shutdown_cancelled), 6 * std::mem::size_of::<usize>());
    }

    #[test]
    fn the_library_loads_or_says_that_it_is_missing() {
        if let Err(error) = sm_lib() {
            assert!(error.to_string().contains("libSM.so.6"), "{error}");
        }
    }
}
