use std::{ffi::c_void, ptr::NonNull};
extern "C" {
    fn ferroui_portable_buffer_info(
        p: *mut c_void,
        width: *mut u32,
        height: *mut u32,
        stride: *mut u64,
        bytes: *mut u64,
        color: *mut i32,
    );
    fn ferroui_portable_buffer_surface(p: *mut c_void) -> *mut c_void;
    fn ferroui_portable_buffer_release(p: *mut c_void);
}
pub struct PixelBuffer(NonNull<c_void>);
// SAFETY: acquired immutable retained CVPixelBuffer. No pixel mutation/mapping
// is exposed, and CoreVideo retain/release is legal from completion threads.
unsafe impl Send for PixelBuffer {}
unsafe impl Sync for PixelBuffer {}
impl PixelBuffer {
    /// Borrowed CoreVideo handle, valid only while this buffer remains leased.
    /// Native clients must retain it for asynchronous use and never mutate it.
    pub fn as_raw(&self) -> *mut c_void {
        self.0.as_ptr()
    }
    /// Adopts an immutable CoreVideo reference from a native producer.
    ///
    /// # Safety
    /// The non-null pointer must be a valid +1 CVPixelBuffer reference. Its
    /// producer must not mutate/reuse pixels while any owner or lease exists.
    pub unsafe fn from_owned(handle: *mut c_void) -> Option<Self> {
        NonNull::new(handle).map(Self)
    }
    pub fn info(&self) -> (u32, u32, u64, u64, i32) {
        let (mut w, mut h, mut stride, mut bytes, mut color) = (0, 0, 0, 0, 0);
        unsafe {
            ferroui_portable_buffer_info(
                self.0.as_ptr(),
                &mut w,
                &mut h,
                &mut stride,
                &mut bytes,
                &mut color,
            );
        }
        (w, h, stride, bytes, color)
    }
    pub fn surface(&self) -> Option<usize> {
        NonNull::new(unsafe { ferroui_portable_buffer_surface(self.0.as_ptr()) })
            .map(|p| p.as_ptr() as usize)
    }
}
impl Drop for PixelBuffer {
    fn drop(&mut self) {
        unsafe {
            ferroui_portable_buffer_release(self.0.as_ptr());
        }
    }
}
