//! The functions of the Windows Runtime: string handles, activation, the
//! activation factory of the composition library and the dispatcher queue
//! controller.

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

/// The apartment of the thread of a dispatcher queue.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms, dead_code)]
pub(crate) enum DISPATCHERQUEUE_THREAD_APARTMENTTYPE {
    DQTAT_COM_NONE = 0,
    DQTAT_COM_ASTA = 1,
    DQTAT_COM_STA = 2,
}

/// The thread of a dispatcher queue: one the system starts, or the caller's.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types, clippy::upper_case_acronyms, dead_code)]
pub(crate) enum DISPATCHERQUEUE_THREAD_TYPE {
    DQTYPE_THREAD_DEDICATED = 1,
    DQTYPE_THREAD_CURRENT = 2,
}

/// The options of `CreateDispatcherQueueController`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Created by the composition connection, the next part of stage 2c, and by the tests.
pub(crate) struct DispatcherQueueOptions {
    pub dw_size: i32,
    pub thread_type: DISPATCHERQUEUE_THREAD_TYPE,
    pub apartment_type: DISPATCHERQUEUE_THREAD_APARTMENTTYPE,
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

    /// The activation factory of a class of the composition library
    /// (`DllGetActivationFactory` of `Windows.UI.Composition.dll`, which
    /// activates without the registration of the class).
    ///
    /// The library is loaded when this is first called, as an import of
    /// the reference is: a system without it fails here with a result
    /// code, not when the process starts.
    #[allow(dead_code)] // Called by the composition connection, the next part of stage 2c, and by the tests.
    pub(crate) fn get_windows_ui_composition_activation_factory(
        class_name: &str,
    ) -> Result<ComPtr<super::IActivationFactory>, HResult> {
        use crate::interop::unmanaged_methods::{get_proc_address, load_library};

        //"Windows.UI.Composition.Compositor"
        let s = HStringInterop::new(Some(class_name))?;
        let function = get_proc_address(load_library("Windows.UI.Composition.dll"), c"DllGetActivationFactory")
            .ok_or(HResult::NOTIMPL)?;
        // SAFETY: the export has this signature: a string handle and the
        // place of the factory.
        let function: unsafe extern "system" fn(isize, *mut *mut c_void) -> i32 =
            unsafe { std::mem::transmute(function) };
        let mut factory = std::ptr::null_mut();
        // SAFETY: a string handle that lives through the call, and a
        // pointer of this frame the library writes the factory to.
        check(unsafe { function(s.handle(), &mut factory) })?;
        // SAFETY: the call succeeded, so the pointer is the factory, whose
        // reference this function owns.
        unsafe { ComPtr::<super::IActivationFactory>::from_raw(factory.cast()) }.ok_or(HResult::POINTER)
    }

    /// `CreateDispatcherQueueController` of `coremessaging.dll`: a
    /// dispatcher queue for the calling thread or on a thread of its own.
    /// The library is loaded when this is first called (Windows 10 1709
    /// and later have it).
    #[allow(dead_code)] // As above.
    pub(crate) fn create_dispatcher_queue_controller(
        options: DispatcherQueueOptions,
    ) -> Result<ComPtr<super::IDispatcherQueueController>, HResult> {
        use crate::interop::unmanaged_methods::{get_proc_address, load_library};

        let function = get_proc_address(load_library("coremessaging.dll"), c"CreateDispatcherQueueController")
            .ok_or(HResult::NOTIMPL)?;
        // SAFETY: the export has this signature: the options by value and
        // the place of the controller.
        let function: unsafe extern "system" fn(DispatcherQueueOptions, *mut *mut c_void) -> i32 =
            unsafe { std::mem::transmute(function) };
        let mut controller = std::ptr::null_mut();
        // SAFETY: plain values, and a pointer of this frame the system
        // writes the controller to.
        check(unsafe { function(options, &mut controller) })?;
        // SAFETY: the call succeeded, so the pointer is the controller,
        // whose reference this function owns.
        unsafe { ComPtr::<super::IDispatcherQueueController>::from_raw(controller.cast()) }.ok_or(HResult::POINTER)
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
