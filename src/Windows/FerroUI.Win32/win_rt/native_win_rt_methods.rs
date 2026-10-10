//! The functions of the Windows Runtime: string handles and activation.
//!
//! Not built here, with the stage that calls them (2c, composition): the
//! activation factory of the composition library
//! (`GetWindowsUICompositionActivationFactory`) and the dispatcher queue
//! controller (`CreateDispatcherQueueController` with its options).

use crate::interop::unmanaged_methods::{co_get_apartment_type, APTTYPE_MAINSTA, APTTYPE_STA};
use ferroui_base::threading::Dispatcher;
use ferroui_microcom::{ComPtr, Guid, HResult, IUnknown, Interface};
use std::ffi::c_void;
use std::cell::Cell;

// The reference imports these from the same libraries. On 32-bit x86 the
// exports have no decoration, where the "system" calling convention would
// add one to the name.
#[cfg_attr(not(target_arch = "x86"), link(name = "api-ms-win-core-winrt-string-l1-1-0", kind = "raw-dylib"))]
#[cfg_attr(
    target_arch = "x86",
    link(name = "api-ms-win-core-winrt-string-l1-1-0", kind = "raw-dylib", import_name_type = "undecorated")
)]
extern "system" {
    fn WindowsCreateString(source_string: *const u16, length: u32, string: *mut isize) -> i32;
    fn WindowsGetStringRawBuffer(string: isize, length: *mut u32) -> *const u16;
    fn WindowsDeleteString(string: isize) -> i32;
}

#[cfg_attr(not(target_arch = "x86"), link(name = "combase", kind = "raw-dylib"))]
#[cfg_attr(target_arch = "x86", link(name = "combase", kind = "raw-dylib", import_name_type = "undecorated"))]
extern "system" {
    fn RoInitialize(init_type: i32) -> i32;
    fn RoActivateInstance(activatable_class_id: isize, instance: *mut *mut c_void) -> i32;
    fn RoGetActivationFactory(activatable_class_id: isize, iid: *const Guid, factory: *mut *mut c_void) -> i32;
}

/// How the Windows Runtime is initialised on a thread.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms)]
pub(crate) enum RO_INIT_TYPE {
    /// Single-threaded application
    RO_INIT_SINGLETHREADED = 0,
    /// COM calls objects on any thread.
    RO_INIT_MULTITHREADED = 1,
}

thread_local! {
    // The reference has one flag for the process: its runtime initialises
    // COM on every thread it starts, so only the first activation has
    // anything to do. A thread of the port has no apartment until this
    // initialises it, so the flag is of the thread.
    static INITIALIZED: Cell<bool> = const { Cell::new(false) };
}

/// A failure code as an error; every success code (`S_FALSE` of a thread
/// that is initialised already among them) is a success, as for a call the
/// reference declares without `PreserveSig`.
fn check(result: i32) -> Result<(), HResult> {
    if result < 0 {
        Err(HResult(result as u32))
    } else {
        Ok(())
    }
}

/// The functions of the Windows Runtime.
pub(crate) struct NativeWinRTMethods;

impl NativeWinRTMethods {
    /// Creates a string handle with the text. The caller deletes it.
    pub(crate) fn windows_create_string(source_string: &str) -> Result<isize, HResult> {
        let text: Vec<u16> = source_string.encode_utf16().collect();
        let mut string = 0;
        // SAFETY: the text is valid for its length, which is passed; the
        // handle is written to a number of this frame. The system copies
        // the text.
        check(unsafe { WindowsCreateString(text.as_ptr(), text.len() as u32, &mut string) })?;
        Ok(string)
    }

    /// The text of a string handle; the empty text for the null handle.
    pub(crate) fn windows_get_string_raw_buffer(hstring: isize) -> String {
        let mut length = 0;
        // SAFETY: a string handle (the null handle is the empty string);
        // the buffer the system returns is valid for the length it reports
        // while the handle lives, which is through this call.
        unsafe {
            let buffer = WindowsGetStringRawBuffer(hstring, &mut length);
            if buffer.is_null() {
                return String::new();
            }
            String::from_utf16_lossy(std::slice::from_raw_parts(buffer, length as usize))
        }
    }

    /// Deletes a string handle.
    pub(crate) fn windows_delete_string(hstring: isize) {
        // SAFETY: a string handle the caller owns and does not use again.
        unsafe {
            WindowsDeleteString(hstring);
        }
    }

    /// Activates the runtime class of the name and asks the object for the
    /// interface.
    pub(crate) fn create_instance<T: Interface>(full_name: &str) -> Result<ComPtr<T>, HResult> {
        let s = HStringInterop::new(Some(full_name))?;
        Self::ensure_ro_initialized()?;
        let mut p_unk = std::ptr::null_mut();
        // SAFETY: a string handle that lives through the call, and a
        // pointer of this frame the system writes the object to.
        check(unsafe { RoActivateInstance(s.handle(), &mut p_unk) })?;
        // SAFETY: the call succeeded, so the pointer is an object whose
        // reference this function owns.
        let unk = unsafe { ComPtr::<IUnknown>::from_raw(p_unk.cast()) }.ok_or(HResult::POINTER)?;
        unk.cast::<T>()
    }

    /// The activation factory of the runtime class of the name, as the
    /// interface.
    pub(crate) fn create_activation_factory<TFactory: Interface>(full_name: &str) -> Result<ComPtr<TFactory>, HResult> {
        let s = HStringInterop::new(Some(full_name))?;
        Self::ensure_ro_initialized()?;
        let guid = TFactory::IID;
        let mut p_unk = std::ptr::null_mut();
        // SAFETY: as `create_instance`; the identifier is a value of this
        // frame.
        check(unsafe { RoGetActivationFactory(s.handle(), &guid, &mut p_unk) })?;
        // SAFETY: as `create_instance`.
        let unk = unsafe { ComPtr::<IUnknown>::from_raw(p_unk.cast()) }.ok_or(HResult::POINTER)?;
        unk.cast::<TFactory>()
    }

    /// Initialises the Windows Runtime on the calling thread before its
    /// first activation, for the kind of apartment the thread has.
    ///
    /// The reference asks its runtime for the apartment state of the
    /// thread, which the runtime decides when the thread starts. A thread
    /// of the port has the apartment COM gave it; where COM is not
    /// initialised yet, the UI thread is single-threaded (OLE, which the
    /// platform initialises on it, needs that) and any other thread is not.
    fn ensure_ro_initialized() -> Result<(), HResult> {
        if INITIALIZED.get() {
            return Ok(());
        }
        let single_threaded = match co_get_apartment_type() {
            Some(apartment_type) => apartment_type == APTTYPE_STA || apartment_type == APTTYPE_MAINSTA,
            None => Dispatcher::ui_thread().check_access(),
        };
        let init_type =
            if single_threaded { RO_INIT_TYPE::RO_INIT_SINGLETHREADED } else { RO_INIT_TYPE::RO_INIT_MULTITHREADED };
        // SAFETY: takes a number.
        check(unsafe { RoInitialize(init_type as i32) })?;
        INITIALIZED.set(true);
        Ok(())
    }
}

/// A string handle of the Windows Runtime with its text; deletes the
/// handle when dropped if it owns it.
pub(crate) struct HStringInterop {
    s: isize,
    owns: bool,
}

impl HStringInterop {
    /// Creates a string handle for the text (the null handle for `None`)
    /// and owns it.
    pub(crate) fn new(s: Option<&str>) -> Result<HStringInterop, HResult> {
        let s = match s {
            None => 0,
            Some(s) => NativeWinRTMethods::windows_create_string(s)?,
        };
        Ok(HStringInterop { s, owns: true })
    }

    /// Wraps a string handle, owned or borrowed.
    pub(crate) fn from_handle(str: isize, owns: bool) -> HStringInterop {
        HStringInterop { s: str, owns }
    }

    /// The handle.
    pub(crate) fn handle(&self) -> isize {
        self.s
    }

    /// The text, or `None` for the null handle.
    pub(crate) fn value(&self) -> Option<String> {
        if self.s == 0 {
            return None;
        }

        Some(NativeWinRTMethods::windows_get_string_raw_buffer(self.s))
    }
}

impl Drop for HStringInterop {
    fn drop(&mut self) {
        if self.s != 0 && self.owns {
            NativeWinRTMethods::windows_delete_string(self.s);
            self.s = 0;
        }
    }
}
