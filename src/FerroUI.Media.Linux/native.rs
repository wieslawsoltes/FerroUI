use ferroui_media::{frames::*, *};

#[cfg(all(test, feature = "native-frame-tests"))]
mod native_frame_tests {
    extern "C" {
        fn ferro_linux_test_frame_contracts() -> i32;
        fn ferro_linux_test_recording() -> i32;
    }
    #[test]
    fn native_descriptor_extents_and_ownership() {
        assert_eq!(unsafe { ferro_linux_test_frame_contracts() }, 0);
    }

    #[test]
    fn native_camera_sample_records_decodable_silent_mp4() {
        let result = unsafe { ferro_linux_test_recording() };
        if result < 0 {
            eprintln!("Linux native recording fixture skipped without GStreamer recording plugins");
        } else {
            assert_eq!(result, 0);
        }
    }
}
use ferroui_media_opengl::{NativeGlFrameProducer, NativeTextureView, ReadyGlFrame};
use ferroui_opengl::IGlContext;
use std::{
    ffi::{c_char, c_void, CStr, CString},
    ptr::NonNull,
    rc::Rc,
    sync::Arc,
    task::Poll,
    time::Duration,
};
#[repr(C)]
#[derive(Clone, Copy)]
pub struct NativeState {
    pub state: i32,
    pub seekable: i32,
    pub buffering: i32,
    pub error_kind: i32,
    pub width: u32,
    pub height: u32,
    pub fps_n: u32,
    pub fps_d: u32,
    pub position_ns: u64,
    pub duration_ns: u64,
    pub seek_done: u64,
    error: [c_char; 512],
}
impl NativeState {
    pub fn error(&self) -> MediaError {
        MediaError::new(
            match self.error_kind {
                1 => MediaErrorKind::Unsupported,
                3 => MediaErrorKind::DeviceLost,
                _ => MediaErrorKind::NativeFailure,
            },
            unsafe { CStr::from_ptr(self.error.as_ptr()) }.to_string_lossy(),
        )
    }
}
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FrameInfo {
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub color: u32,
    pub plane_count: u32,
    strides: [u32; 4],
    offsets: [u32; 4],
    fds: [i32; 4],
    pub modifier: u64,
    pub bytes: u64,
    pub timestamp_ns: u64,
    pub sequence: u64,
    pub epoch: u64,
}
extern "C" {
    fn ferro_linux_open(
        source: *const c_char,
        caps: *const c_char,
        profile: i32,
        fd: i32,
        node: u32,
        w: u32,
        h: u32,
        n: u32,
        d: u32,
    ) -> *mut c_void;
    fn ferro_linux_close(s: *mut c_void);
    fn ferro_linux_state(s: *mut c_void, state: *mut NativeState);
    fn ferro_linux_intent(s: *mut c_void, intent: i32);
    fn ferro_linux_seek(s: *mut c_void, ns: u64) -> u64;
    fn ferro_linux_audio(s: *mut c_void, volume: f64, muted: i32);
    fn ferro_linux_take(s: *mut c_void) -> *mut c_void;
    fn ferro_linux_frame_info(f: *mut c_void, info: *mut FrameInfo);
    fn ferro_linux_frame_release(f: *mut c_void);
    fn ferro_linux_alias(f: *mut c_void, display: isize, texture: *mut u32) -> *mut c_void;
    fn ferro_linux_alias_release(alias: *mut c_void, alive: i32);
    fn ferro_linux_convert(f: *mut c_void, display: isize, output: u32) -> i32;
    fn ferro_linux_photo_rgb(f: *mut c_void, rgb: *mut u8, capacity: u64) -> i32;
    fn ferro_linux_can_record(s: *mut c_void) -> i32;
    fn ferro_linux_record(s: *mut c_void) -> *mut c_void;
    fn ferro_linux_record_finish(r: *mut c_void);
    fn ferro_linux_record_cancel(r: *mut c_void);
    fn ferro_linux_record_release(r: *mut c_void);
    fn ferro_linux_record_poll(r: *mut c_void, out: *mut RecordingInfo) -> i32;
    fn ferro_linux_record_copy(r: *mut c_void, bytes: *mut u8, length: u64) -> i32;

}
struct Session(NonNull<c_void>);
// C session methods serialize through their mutex. Native pipeline calls belong
// to the worker, and close signals it without joining the UI thread.
unsafe impl Send for Session {}
unsafe impl Sync for Session {}
impl Drop for Session {
    fn drop(&mut self) {
        unsafe { ferro_linux_close(self.0.as_ptr()) }
    }
}
#[derive(Clone)]
pub struct NativeSession(Arc<Session>);
impl NativeSession {
    pub fn supports_recording(&self) -> bool {
        unsafe { ferro_linux_can_record(self.0 .0.as_ptr()) != 0 }
    }

    pub fn record(&self) -> Result<NativeRecording, MediaError> {
        if !self.supports_recording() {
            return Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "Native H.264 recording needs a supported linear camera format and GStreamer plugins",
            ));
        }
        NonNull::new(unsafe { ferro_linux_record(self.0 .0.as_ptr()) })
            .map(NativeRecording)
            .ok_or_else(|| invalid("The native recording worker is unavailable"))
    }

    pub fn open(
        source: &str,
        caps: &str,
        profile: i32,
        fd: i32,
        node: u32,
    ) -> Result<Self, MediaError> {
        let source = CString::new(source).map_err(|_| invalid("NUL in source"))?;
        let caps = CString::new(caps).map_err(|_| invalid("NUL in caps"))?;
        NonNull::new(unsafe {
            ferro_linux_open(
                source.as_ptr(),
                caps.as_ptr(),
                profile,
                fd,
                node,
                0,
                0,
                0,
                1,
            )
        })
        .map(|p| Self(Arc::new(Session(p))))
        .ok_or_else(|| invalid("Native session allocation failed"))
    }
    pub fn state(&self) -> Result<NativeState, MediaError> {
        let mut s = std::mem::MaybeUninit::uninit();
        unsafe {
            ferro_linux_state(self.0 .0.as_ptr(), s.as_mut_ptr());
        }
        let s: NativeState = unsafe { s.assume_init() };
        if s.state == 5 {
            Err(s.error())
        } else {
            Ok(s)
        }
    }
    pub fn intent(&self, value: i32) {
        unsafe { ferro_linux_intent(self.0 .0.as_ptr(), value) }
    }
    pub fn seek(&self, ns: u64) -> u64 {
        unsafe { ferro_linux_seek(self.0 .0.as_ptr(), ns) }
    }
    pub fn audio(&self, volume: f64, muted: bool) {
        unsafe { ferro_linux_audio(self.0 .0.as_ptr(), volume, muted as i32) }
    }
    pub fn take_frame(&self) -> Result<Option<NativeFrame>, MediaError> {
        self.state()?;
        Ok(
            NonNull::new(unsafe { ferro_linux_take(self.0 .0.as_ptr()) })
                .map(|p| NativeFrame(Arc::new(Frame(p)))),
        )
    }
}
fn invalid(s: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::NativeFailure, s)
}
#[repr(C)]
struct RecordingInfo {
    width: u32,
    height: u32,
    duration_ns: u64,
    length: u64,
    error: [c_char; 512],
}

pub struct EncodedRecording {
    pub bytes: Arc<[u8]>,
    pub width: u32,
    pub height: u32,
    pub duration: Duration,
}

pub struct NativeRecording(NonNull<c_void>);
impl NativeRecording {
    pub fn started(&self) -> Poll<Result<(), MediaError>> {
        self.poll().map(|result| result.map(|_| ()))
    }

    pub fn finish(&self) {
        unsafe { ferro_linux_record_finish(self.0.as_ptr()) }
    }

    pub fn cancel(&self) {
        unsafe { ferro_linux_record_cancel(self.0.as_ptr()) }
    }

    fn poll(&self) -> Poll<Result<(i32, RecordingInfo), MediaError>> {
        let mut info = std::mem::MaybeUninit::uninit();
        let state = unsafe { ferro_linux_record_poll(self.0.as_ptr(), info.as_mut_ptr()) };
        let info = unsafe { info.assume_init() };
        match state {
            0 => Poll::Pending,
            1 | 2 => Poll::Ready(Ok((state, info))),
            -2 => Poll::Ready(Err(MediaError::new(
                MediaErrorKind::Cancelled,
                "Recording cancelled",
            ))),
            _ => {
                let message = unsafe { CStr::from_ptr(info.error.as_ptr()) }.to_string_lossy();
                Poll::Ready(Err(invalid(format!(
                    "GStreamer H.264 recording failed: {message}"
                ))))
            }
        }
    }

    pub fn encoded(&self) -> Poll<Result<EncodedRecording, MediaError>> {
        let (state, info) = match self.poll()? {
            Poll::Ready(info) => info,
            Poll::Pending => return Poll::Pending,
        };
        if state != 2 {
            return Poll::Pending;
        }
        if !(1..=256 * 1024 * 1024).contains(&info.length) {
            return Poll::Ready(Err(invalid("Invalid native recording length")));
        }
        let mut bytes = Arc::<[u8]>::new_uninit_slice(info.length as usize);
        let destination = Arc::get_mut(&mut bytes).unwrap().as_mut_ptr().cast();
        if unsafe { ferro_linux_record_copy(self.0.as_ptr(), destination, info.length) } == 0 {
            return Poll::Ready(Err(MediaError::new(
                MediaErrorKind::Cancelled,
                "Recording cancelled",
            )));
        }
        Poll::Ready(Ok(EncodedRecording {
            bytes: unsafe { bytes.assume_init() },
            width: info.width,
            height: info.height,
            duration: Duration::from_nanos(info.duration_ns),
        }))
    }
}
impl Drop for NativeRecording {
    fn drop(&mut self) {
        unsafe { ferro_linux_record_release(self.0.as_ptr()) }
    }
}

struct Frame(NonNull<c_void>);
// GstSample/GstBuffer are refcounted thread-safe, retained read-only until final
// GPU consumers complete. Native frame metadata is immutable after acquisition.
unsafe impl Send for Frame {}
unsafe impl Sync for Frame {}
impl Drop for Frame {
    fn drop(&mut self) {
        unsafe { ferro_linux_frame_release(self.0.as_ptr()) }
    }
}
#[derive(Clone)]
pub struct NativeFrame(Arc<Frame>);
impl NativeFrame {
    pub fn info(&self) -> FrameInfo {
        let mut v = FrameInfo::default();
        unsafe { ferro_linux_frame_info(self.0 .0.as_ptr(), &mut v) }
        v
    }
    pub fn photo_rgb(&self, counters: &FrameTransferCounters) -> Result<Vec<u8>, MediaError> {
        let info = self.info();
        if info.width > 4096 || info.height > 4096 || info.modifier != 0 {
            return Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "Photo snapshot requires a linear SDR frame up to 4096x4096",
            ));
        }
        let length = u64::from(info.width) * u64::from(info.height) * 3;
        let mut rgb = vec![0; length as usize];
        if unsafe { ferro_linux_photo_rgb(self.0 .0.as_ptr(), rgb.as_mut_ptr(), length) } == 0 {
            return Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "Native DMA-BUF snapshot mapping failed",
            ));
        }
        let source_bytes = if info.format == 0x3231564e {
            u64::from(info.width) * u64::from(info.height)
                + u64::from(info.width.div_ceil(2)) * u64::from(info.height.div_ceil(2)) * 2
        } else {
            u64::from(info.width) * u64::from(info.height) * 4
        };
        counters
            .cpu_transfer(
                PixelTransferPurpose::Photo,
                source_bytes,
                PresentationRoute::SharedTexture,
            )
            .map_err(|e| invalid(e.to_string()))?;
        counters
            .cpu_transfer(
                PixelTransferPurpose::Photo,
                length,
                PresentationRoute::SharedTexture,
            )
            .map_err(|e| invalid(e.to_string()))?;
        Ok(rgb)
    }
    pub(crate) fn convert(
        &self,
        context: &Rc<dyn IGlContext>,
        output: u32,
    ) -> Result<(), MediaError> {
        let display = context
            .native_egl_display()
            .ok_or_else(|| invalid("No current EGL display"))?;
        if unsafe { ferro_linux_convert(self.0 .0.as_ptr(), display, output) } != 0 {
            Ok(())
        } else {
            Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "DMA-BUF external-texture GPU conversion failed",
            ))
        }
    }
}
pub(crate) fn import(
    producer: &NativeGlFrameProducer,
    metadata: FrameMetadata,
    frame: NativeFrame,
) -> MediaFuture<Option<ReadyGlFrame>> {
    unsafe {
        producer.import_native(metadata, move |context| {
            let display = context
                .native_egl_display()
                .ok_or_else(|| invalid("No current EGL display"))?;
            let mut texture = 0;
            let alias = ferro_linux_alias(frame.0 .0.as_ptr(), display, &mut texture) as usize;
            if alias == 0 {
                return Err(MediaError::new(
                    MediaErrorKind::Unsupported,
                    "DMA-BUF EGL alias import failed",
                ));
            }
            Ok(NativeTextureView::new(
                context.clone(),
                texture,
                move |gl| {
                    ferro_linux_alias_release(alias as *mut c_void, gl.is_some() as i32);
                    drop(frame);
                },
            ))
        })
    }
}
