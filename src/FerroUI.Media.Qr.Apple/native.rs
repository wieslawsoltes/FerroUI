use ferroui_media_apple::PixelBuffer;
use ferroui_media_qr::QrError;
use std::ffi::c_void;
extern "C" {
    fn ferroui_qr_vision_recognize(
        buffer: *mut c_void,
        output: *mut *mut c_void,
        length: *mut usize,
    ) -> i32;
    fn ferroui_qr_vision_free(pointer: *mut c_void);
}
pub fn recognize(buffer: &PixelBuffer) -> Result<Vec<serde_json::Value>, QrError> {
    let mut pointer = std::ptr::null_mut();
    let mut length = 0;
    // SAFETY: the borrowed pixel buffer remains retained by the analysis token
    // for this entire synchronous call. Native output is owned until freed.
    let code = unsafe { ferroui_qr_vision_recognize(buffer.as_raw(), &mut pointer, &mut length) };
    struct Output(*mut c_void);
    impl Drop for Output {
        fn drop(&mut self) {
            unsafe {
                ferroui_qr_vision_free(self.0);
            }
        }
    }
    let owned = Output(pointer);
    if code != 0 {
        return Err(if code == 2 {
            QrError::InputTooLarge
        } else {
            QrError::DecodeFailure
        });
    }
    if length == 0 || length > 1024 * 1024 || pointer.is_null() {
        return Err(QrError::DecodeFailure);
    }
    let bytes = unsafe { std::slice::from_raw_parts(owned.0.cast::<u8>(), length) };
    serde_json::from_slice(bytes).map_err(|_| QrError::DecodeFailure)
}
