//! The functions of the NDK the backend calls, as safe functions.
//!
//! The reference declares them itself (`AndroidFramebuffer.cs`:
//! `ANativeWindow_*` and `AChoreographer_*` of `libandroid`), and its
//! runtime gives it the looper and the log. Here they are declared in this
//! one module: a native window is a counted reference ([`NativeWindow`]),
//! a locked buffer gives the buffer back when dropped ([`LockedBuffer`]),
//! and a frame callback is a reference that lives as long as the process.

use super::java::{raw_env, JavaObject, JavaRef};
use crate::log::LogPriority;
use std::ffi::{c_char, c_int, c_long, c_void, CString};
use std::ptr::{self, NonNull};
use std::sync::OnceLock;

#[repr(C)]
struct ANativeWindow {
    _private: [u8; 0],
}

#[repr(C)]
struct AChoreographer {
    _private: [u8; 0],
}

#[repr(C)]
struct ALooper {
    _private: [u8; 0],
}

#[repr(C)]
struct ARect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct ANativeWindowBuffer {
    // The number of pixels that are shown horizontally.
    width: i32,
    // The number of pixels that are shown vertically.
    height: i32,
    // The number of *pixels* that a line in the buffer takes in
    // memory.  This may be >= width.
    stride: i32,
    // The format of the buffer.  One of WINDOW_FORMAT_*
    format: i32,
    // The actual bits.
    bits: *mut c_void,
    // Do not touch.
    reserved: [u32; 6],
}

type FrameCallback = unsafe extern "C" fn(frame_time_nanos: c_long, data: *mut c_void);
type FrameCallback64 = unsafe extern "C" fn(frame_time_nanos: i64, data: *mut c_void);
type PostFrameCallback64 =
    unsafe extern "C" fn(choreographer: *mut AChoreographer, callback: FrameCallback64, data: *mut c_void);

#[link(name = "android")]
extern "C" {
    fn ANativeWindow_fromSurface(env: *mut c_void, surface: *mut c_void) -> *mut ANativeWindow;
    fn ANativeWindow_acquire(window: *mut ANativeWindow);
    fn ANativeWindow_release(window: *mut ANativeWindow);
    fn ANativeWindow_getWidth(window: *mut ANativeWindow) -> i32;
    fn ANativeWindow_getHeight(window: *mut ANativeWindow) -> i32;
    fn ANativeWindow_lock(
        window: *mut ANativeWindow,
        out_buffer: *mut ANativeWindowBuffer,
        in_out_dirty_bounds: *mut ARect,
    ) -> i32;
    fn ANativeWindow_unlockAndPost(window: *mut ANativeWindow) -> i32;

    fn AChoreographer_getInstance() -> *mut AChoreographer;
    fn AChoreographer_postFrameCallback(choreographer: *mut AChoreographer, callback: FrameCallback, data: *mut c_void);

    fn ALooper_prepare(opts: c_int) -> *mut ALooper;
    fn ALooper_pollOnce(
        timeout_millis: c_int,
        out_fd: *mut c_int,
        out_events: *mut c_int,
        out_data: *mut *mut c_void,
    ) -> c_int;
}

#[link(name = "log")]
extern "C" {
    fn __android_log_write(prio: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

/// A native window: a counted reference, taken from a surface and released
/// when the value is dropped.
pub(crate) struct NativeWindow {
    window: NonNull<ANativeWindow>,
}

// SAFETY: a native window is counted with atomic operations and its functions (lock,
// post, queries, the window surface of EGL) may be called from any thread; the value
// holds one count for as long as it lives.
unsafe impl Send for NativeWindow {}
// SAFETY: as above.
unsafe impl Sync for NativeWindow {}

impl NativeWindow {
    /// The native window of a `android.view.Surface`; `None` when the
    /// surface has none (it was released).
    pub fn from_surface(surface: &JavaObject) -> Option<NativeWindow> {
        // SAFETY: the environment is the one of the calling thread and the reference is a
        // valid object; the function checks that it is a surface and returns null when
        // it is not valid. The returned window carries a count, which this value owns.
        let window = unsafe { ANativeWindow_fromSurface(raw_env(), surface.raw().cast()) };
        NonNull::new(window).map(|window| NativeWindow { window })
    }

    /// The address of the window, which is what EGL takes as the native
    /// window. Valid while a reference to the window lives.
    pub fn handle(&self) -> isize {
        self.window.as_ptr() as isize
    }

    pub fn width(&self) -> i32 {
        // SAFETY: the window is alive: this value holds a count.
        unsafe { ANativeWindow_getWidth(self.window.as_ptr()) }
    }

    pub fn height(&self) -> i32 {
        // SAFETY: as `width`.
        unsafe { ANativeWindow_getHeight(self.window.as_ptr()) }
    }

    /// Locks the next buffer of the window for drawing in software; `None`
    /// when the window cannot be locked (it is connected to a GPU API, or
    /// its surface is gone).
    pub fn lock(&self) -> Option<LockedBuffer> {
        let mut buffer =
            ANativeWindowBuffer { width: 0, height: 0, stride: 0, format: 0, bits: ptr::null_mut(), reserved: [0; 6] };
        let mut bounds = ARect { left: 0, top: 0, right: self.width(), bottom: self.height() };
        // SAFETY: the window is alive; both pointers are valid for the structures the
        // function writes.
        let status = unsafe { ANativeWindow_lock(self.window.as_ptr(), &mut buffer, &mut bounds) };
        if status != 0 || buffer.bits.is_null() {
            return None;
        }
        Some(LockedBuffer {
            window: self.clone(),
            width: buffer.width,
            height: buffer.height,
            stride: buffer.stride,
            format: buffer.format,
            bits: buffer.bits.cast(),
        })
    }
}

impl Clone for NativeWindow {
    fn clone(&self) -> Self {
        // SAFETY: the window is alive; the new value owns the count taken here.
        unsafe { ANativeWindow_acquire(self.window.as_ptr()) };
        NativeWindow { window: self.window }
    }
}

impl Drop for NativeWindow {
    fn drop(&mut self) {
        // SAFETY: the count released is the one this value owns.
        unsafe { ANativeWindow_release(self.window.as_ptr()) };
    }
}

/// The buffer of a window while it is locked. Dropping it posts the buffer
/// to the screen.
pub(crate) struct LockedBuffer {
    window: NativeWindow,
    pub width: i32,
    pub height: i32,
    /// The number of pixels a line takes in memory.
    pub stride: i32,
    /// One of the `WINDOW_FORMAT_*` values.
    pub format: i32,
    bits: *mut u8,
}

impl LockedBuffer {
    /// The address of the first pixel: `stride * height` pixels of the
    /// format, valid until the buffer is dropped.
    pub fn bits(&self) -> *mut u8 {
        self.bits
    }
}

impl Drop for LockedBuffer {
    fn drop(&mut self) {
        // SAFETY: the window is locked by this value, which unlocks it once.
        unsafe { ANativeWindow_unlockAndPost(self.window.window.as_ptr()) };
    }
}

/// What a frame callback of the choreographer calls.
pub(crate) trait IFrameCallback: Sync + 'static {
    /// `frame_time_nanos` is the time the frame started being rendered, on
    /// the monotonic clock of the system.
    fn do_frame(&'static self, frame_time_nanos: i64);
}

/// The choreographer of the thread that asked for it.
#[derive(Clone, Copy)]
pub(crate) struct Choreographer {
    choreographer: NonNull<AChoreographer>,
}

// SAFETY: the choreographer of a thread lives as long as the thread's looper, which the
// thread that owns it never leaves (`looper_loop`), and posting a frame callback is the one
// thing done with it: the implementation takes its lock and hands the request to its own
// thread when called from another, which is how the reference uses it.
unsafe impl Send for Choreographer {}
// SAFETY: as above.
unsafe impl Sync for Choreographer {}

fn post_frame_callback64() -> Option<PostFrameCallback64> {
    static ENTRY: OnceLock<Option<PostFrameCallback64>> = OnceLock::new();
    *ENTRY.get_or_init(|| {
        // AChoreographer_postFrameCallback is deprecated on 10.0+; the 64-bit function is
        // newer than the API level the library is linked against, so it is looked up.
        // SAFETY: both strings are terminated; the library is the one this library is
        // linked against and stays loaded.
        let address = unsafe {
            let library = libc::dlopen(c"libandroid.so".as_ptr(), libc::RTLD_NOW);
            if library.is_null() {
                return None;
            }
            libc::dlsym(library, c"AChoreographer_postFrameCallback64".as_ptr())
        };
        // SAFETY: an address that is not null is the function of that name, whose
        // signature is the declared one.
        (!address.is_null()).then(|| unsafe { std::mem::transmute::<*mut c_void, PostFrameCallback64>(address) })
    })
}

impl Choreographer {
    /// The choreographer of the calling thread, which must have a looper
    /// ([`looper_prepare`]).
    pub fn instance() -> Option<Choreographer> {
        // SAFETY: the function takes nothing; it returns null on a thread without a looper.
        NonNull::new(unsafe { AChoreographer_getInstance() }).map(|choreographer| Choreographer { choreographer })
    }

    /// Asks for `target` to be called at the next frame, on the thread of
    /// the choreographer.
    pub fn post_frame_callback<T: IFrameCallback>(&self, target: &'static T) {
        unsafe extern "C" fn callback64<T: IFrameCallback>(frame_time_nanos: i64, data: *mut c_void) {
            // SAFETY: `data` is the `&'static T` this callback was posted with.
            let target: &'static T = unsafe { &*data.cast::<T>() };
            run_callback(|| target.do_frame(frame_time_nanos));
        }

        unsafe extern "C" fn callback<T: IFrameCallback>(frame_time_nanos: c_long, data: *mut c_void) {
            // SAFETY: as `callback64`.
            let target: &'static T = unsafe { &*data.cast::<T>() };
            run_callback(|| target.do_frame(frame_time_nanos as i64));
        }

        let data = ptr::from_ref(target).cast_mut().cast::<c_void>();
        // SAFETY: the choreographer is alive (see the `Send` argument); the callback is
        // called once with `data`, which refers to a value that lives as long as the
        // process.
        unsafe {
            match post_frame_callback64() {
                Some(post) => post(self.choreographer.as_ptr(), callback64::<T>, data),
                None => AChoreographer_postFrameCallback(self.choreographer.as_ptr(), callback::<T>, data),
            }
        }
    }
}

/// A callback of the system must not unwind into it: a panic is written to
/// the log and the process is stopped.
fn run_callback(body: impl FnOnce()) {
    if let Err(panic) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        let message = crate::interop::panic_message(&*panic);
        log_write(LogPriority::Fatal, crate::log::TAG, &format!("panic in a frame callback: {message}"));
        std::process::abort();
    }
}

/// Gives the calling thread a looper.
pub(crate) fn looper_prepare() {
    // SAFETY: the function takes an option value and creates or returns the looper of the
    // thread.
    unsafe { ALooper_prepare(0) };
}

/// Runs the looper of the calling thread, forever.
pub(crate) fn looper_loop() -> ! {
    loop {
        // SAFETY: null is allowed for the three outputs; the call waits for the next
        // event of the looper and runs its callbacks.
        unsafe { ALooper_pollOnce(-1, ptr::null_mut(), ptr::null_mut(), ptr::null_mut()) };
    }
}

/// Writes a line to the log of the system.
pub(crate) fn log_write(priority: LogPriority, tag: &str, message: &str) {
    let tag = CString::new(tag.replace('\0', " ")).expect("the tag has no NUL");
    let message = CString::new(message.replace('\0', " ")).expect("the message has no NUL");
    // SAFETY: both strings are terminated and live across the call.
    unsafe { __android_log_write(priority as c_int, tag.as_ptr(), message.as_ptr()) };
}
