//! Native ownership is confined here. Players/seeks are main-thread objects;
//! immutable CoreVideo buffer owners can retire from a GPU completion thread.
use crate::geometry::Geometry;
use ferroui_media::{MediaError, MediaErrorKind};
use ferroui_media_apple::PixelBuffer;
use std::{
    ffi::{c_char, c_void, CStr, CString},
    marker::PhantomData,
    ptr::NonNull,
    rc::Rc,
};
#[derive(Default)]
#[repr(C)]
pub struct State {
    pub position: f64,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub state: i32,
    pub seekable: i32,
}
extern "C" {
    fn ferroui_portable_player_create(source: *const c_char) -> *mut c_void;
    fn ferroui_portable_player_release(p: *mut c_void);
    fn ferroui_portable_player_state(p: *mut c_void, state: *mut State);
    fn ferroui_portable_player_error(p: *mut c_void, out: *mut c_char, len: usize);
    fn ferroui_portable_player_play(p: *mut c_void);
    fn ferroui_portable_player_pause(p: *mut c_void);
    fn ferroui_portable_player_audio(p: *mut c_void, volume: f64, muted: i32);
    fn ferroui_portable_player_seek(p: *mut c_void, seconds: f64) -> *mut c_void;
    fn ferroui_portable_seek_state(p: *mut c_void, position: *mut f64) -> i32;
    fn ferroui_portable_seek_release(p: *mut c_void);
    fn ferroui_portable_player_frame(p: *mut c_void, timestamp: *mut f64) -> *mut c_void;
    fn ferroui_portable_player_geometry(
        p: *mut c_void,
        pixels: *mut c_void,
        out: *mut Geometry,
    ) -> i32;
}
pub struct Player {
    handle: NonNull<c_void>,
    _thread: PhantomData<Rc<()>>,
}
impl Player {
    pub fn new(source: &str) -> Result<Self, MediaError> {
        let source = CString::new(source).map_err(|_| {
            MediaError::new(MediaErrorKind::InvalidSource, "Media source contains NUL")
        })?;
        // SAFETY: native copies UTF-8 and returns one owned main-thread object.
        let handle = NonNull::new(unsafe { ferroui_portable_player_create(source.as_ptr()) })
            .ok_or_else(|| {
                MediaError::new(
                    MediaErrorKind::NativeFailure,
                    "Cannot create AVPlayer; a valid source and main thread are required",
                )
            })?;
        Ok(Self {
            handle,
            _thread: PhantomData,
        })
    }
    pub fn state(&self) -> State {
        let mut out = State::default();
        unsafe {
            ferroui_portable_player_state(self.handle.as_ptr(), &mut out);
        }
        out
    }
    pub fn error(&self) -> MediaError {
        let mut text = [0 as c_char; 1024];
        unsafe {
            ferroui_portable_player_error(self.handle.as_ptr(), text.as_mut_ptr(), text.len());
        }
        MediaError::new(
            MediaErrorKind::NativeFailure,
            unsafe { CStr::from_ptr(text.as_ptr()) }.to_string_lossy(),
        )
    }
    pub fn play(&self) {
        unsafe {
            ferroui_portable_player_play(self.handle.as_ptr());
        }
    }
    pub fn pause(&self) {
        unsafe {
            ferroui_portable_player_pause(self.handle.as_ptr());
        }
    }
    pub fn audio(&self, volume: f64, muted: bool) {
        unsafe {
            ferroui_portable_player_audio(self.handle.as_ptr(), volume, muted as i32);
        }
    }
    pub fn seek(&self, seconds: f64) -> Result<Seek, MediaError> {
        let handle =
            NonNull::new(unsafe { ferroui_portable_player_seek(self.handle.as_ptr(), seconds) })
                .ok_or_else(|| {
                    MediaError::new(MediaErrorKind::NativeFailure, "Cannot create seek request")
                })?;
        Ok(Seek {
            handle,
            _thread: PhantomData,
        })
    }
    pub fn frame(&self) -> Option<(PixelBuffer, f64)> {
        let mut timestamp = f64::NAN;
        unsafe {
            PixelBuffer::from_owned(ferroui_portable_player_frame(
                self.handle.as_ptr(),
                &mut timestamp,
            ))
        }
        .map(|buffer| (buffer, timestamp))
    }
    pub fn geometry(&self, buffer: &PixelBuffer) -> Result<Geometry, MediaError> {
        let mut geometry = Geometry::default();
        // Both handles remain owned throughout this synchronous metadata query.
        if unsafe {
            ferroui_portable_player_geometry(self.handle.as_ptr(), buffer.as_raw(), &mut geometry)
        } == 0
        {
            return Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "Video geometry is not ready",
            ));
        }
        Ok(geometry)
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        unsafe {
            ferroui_portable_player_release(self.handle.as_ptr());
        }
    }
}
pub struct Seek {
    handle: NonNull<c_void>,
    _thread: PhantomData<Rc<()>>,
}
impl Seek {
    pub fn poll(&self) -> Option<Result<f64, MediaError>> {
        let mut position = 0.;
        let result = unsafe { ferroui_portable_seek_state(self.handle.as_ptr(), &mut position) };
        match result {
            0 => None,
            1 => Some(Ok(position)),
            _ => Some(Err(MediaError::cancelled())),
        }
    }
}
impl Drop for Seek {
    fn drop(&mut self) {
        unsafe {
            ferroui_portable_seek_release(self.handle.as_ptr());
        }
    }
}
