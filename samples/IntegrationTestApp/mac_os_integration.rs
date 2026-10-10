//! Port of `MacOSIntegration.cs`: what the sample asks of AppKit directly.

#[cfg(target_os = "macos")]
use ferroui_controls::platform::IPlatformHandle;
#[cfg(target_os = "macos")]
use ferroui_controls::Window;

/// The calls into the Objective-C runtime of the managed original (its two imported
/// functions of `/usr/lib/libobjc.dylib`).
#[cfg(target_os = "macos")]
mod objc {
    use std::ffi::{c_char, c_void};

    #[link(name = "objc")]
    extern "C" {
        /// `GetHandle`: the entry point `sel_registerName`.
        pub fn sel_registerName(name: *const c_char) -> *mut c_void;
        /// `Int64_objc_msgSend`: the entry point `objc_msgSend`, called through a pointer of
        /// the signature of the method it dispatches to.
        pub fn objc_msgSend();
    }
}

pub struct MacOSIntegration;

impl MacOSIntegration {
    /// The position of the native window of `window` in the front-to-back order of the windows
    /// of the application (`[NSWindow orderedIndex]`).
    ///
    /// # Panics
    /// Panics if the window has no platform implementation or no native window (a null
    /// reference in the managed original).
    #[cfg(target_os = "macos")]
    pub fn get_ordered_index(window: &Window) -> i64 {
        use std::ffi::c_void;

        // `window.PlatformImpl!.Handle!.Handle`.
        let handle = window.try_get_platform_handle().expect("the window has a platform implementation with a handle");
        // The handle of a window of the macOS backend is its `NSWindow`; a handle of another
        // kind is not an object the message can be sent to.
        assert_eq!(handle.handle_descriptor(), Some("NSWindow"), "the handle of the window is a native window");
        let receiver = handle.handle() as *mut c_void;
        // SAFETY: the selector name is NUL-terminated; the receiver is the `NSWindow` of a
        // window that is alive (checked above), which responds to `orderedIndex` and returns
        // an `NSInteger`; `objc_msgSend` is called through a pointer of that signature.
        unsafe {
            let selector = objc::sel_registerName(c"orderedIndex".as_ptr());
            let send: unsafe extern "C" fn(*mut c_void, *mut c_void) -> i64 =
                std::mem::transmute(objc::objc_msgSend as unsafe extern "C" fn());
            send(receiver, selector)
        }
    }
}
