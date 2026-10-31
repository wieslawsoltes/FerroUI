use ferroui_media::{frames::GpuCompletion, MediaError, MediaErrorKind};
use ferroui_media_apple::PixelBuffer;
use std::{ffi::c_void, ptr::NonNull};
extern "C" {
    fn ferroui_media_metal_layout(w: u32, h: u32, stride: *mut u64, bytes: *mut u64);
    fn ferroui_media_metal_create(
        device: *mut c_void,
        queue: *mut c_void,
        w: u32,
        h: u32,
    ) -> *mut c_void;
    fn ferroui_media_metal_buffer(p: *mut c_void) -> *mut c_void;
    fn ferroui_media_metal_texture(p: *mut c_void) -> *mut c_void;
    fn ferroui_media_metal_commands(p: *mut c_void) -> *mut c_void;
    fn ferroui_media_metal_commit(
        p: *mut c_void,
        context: *mut c_void,
        finished: extern "C" fn(*mut c_void, i32),
    );
    fn ferroui_media_metal_release(p: *mut c_void);
    fn ferroui_media_metal_input(device: *mut c_void, buffer: *mut c_void) -> *mut c_void;
}
pub fn layout(w: u32, h: u32) -> (u64, u64) {
    let (mut stride, mut bytes) = (0, 0);
    unsafe { ferroui_media_metal_layout(w, h, &mut stride, &mut bytes) };
    (stride, bytes)
}
pub struct Output(NonNull<c_void>);
impl Output {
    pub unsafe fn create(
        device: *mut c_void,
        queue: *mut c_void,
        w: u32,
        h: u32,
    ) -> Result<Self, MediaError> {
        NonNull::new(unsafe { ferroui_media_metal_create(device, queue, w, h) })
            .map(Self)
            .ok_or_else(|| {
                MediaError::new(
                    MediaErrorKind::NativeFailure,
                    "Metal output allocation failed",
                )
            })
    }
    pub fn buffer(&self) -> PixelBuffer {
        unsafe {
            PixelBuffer::from_owned(ferroui_media_metal_buffer(self.0.as_ptr()))
                .expect("owned native pixel buffer")
        }
    }
    pub fn texture(&self) -> *mut c_void {
        unsafe { ferroui_media_metal_texture(self.0.as_ptr()) }
    }
    pub fn commands(&self) -> *mut c_void {
        unsafe { ferroui_media_metal_commands(self.0.as_ptr()) }
    }
    pub fn commit(&self, completion: GpuCompletion) {
        extern "C" fn finished(p: *mut c_void, success: i32) {
            let completion = unsafe { Box::from_raw(p.cast::<GpuCompletion>()) };
            if success != 0 {
                completion.complete();
            } else {
                completion.mark_indeterminate();
            }
        }
        unsafe {
            ferroui_media_metal_commit(
                self.0.as_ptr(),
                Box::into_raw(Box::new(completion)).cast(),
                finished,
            )
        };
    }
}
impl Drop for Output {
    fn drop(&mut self) {
        unsafe { ferroui_media_metal_release(self.0.as_ptr()) }
    }
}
pub struct Texture(NonNull<c_void>);
impl Texture {
    pub unsafe fn new(device: *mut c_void, buffer: &PixelBuffer) -> Option<Self> {
        NonNull::new(unsafe { ferroui_media_metal_input(device, buffer.as_raw()) }).map(Self)
    }
    pub fn handle(&self) -> *mut c_void {
        self.0.as_ptr()
    }
}
impl Drop for Texture {
    fn drop(&mut self) {
        unsafe { ferroui_media_metal_release(self.0.as_ptr()) }
    }
}
