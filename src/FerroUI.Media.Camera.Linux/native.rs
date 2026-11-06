use ferroui_media::{MediaError, MediaErrorKind};
use std::{
    ffi::{c_char, c_void, CStr},
    os::fd::{FromRawFd, OwnedFd},
    ptr::NonNull,
};
fn invalid(s: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::NativeFailure, s)
}
extern "C" {
    fn ferro_linux_portal_begin() -> *mut c_void;
    fn ferro_linux_portal_poll(
        r: *mut c_void,
        fd: *mut i32,
        node: *mut u32,
        error: *mut c_char,
        capacity: u32,
    ) -> i32;
    fn ferro_linux_portal_release(r: *mut c_void);
    fn ferro_linux_portal_present() -> i32;
}
pub struct PortalAccess(NonNull<c_void>);
impl PortalAccess {
    pub fn begin() -> Result<Self, MediaError> {
        NonNull::new(unsafe { ferro_linux_portal_begin() })
            .map(Self)
            .ok_or_else(|| invalid("Portal request is already active or allocation failed"))
    }
    pub fn poll(&self) -> std::task::Poll<Result<(OwnedFd, u32), MediaError>> {
        let mut fd = -1;
        let mut node = 0;
        let mut error = [0 as c_char; 512];
        match unsafe {
            ferro_linux_portal_poll(self.0.as_ptr(), &mut fd, &mut node, error.as_mut_ptr(), 512)
        } {
            0 => std::task::Poll::Pending,
            1 if fd >= 0 => std::task::Poll::Ready(Ok((unsafe { OwnedFd::from_raw_fd(fd) }, node))),
            kind => std::task::Poll::Ready(Err(MediaError::new(
                if kind == 2 {
                    MediaErrorKind::Permission
                } else if kind == 4 {
                    MediaErrorKind::Cancelled
                } else {
                    MediaErrorKind::NativeFailure
                },
                unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy(),
            ))),
        }
    }
    /// Metadata only; callers dispatch this bus query off the UI thread.
    pub fn present() -> bool {
        unsafe { ferro_linux_portal_present() != 0 }
    }
}
impl Drop for PortalAccess {
    fn drop(&mut self) {
        unsafe { ferro_linux_portal_release(self.0.as_ptr()) }
    }
}
