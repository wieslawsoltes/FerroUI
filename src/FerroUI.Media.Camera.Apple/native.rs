//! Narrow, ownership-checked camera FFI. No pixel mapping or preview copy API.
use ferroui_media_apple::PixelBuffer;
use ferroui_media_camera::{CameraError, CameraErrorKind, CameraPermission, CameraSelection};
use std::{
    ffi::{CStr, CString, c_char, c_void},
    marker::PhantomData,
    ptr::NonNull,
    rc::Rc,
};
extern "C" {
    fn ferroui_portable_camera_permission() -> i32;
    fn ferroui_portable_camera_devices() -> *mut c_char;
    fn ferroui_portable_camera_free(p: *mut c_void);
    fn ferroui_portable_camera_start(
        id: *const c_char,
        facing: i32,
        width: u32,
        height: u32,
        fps: f64,
        exact: i32,
    ) -> *mut c_void;
    fn ferroui_portable_camera_stop(p: *mut c_void);
    fn ferroui_portable_camera_release(p: *mut c_void);
    fn ferroui_portable_camera_status(p: *mut c_void, error: *mut c_char, capacity: usize) -> i32;
    fn ferroui_portable_camera_settings(p: *mut c_void) -> *mut c_char;
    fn ferroui_portable_camera_frame(p: *mut c_void, timestamp: *mut f64) -> *mut c_void;
    fn ferroui_portable_camera_photo(p: *mut c_void) -> *mut c_void;
    fn ferroui_portable_photo_state(p: *mut c_void, info: *mut PhotoInfo) -> i32;
    fn ferroui_portable_photo_copy(p: *mut c_void, bytes: *mut u8, capacity: usize) -> i32;
    fn ferroui_portable_photo_release(p: *mut c_void);
}
fn failed(kind: CameraErrorKind, text: &str) -> CameraError {
    CameraError::new(kind, text)
}
// SAFETY: shim returns a malloc-owned NUL-terminated UTF-8 serialization.
unsafe fn json(p: *mut c_char) -> Result<serde_json::Value, CameraError> {
    if p.is_null() {
        return Err(failed(
            CameraErrorKind::NativeFailure,
            "Native camera metadata is unavailable",
        ));
    }
    let parsed = serde_json::from_slice(CStr::from_ptr(p).to_bytes());
    ferroui_portable_camera_free(p.cast());
    parsed.map_err(|_| {
        failed(
            CameraErrorKind::NativeFailure,
            "Invalid native camera metadata",
        )
    })
}
pub fn devices() -> Result<serde_json::Value, CameraError> {
    unsafe { json(ferroui_portable_camera_devices()) }
}
pub fn permission() -> CameraPermission {
    match unsafe { ferroui_portable_camera_permission() } {
        1 => CameraPermission::Granted,
        2 => CameraPermission::Denied,
        3 => CameraPermission::Restricted,
        _ => CameraPermission::NotDetermined,
    }
}
pub struct Camera {
    handle: NonNull<c_void>,
    _thread: PhantomData<Rc<()>>,
}
impl Camera {
    pub fn new(selection: &CameraSelection) -> Result<Self, CameraError> {
        use ferroui_media_camera::CameraFacing;
        let id = CString::new(selection.device_id.as_deref().unwrap_or_default())
            .map_err(|_| failed(CameraErrorKind::InvalidRequest, "Camera id contains NUL"))?;
        let facing = match selection.preferred_facing {
            Some(CameraFacing::Front) => 1,
            Some(CameraFacing::Rear) => 2,
            Some(CameraFacing::External) => 3,
            _ => 0,
        };
        let (w, h, fps) = selection
            .format
            .map(|f| (f.width, f.height, f.frames_per_second))
            .unwrap_or_default();
        let handle = NonNull::new(unsafe {
            ferroui_portable_camera_start(id.as_ptr(), facing, w, h, fps, selection.exact as i32)
        })
        .ok_or_else(|| {
            failed(
                CameraErrorKind::NativeFailure,
                "Camera start requires the main thread",
            )
        })?;
        Ok(Self {
            handle,
            _thread: PhantomData,
        })
    }
    pub fn status(&self) -> Result<bool, CameraError> {
        let mut error = [0 as c_char; 1024];
        let status = unsafe {
            ferroui_portable_camera_status(self.handle.as_ptr(), error.as_mut_ptr(), error.len())
        };
        if status >= 0 {
            return Ok(status == 1);
        }
        let kind = match status {
            -1 => CameraErrorKind::Denied,
            -2 => CameraErrorKind::NoDevice,
            -3 => CameraErrorKind::Disconnected,
            -4 => CameraErrorKind::Unsupported,
            -6 => CameraErrorKind::Cancelled,
            -7 => CameraErrorKind::Restricted,
            _ => CameraErrorKind::NativeFailure,
        };
        Err(CameraError::new(
            kind,
            unsafe { CStr::from_ptr(error.as_ptr()) }.to_string_lossy(),
        ))
    }
    pub fn settings(&self) -> Result<serde_json::Value, CameraError> {
        unsafe { json(ferroui_portable_camera_settings(self.handle.as_ptr())) }
    }
    pub fn frame(&self) -> Option<(PixelBuffer, f64)> {
        let mut timestamp = 0.;
        unsafe {
            PixelBuffer::from_owned(ferroui_portable_camera_frame(
                self.handle.as_ptr(),
                &mut timestamp,
            ))
        }
        .map(|buffer| (buffer, timestamp))
    }
    pub fn photo(&self) -> Result<Photo, CameraError> {
        let handle = NonNull::new(unsafe { ferroui_portable_camera_photo(self.handle.as_ptr()) })
            .ok_or_else(|| {
            failed(
                CameraErrorKind::Busy,
                "Native JPEG capture is unavailable or a photo is still in flight",
            )
        })?;
        Ok(Photo {
            handle,
            _thread: PhantomData,
        })
    }
    pub fn stop(&self) {
        unsafe {
            ferroui_portable_camera_stop(self.handle.as_ptr());
        }
    }
}
impl Drop for Camera {
    fn drop(&mut self) {
        unsafe {
            ferroui_portable_camera_release(self.handle.as_ptr());
        }
    }
}
#[repr(C)]
#[derive(Default)]
pub struct PhotoInfo {
    pub length: u64,
    pub width: u32,
    pub height: u32,
    pub orientation: u32,
    pub timestamp: f64,
}
pub struct Photo {
    handle: NonNull<c_void>,
    _thread: PhantomData<Rc<()>>,
}
impl Photo {
    pub fn poll(&self) -> Option<Result<PhotoInfo, CameraError>> {
        let mut info = PhotoInfo::default();
        match unsafe { ferroui_portable_photo_state(self.handle.as_ptr(), &mut info) } {
            0 => None,
            1 => Some(Ok(info)),
            -2 => Some(Err(failed(
                CameraErrorKind::Cancelled,
                "Photo capture was canceled",
            ))),
            _ => Some(Err(failed(
                CameraErrorKind::NativeFailure,
                "Native JPEG capture failed",
            ))),
        }
    }
    pub fn copy(&self, info: &PhotoInfo) -> Result<Vec<u8>, CameraError> {
        if info.length == 0 || info.length > 64 * 1024 * 1024 || info.width == 0 || info.height == 0
        {
            return Err(failed(
                CameraErrorKind::NativeFailure,
                "Invalid or oversized native JPEG",
            ));
        }
        let mut bytes = vec![0; info.length as usize];
        if unsafe {
            ferroui_portable_photo_copy(self.handle.as_ptr(), bytes.as_mut_ptr(), bytes.len())
        } != 1
        {
            return Err(failed(
                CameraErrorKind::Cancelled,
                "Photo capture was canceled",
            ));
        }
        Ok(bytes)
    }
}
impl Drop for Photo {
    fn drop(&mut self) {
        unsafe {
            ferroui_portable_photo_release(self.handle.as_ptr());
        }
    }
}

#[cfg(all(test, feature = "native-lifetime-tests"))]
mod tests {
    use super::*;
    extern "C" {
        fn ferroui_camera_lifetime_fixture() -> *mut c_void;
        fn ferroui_camera_lifetime_result() -> i32;
        fn ferroui_camera_frame_duration_contract() -> i32;
    }
    #[test]
    fn native_frame_duration_keeps_exact_supported_boundaries() {
        assert_eq!(unsafe { ferroui_camera_frame_duration_contract() }, 1);
    }
    #[test]
    fn native_stop_releases_the_retaining_delegate_and_session_off_ui() {
        let owner = unsafe { ferroui_camera_lifetime_fixture() };
        assert!(!owner.is_null());
        unsafe {
            ferroui_portable_camera_release(owner);
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while unsafe { ferroui_camera_lifetime_result() } == 0
            && std::time::Instant::now() < deadline
        {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(unsafe { ferroui_camera_lifetime_result() }, 1);
    }
}
