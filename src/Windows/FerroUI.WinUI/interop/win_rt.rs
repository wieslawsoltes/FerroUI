//! What the integration needs around the bindings to call the Windows
//! Runtime: string handles, activation, the `boolean` of the ABI, and a
//! pointer that may be handed to another thread. The reference has none of
//! this: its runtime projects the classes, strings and booleans.

use ferroui_microcom::{ComPtr, HResult, Interface};

/// A value of the ABI's `boolean` (one byte, zero or one).
pub(crate) fn to_abi(value: bool) -> u8 {
    u8::from(value)
}

/// The value of a `boolean` of the ABI: anything but zero is true.
pub(crate) fn from_abi(value: u8) -> bool {
    value != 0
}

/// The `boolean` result of a call, false on a failure.
pub(crate) fn flag(result: Result<u8, HResult>) -> bool {
    result.map(from_abi).unwrap_or(false)
}

/// A counted reference to an object of the Windows Runtime that is agile
/// (a `DispatcherQueue`, an asynchronous operation of the system): such an
/// object may be called, and released, on any thread, so the reference may
/// be moved to and shared with another thread.
pub(crate) struct AgileComPtr<T: Interface>(ComPtr<T>);

// SAFETY: the constructor's contract: the object is agile, so its methods
// and its reference count may be used from any thread.
unsafe impl<T: Interface> Send for AgileComPtr<T> {}
// SAFETY: as above.
unsafe impl<T: Interface> Sync for AgileComPtr<T> {}

impl<T: Interface> AgileComPtr<T> {
    /// # Safety
    /// The object must be agile: callable and releasable on any thread.
    pub(crate) unsafe fn new(pointer: ComPtr<T>) -> AgileComPtr<T> {
        AgileComPtr(pointer)
    }

    pub(crate) fn get(&self) -> &ComPtr<T> {
        &self.0
    }
}

#[cfg(windows)]
pub(crate) use imp::*;

#[cfg(windows)]
mod imp {
    use ferroui_microcom::{ComPtr, HResult, Interface};
    use ferroui_win32::win_rt::{HStringInterop, NativeWinRTMethods};

    /// A string handle with the text, deleted when it is dropped: what a
    /// method that takes a string is given.
    pub(crate) fn hstring(text: &str) -> Result<HStringInterop, HResult> {
        HStringInterop::new(Some(text))
    }

    /// The text of a string handle a method returned, whose ownership is the
    /// caller's: the handle is deleted.
    pub(crate) fn take_string(handle: isize) -> String {
        HStringInterop::from_handle(handle, true).value().unwrap_or_default()
    }

    /// Takes an owned HSTRING, rejecting malformed UTF-16 rather than changing
    /// an exact native edit into a replacement-character edit. Ownership is
    /// released on both success and failure.
    pub(crate) fn take_string_exact(handle: isize) -> Result<String, HResult> {
        HStringInterop::from_handle(handle, true)
            .try_value()
            .map(Option::unwrap_or_default)
    }

    /// The text of a string handle a method of an object of the integration
    /// is given: the caller's, which is not deleted; none for the null
    /// handle (the null string of C#).
    pub(crate) fn borrowed_string(handle: isize) -> Option<String> {
        if handle == 0 {
            return None;
        }
        HStringInterop::from_handle(handle, false).value()
    }

    /// A new string handle with the text, for a method of an object of the
    /// integration that returns a string (the caller deletes it).
    pub(crate) fn new_string(text: &str) -> Result<isize, HResult> {
        if text.is_empty() {
            return Ok(0);
        }
        NativeWinRTMethods::windows_create_string(text)
    }

    /// The activation factory, or the interface of the static members, of
    /// the runtime class of the name.
    pub(crate) fn factory<T: Interface>(class_name: &str) -> Result<ComPtr<T>, HResult> {
        NativeWinRTMethods::create_activation_factory::<T>(class_name)
    }

    /// A new object of the runtime class of the name (its constructor
    /// without arguments), as the interface.
    pub(crate) fn activate<T: Interface>(class_name: &str) -> Result<ComPtr<T>, HResult> {
        NativeWinRTMethods::create_instance::<T>(class_name)
    }

    /// What an object of the integration answers to `IInspectable::GetIids`:
    /// the identifiers of its interfaces, in memory of the COM allocator,
    /// which the caller frees.
    ///
    /// # Safety
    /// `iid_count` must be the out parameter of the count of the call:
    /// valid for a write.
    pub(crate) unsafe fn iids(interfaces: &[ferroui_microcom::Guid], iid_count: *mut u32) -> Result<*mut ferroui_microcom::Guid, HResult> {
        use windows_sys::Win32::System::Com::CoTaskMemAlloc;
        let size = std::mem::size_of_val(interfaces);
        // SAFETY: an allocation of the size of the identifiers.
        let memory = unsafe { CoTaskMemAlloc(size.max(1)) }.cast::<ferroui_microcom::Guid>();
        if memory.is_null() {
            return Err(HResult(0x8007_000E)); // E_OUTOFMEMORY
        }
        // SAFETY: the block has room for every identifier; the count is
        // valid for a write by the contract of this function.
        unsafe {
            for (index, interface) in interfaces.iter().enumerate() {
                memory.add(index).write(*interface);
            }
            iid_count.write(interfaces.len() as u32);
        }
        Ok(memory)
    }

    /// What an object of the integration answers to
    /// `IInspectable::GetRuntimeClassName`.
    pub(crate) fn runtime_class_name(name: &str) -> Result<isize, HResult> {
        NativeWinRTMethods::windows_create_string(name)
    }

    /// `Microsoft.UI.Win32Interop.GetWindowFromWindowId`: the window handle
    /// of a window identifier of WinUI, zero when there is none.
    ///
    /// The projection of the reference calls the export of the framework
    /// library of the Windows App SDK, as `Microsoft.UI.Interop.h` does: the
    /// library is loaded (from the package graph of the process, which the
    /// bootstrap has set up) at the first call.
    pub(crate) fn get_window_from_window_id(window_id: super::super::WindowId) -> isize {
        use std::sync::OnceLock;
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};

        type GetWindowFromWindowId = unsafe extern "system" fn(super::super::WindowId, *mut isize) -> i32;
        static FUNCTION: OnceLock<Option<GetWindowFromWindowId>> = OnceLock::new();
        let function = *FUNCTION.get_or_init(|| {
            let name: Vec<u16> = "Microsoft.Internal.FrameworkUdk.dll".encode_utf16().chain(Some(0)).collect();
            // SAFETY: a terminated name; the module, once loaded, stays
            // loaded for the process, as the header of the SDK expects.
            unsafe {
                let mut module = GetModuleHandleW(name.as_ptr());
                if module.is_null() {
                    module = LoadLibraryW(name.as_ptr());
                }
                if module.is_null() {
                    return None;
                }
                let address = GetProcAddress(module, c"Windowing_GetWindowFromWindowId".as_ptr().cast())?;
                Some(std::mem::transmute::<unsafe extern "system" fn() -> isize, GetWindowFromWindowId>(address))
            }
        });
        let Some(function) = function else {
            return 0;
        };
        let mut hwnd = 0isize;
        // SAFETY: the export has this signature; the handle is written to a
        // number of this frame.
        let result = unsafe { function(window_id, &mut hwnd) };
        if result < 0 {
            0
        } else {
            hwnd
        }
    }

    /// `ClientToScreen` of the origin of the client area of a window: where
    /// it is on the screen, none when the system refuses.
    pub(crate) fn client_origin_on_screen(hwnd: isize) -> Option<(i32, i32)> {
        use windows_sys::Win32::Foundation::POINT;
        use windows_sys::Win32::Graphics::Gdi::ClientToScreen;

        let mut pt = POINT { x: 0, y: 0 };
        // SAFETY: a window handle (the system checks it) and a point of this
        // frame.
        if unsafe { ClientToScreen(hwnd as _, &mut pt) } != 0 {
            Some((pt.x, pt.y))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_boolean_of_the_abi() {
        assert_eq!(1, to_abi(true));
        assert_eq!(0, to_abi(false));
        assert!(from_abi(1));
        assert!(from_abi(0xFF));
        assert!(!from_abi(0));
        assert!(flag(Ok(1)));
        assert!(!flag(Err(HResult(0x8000_4005))));
    }
}
