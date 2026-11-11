use ferroui_media::{frames::*, *};
use ferroui_media_opengl::{NativeGlFrameProducer, NativeTextureView, ReadyGlFrame};
use ferroui_opengl::IGlContext;
use std::{
    ffi::{c_char, c_void, CStr},
    ptr::NonNull,
    rc::Rc,
    sync::Arc,
};

#[cfg(all(test, feature = "native-frame-tests"))]
mod native_frame_tests {
    use super::Egl;
    use ferroui_opengl::IPlatformGraphicsOpenGlContextFactory;
    extern "C" {
        fn ferro_windows_test_frames() -> i32;
        fn ferro_windows_test_mp4_geometry() -> i32;
        fn ferro_windows_test_alias(display: isize, egl: *const Egl, gl: *const isize) -> i32;
    }

    #[test]
    fn bounded_mp4_geometry() {
        assert_eq!(unsafe { ferro_windows_test_mp4_geometry() }, 0);
    }
    #[test]
    fn angle_context_churn_keeps_namespaces_and_other_contexts_alive() {
        let graphics =
            ferroui_win32::open_gl::angle::D3D11AngleWin32PlatformGraphics::new(None, None);
        let a = graphics.create_context(None);
        let b = graphics.create_context(None);
        let display = a.native_egl_display();
        assert_eq!(display, b.native_egl_display());
        assert!(!a.is_shared_with(&*b));
        let old_id = a.shared_frame_context_id();
        let current = a.make_current();
        let gl = a.gl_interface();
        let texture = gl.gen_texture();
        gl.bind_texture(0x0de1, texture);
        let other = b.make_current();
        let address = b.gl_interface().get_proc_address("glIsTexture");
        assert!(!address.is_null());
        let is_texture: unsafe extern "system" fn(u32) -> u8 =
            unsafe { std::mem::transmute(address) };
        assert_eq!(
            unsafe { is_texture(texture as u32) },
            0,
            "independent context must not share GL texture names"
        );
        other.dispose();
        gl.delete_texture(texture);
        current.dispose();
        a.dispose();
        for _ in 0..100 {
            let c = graphics.create_context(None);
            assert_eq!(display, c.native_egl_display());
            assert_ne!(old_id, c.shared_frame_context_id());
            let egl = c
                .as_any()
                .downcast_ref::<ferroui_opengl::egl::EglContext>()
                .unwrap()
                .egl_interface()
                .clone();
            let current = c.make_current();
            assert_ne!(egl.get_current_context(), 0);
            c.dispose();
            assert_eq!(
                egl.get_current_context(),
                0,
                "disposed context must not retain current native resources"
            );
            current.dispose();
            let current = b.make_current();
            assert!(!b.is_lost());
            assert_eq!(b.gl_interface().get_error(), 0);
            current.dispose();
        }
        b.dispose();
        let c = graphics.create_context(None);
        assert_eq!(display, c.native_egl_display());
        c.dispose();
        eprintln!("NATIVE_ANGLE_CONTEXT_CHURN_PASS contexts=103 namespaces=independent surviving_context=valid native_display=reused");
    }
    #[test]
    fn independent_d3d11_frames_and_admission() {
        assert_eq!(unsafe { ferro_windows_test_frames() }, 0);
    }

    #[test]
    fn independent_angle_alias_pixels() {
        let graphics =
            ferroui_win32::open_gl::angle::D3D11AngleWin32PlatformGraphics::new(None, None);
        let context = graphics.create_context(None);
        let current = context.make_current();
        let (display, egl) = Egl::current(&context).unwrap();
        let functions = [
            "glGenFramebuffers",
            "glBindFramebuffer",
            "glFramebufferTexture2D",
            "glCheckFramebufferStatus",
            "glReadPixels",
            "glDeleteFramebuffers",
        ]
        .map(|name| context.gl_interface().get_proc_address(name) as isize);
        assert!(!functions.contains(&0));
        let result = unsafe { ferro_windows_test_alias(display, &egl, functions.as_ptr()) };
        current.dispose();
        context.dispose();
        assert_eq!(result, 0);
    }
}
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
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FrameInfo {
    pub width: u32,
    pub height: u32,
    pub color: u32,
    pub rotation: u32,
    pub bytes: u64,
    pub timestamp_ns: u64,
    pub sequence: u64,
    pub epoch: u64,
    pub mirrored: u32,
    pub x: u32,
    pub y: u32,
    pub visible_width: u32,
    pub visible_height: u32,
}
#[repr(C)]
struct Egl {
    functions: [isize; 13],
}
impl Egl {
    fn current(context: &Rc<dyn IGlContext>) -> Result<(isize, Self), MediaError> {
        let display = context
            .native_egl_display()
            .ok_or_else(|| unsupported("Windows media needs an admitted ANGLE EGL context"))?;
        let names = [
            "eglQueryDisplayAttribEXT",
            "eglQueryDeviceAttribEXT",
            "eglCreateImageKHR",
            "eglDestroyImageKHR",
            "glEGLImageTargetTexture2DOES",
            "glGetIntegerv",
            "glActiveTexture",
            "glGenTextures",
            "glBindTexture",
            "glTexParameteri",
            "glGetError",
            "glDeleteTextures",
            "eglQueryString",
        ];
        let functions = names.map(|name| context.gl_interface().get_proc_address(name) as isize);
        if functions.contains(&0) {
            return Err(unsupported(
                "Required ANGLE native-image entry points are missing",
            ));
        }
        Ok((display, Self { functions }))
    }
}
extern "C" {
    fn ferro_windows_device(display: isize, egl: *const Egl) -> *mut c_void;
    fn ferro_windows_device_release(device: *mut c_void);
    fn ferro_windows_open(
        device: *mut c_void,
        source: *const u16,
        camera: i32,
        resource_file: i32,
        w: u32,
        h: u32,
        n: u32,
        d: u32,
    ) -> *mut c_void;
    fn ferro_windows_close(s: *mut c_void);
    fn ferro_windows_state(s: *mut c_void, out: *mut NativeState);
    fn ferro_windows_intent(s: *mut c_void, intent: i32);
    fn ferro_windows_seek(s: *mut c_void, ns: u64) -> u64;
    fn ferro_windows_audio(s: *mut c_void, volume: f64, muted: i32);
    fn ferro_windows_take(s: *mut c_void) -> *mut c_void;
    fn ferro_windows_frame_info(f: *mut c_void, out: *mut FrameInfo);
    fn ferro_windows_frame_release(f: *mut c_void);
    fn ferro_windows_alias(
        f: *mut c_void,
        display: isize,
        egl: *const Egl,
        texture: *mut u32,
    ) -> *mut c_void;
    fn ferro_windows_alias_release(alias: *mut c_void, alive: i32);
    fn ferro_windows_photo_rgb(f: *mut c_void, rgb: *mut u8, length: u64) -> i32;
    fn ferro_windows_devices() -> *mut c_char;
    fn ferro_windows_string_release(s: *mut c_char);
}
fn unsupported(s: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::Unsupported, s)
}
fn failure(s: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::NativeFailure, s)
}
struct Device(NonNull<c_void>);
// ID3D11Device is free-threaded. Its immediate context is protected using
// ID3D11Multithread before MF or still capture is given access to it.
unsafe impl Send for Device {}
unsafe impl Sync for Device {}
impl Drop for Device {
    fn drop(&mut self) {
        unsafe { ferro_windows_device_release(self.0.as_ptr()) }
    }
}
#[derive(Clone)]
pub struct NativeDevice(Arc<Device>);
impl NativeDevice {
    #[cfg(feature = "direct2d")]
    pub fn from_direct2d() -> Result<Self, MediaError> {
        NonNull::new(ferroui_direct2d::native_device())
            .map(|p| Self(Arc::new(Device(p))))
            .ok_or_else(|| unsupported("No native hardware D3D11 device"))
    }
    /// Native session objects, reader callbacks, synthetic streams, frame and
    /// alias wrappers. Complements worker/texture counts after asynchronous close.
    #[cfg(feature = "synthetic-camera")]
    pub fn fixture_owner_stats() -> [u32; 5] {
        extern "C" {
            fn ferro_windows_fixture_owners(out: *mut u32);
        }
        let mut out = [0; 5];
        unsafe { ferro_windows_fixture_owners(out.as_mut_ptr()) };
        out
    }
    /// Default-process-heap allocated/committed bytes (MAX if unavailable),
    /// then VirtualQuery private/mapped/image committed regions. Not all heaps.
    #[cfg(feature = "synthetic-camera")]
    pub fn fixture_memory_stats() -> [u64; 5] {
        extern "C" {
            fn ferro_windows_fixture_memory_stats(out: *mut u64);
        }
        let mut values = [0; 5];
        unsafe { ferro_windows_fixture_memory_stats(values.as_mut_ptr()) };
        values
    }
    #[cfg(feature = "synthetic-camera")]
    pub fn fixture_retirement_stats(&self) -> [u32; 3] {
        extern "C" {
            fn ferro_windows_fixture_retirement_stats(device: *mut c_void, out: *mut u32);
        }
        let mut counts = [0; 3];
        unsafe { ferro_windows_fixture_retirement_stats(self.0 .0.as_ptr(), counts.as_mut_ptr()) };
        counts
    }
    /// Opt-in fixture diagnostics: private bytes, working set, local GPU bytes
    /// (u64::MAX if unavailable), and process handle count.
    #[cfg(feature = "synthetic-camera")]
    pub fn fixture_process_stats(&self) -> Result<[u64; 4], MediaError> {
        extern "C" {
            fn ferro_windows_fixture_process_stats(device: *mut c_void, out: *mut u64) -> i32;
        }
        let mut values = [0; 4];
        if unsafe { ferro_windows_fixture_process_stats(self.0 .0.as_ptr(), values.as_mut_ptr()) }
            == 0
        {
            return Err(failure("Cannot query fixture process resources"));
        }
        Ok(values)
    }
    pub fn from_renderer(producer: &NativeGlFrameProducer) -> MediaFuture<Self> {
        producer.invoke(|context| {
            let (display, egl) = Egl::current(context)?;
            NonNull::new(unsafe { ferro_windows_device(display, &egl) })
                .map(|p| Self(Arc::new(Device(p))))
                .ok_or_else(|| unsupported("Active ANGLE device cannot share native D3D11 images"))
        })
    }
}
struct Session(NonNull<c_void>);
// All commands/snapshots use the C++ session mutex. COM and native pipeline work
// stay on its MTA worker; Drop signals cancellation without joining that worker.
unsafe impl Send for Session {}
unsafe impl Sync for Session {}
impl Drop for Session {
    fn drop(&mut self) {
        unsafe { ferro_windows_close(self.0.as_ptr()) }
    }
}
#[derive(Clone)]
pub struct NativeSession(Arc<Session>);
impl NativeSession {
    #[cfg(feature = "synthetic-camera")]
    pub fn fixture_photo_status() -> (bool, bool) {
        extern "C" {
            fn ferro_windows_fixture_photo_status() -> u32;
        }
        let status = unsafe { ferro_windows_fixture_photo_status() };
        (status & 1 != 0, status & 2 != 0)
    }
    #[cfg(feature = "synthetic-camera")]
    pub fn synthetic_pattern_epoch() -> u64 {
        extern "C" {
            fn ferro_windows_synthetic_pattern_epoch() -> u64;
        }
        unsafe { ferro_windows_synthetic_pattern_epoch() }
    }
    /// Diagnostic counts for the opt-in in-process camera fixture only.
    #[cfg(feature = "synthetic-camera")]
    pub fn synthetic_usage() -> (u32, u32, u32) {
        extern "C" {
            fn ferro_windows_synthetic_usage(
                sources: *mut u32,
                textures: *mut u32,
                workers: *mut u32,
            );
        }
        let (mut sources, mut textures, mut workers) = (0, 0, 0);
        unsafe { ferro_windows_synthetic_usage(&mut sources, &mut textures, &mut workers) };
        (sources, textures, workers)
    }
    pub fn open(
        device: &NativeDevice,
        source: &str,
        camera: bool,
        resource_file: bool,
        size: Option<(u32, u32, u32, u32)>,
    ) -> Result<Self, MediaError> {
        if source.contains('\0') {
            return Err(MediaError::new(
                MediaErrorKind::InvalidSource,
                "NUL in native media source",
            ));
        }
        let source: Vec<u16> = source.encode_utf16().chain(Some(0)).collect();
        let (w, h, n, d) = size.unwrap_or((0, 0, 0, 1));
        NonNull::new(unsafe {
            ferro_windows_open(
                device.0 .0.as_ptr(),
                source.as_ptr(),
                camera as i32,
                resource_file as i32,
                w,
                h,
                n,
                d,
            )
        })
        .map(|p| Self(Arc::new(Session(p))))
        .ok_or_else(|| failure("Native session slots are saturated or allocation failed"))
    }
    pub fn state(&self) -> Result<NativeState, MediaError> {
        let mut out = std::mem::MaybeUninit::uninit();
        unsafe { ferro_windows_state(self.0 .0.as_ptr(), out.as_mut_ptr()) };
        let s: NativeState = unsafe { out.assume_init() };
        if s.state == 5 {
            Err(MediaError::new(
                match s.error_kind {
                    1 => MediaErrorKind::Unsupported,
                    2 => MediaErrorKind::Permission,
                    3 => MediaErrorKind::DeviceLost,
                    _ => MediaErrorKind::NativeFailure,
                },
                unsafe { CStr::from_ptr(s.error.as_ptr()) }.to_string_lossy(),
            ))
        } else {
            Ok(s)
        }
    }
    pub fn intent(&self, v: i32) {
        unsafe { ferro_windows_intent(self.0 .0.as_ptr(), v) }
    }
    pub fn seek(&self, ns: u64) -> u64 {
        unsafe { ferro_windows_seek(self.0 .0.as_ptr(), ns) }
    }
    pub fn audio(&self, v: f64, m: bool) {
        unsafe { ferro_windows_audio(self.0 .0.as_ptr(), v, m as i32) }
    }
    pub fn take_frame(&self) -> Result<Option<NativeFrame>, MediaError> {
        self.state()?;
        Ok(
            NonNull::new(unsafe { ferro_windows_take(self.0 .0.as_ptr()) })
                .map(|p| NativeFrame(Arc::new(Frame(p)))),
        )
    }
    pub fn devices() -> Result<String, MediaError> {
        let text = NonNull::new(unsafe { ferro_windows_devices() })
            .ok_or_else(|| failure("Media Foundation camera enumeration failed"))?;
        let result = unsafe { CStr::from_ptr(text.as_ptr()) }
            .to_string_lossy()
            .into_owned();
        unsafe { ferro_windows_string_release(text.as_ptr()) };
        Ok(result)
    }
}
struct Frame(NonNull<c_void>);
// Immutable MF sample + retained DXGI texture. MF/D3D buffers are free-threaded;
// the sample remains held until the last GPU read and queued alias destruction.
unsafe impl Send for Frame {}
unsafe impl Sync for Frame {}
impl Drop for Frame {
    fn drop(&mut self) {
        unsafe { ferro_windows_frame_release(self.0.as_ptr()) }
    }
}
#[derive(Clone)]
pub struct NativeFrame(Arc<Frame>);
impl NativeFrame {
    #[cfg(feature = "direct2d")]
    pub fn direct2d_texture(&self) -> Result<ferroui_direct2d::D3D11Texture, MediaError> {
        extern "C" { fn ferro_windows_frame_texture(frame: *mut c_void,slice: *mut u32) -> *mut c_void; }
        let mut slice=0;
        // SAFETY: self retains the immutable MF sample and its exact array slice.
        unsafe {
            let texture=ferro_windows_frame_texture(self.0.0.as_ptr(),&mut slice);
            ferroui_direct2d::D3D11Texture::from_texture(texture,slice,self.clone())
                .ok_or_else(|| unsupported("Media texture is incompatible with native Direct2D device"))
        }
    }
    pub fn info(&self) -> FrameInfo {
        let mut v = FrameInfo::default();
        unsafe { ferro_windows_frame_info(self.0 .0.as_ptr(), &mut v) }
        v
    }
    pub fn photo_rgb(&self, counters: &FrameTransferCounters) -> Result<Vec<u8>, MediaError> {
        let info = self.info();
        if info.width > 4096 || info.height > 4096 {
            return Err(unsupported("Snapshot exceeds 4096x4096"));
        }
        let length = u64::from(info.width) * u64::from(info.height) * 3;
        // Photo-only staging; never callable from the preview import path.
        let mut rgb = vec![0; length as usize];
        let result =
            unsafe { ferro_windows_photo_rgb(self.0 .0.as_ptr(), rgb.as_mut_ptr(), length) };
        if result == 1 || result == 2 {
            counters.gpu_copy(GpuTransferReason::NativeImageTransfer);
        }
        if result != 1 {
            return Err(failure(if result == 2 {
                "Photo GPU completion is indeterminate; native source and staging quarantined"
            } else {
                "Native DXGI photo snapshot failed"
            }));
        }
        counters
            .cpu_transfer(
                PixelTransferPurpose::Photo,
                u64::from(info.width) * u64::from(info.height) * 4,
                PresentationRoute::SharedTexture,
            )
            .map_err(|e| failure(e.to_string()))?;
        counters
            .cpu_transfer(
                PixelTransferPurpose::Photo,
                length,
                PresentationRoute::SharedTexture,
            )
            .map_err(|e| failure(e.to_string()))?;
        Ok(rgb)
    }
}
pub(crate) fn import(
    producer: &NativeGlFrameProducer,
    metadata: FrameMetadata,
    frame: NativeFrame,
) -> MediaFuture<Option<ReadyGlFrame>> {
    unsafe {
        producer.import_native(metadata, move |context| {
            let (display, egl) = Egl::current(context)?;
            let mut texture = 0;
            let alias =
                ferro_windows_alias(frame.0 .0.as_ptr(), display, &egl, &mut texture) as usize;
            if alias == 0 {
                return Err(unsupported(
                    "Retained DXGI surface cannot be imported directly into ANGLE",
                ));
            }
            Ok(NativeTextureView::new(
                context.clone(),
                texture,
                move |gl| {
                    ferro_windows_alias_release(alias as *mut c_void, gl.is_some() as i32);
                    drop(frame);
                },
            ))
        })
    }
}
