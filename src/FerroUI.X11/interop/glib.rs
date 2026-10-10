//! The functions of GLib and GObject the platform calls (the port of
//! `Interop/Glib.cs`): the main loop, idle, timeout and file descriptor
//! sources of the default main context, and signals of objects.
//!
//! `libglib-2.0.so.0` and `libgobject-2.0.so.0` are opened when
//! [`Glib::try_get`] is first called; without them it answers with an
//! error, and what needs GLib (the GTK dialogs, the dispatcher over the
//! GLib main loop) is not available.
//!
//! A callback that is given to a source runs on the thread that iterates
//! the default main context, which is in general not the thread that added
//! the source: callbacks are `Send`. A panic cannot cross the C frames of
//! GLib, so a callback that panics is caught at the boundary and logged;
//! callers that want the panic (the dispatcher) catch it themselves first.

use super::native_library::{native_functions, NativeLibrary, NativeLibraryError};
use bitflags::bitflags;
use ferroui_base::logging::{LogEventLevel, Logger};
use std::any::Any;
use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void, CString};
use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::task::{Context, Poll, Waker};

const GLIB_NAME: &std::ffi::CStr = c"libglib-2.0.so.0";
const GOBJECT_NAME: &std::ffi::CStr = c"libgobject-2.0.so.0";

/// `GSourceFunc`.
pub type GSourceFunc = unsafe extern "C" fn(user_data: *mut c_void) -> c_int;
/// `GDestroyNotify`.
pub type GDestroyNotify = unsafe extern "C" fn(user_data: *mut c_void);
/// `GUnixFDSourceFunc`.
pub type GUnixFDSourceFunc = unsafe extern "C" fn(fd: c_int, condition: c_uint, user_data: *mut c_void) -> c_int;
/// `GClosureNotify`.
type GClosureNotify = unsafe extern "C" fn(data: *mut c_void, closure: *mut c_void);

/// A node of a singly linked list of GLib.
#[repr(C)]
pub struct GSList {
    pub data: *mut c_void,
    pub next: *mut GSList,
}

bitflags! {
    /// `GIOCondition`.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct GIOCondition: u32 {
        const G_IO_IN = 1;
        const G_IO_OUT = 4;
        const G_IO_PRI = 2;
        const G_IO_ERR = 8;
        const G_IO_HUP = 16;
        const G_IO_NVAL = 32;
    }
}

pub const G_PRIORITY_HIGH: i32 = -100;
pub const G_PRIORITY_DEFAULT: i32 = 0;
pub const G_PRIORITY_HIGH_IDLE: i32 = 100;
pub const G_PRIORITY_DEFAULT_IDLE: i32 = 200;

native_functions! {
    /// The functions, as C declares them.
    pub(crate) struct GlibApi(glib, gobject) {
        glib::g_slist_free: unsafe extern "C" fn(*mut GSList);
        gobject::g_object_ref: unsafe extern "C" fn(*mut c_void) -> *mut c_void;
        // The reference connects with `g_signal_connect_object` and no object, and gives it
        // a function its runtime generates for a delegate. A closure of this port reaches
        // its state through the data of the connection, which is this function.
        gobject::g_signal_connect_data: unsafe extern "C" fn(
            *mut c_void,
            *const c_char,
            *const c_void,
            *mut c_void,
            Option<GClosureNotify>,
            c_int,
        ) -> c_ulong;
        gobject::g_object_unref: unsafe extern "C" fn(*mut c_void);
        gobject::g_signal_handler_disconnect: unsafe extern "C" fn(*mut c_void, c_ulong);
        glib::g_main_loop_new: unsafe extern "C" fn(*mut c_void, c_int) -> *mut c_void;
        glib::g_main_loop_quit: unsafe extern "C" fn(*mut c_void);
        glib::g_main_loop_run: unsafe extern "C" fn(*mut c_void);
        glib::g_main_loop_unref: unsafe extern "C" fn(*mut c_void);
        glib::g_idle_add: unsafe extern "C" fn(GSourceFunc, *mut c_void) -> c_uint;
        // The reference adds a timeout with `g_timeout_add` and never frees the state of
        // one that is removed before it fires; with a destroy notification it is freed.
        glib::g_timeout_add_full:
            unsafe extern "C" fn(c_int, c_uint, GSourceFunc, *mut c_void, Option<GDestroyNotify>) -> c_uint;
        glib::g_idle_add_full: unsafe extern "C" fn(c_int, GSourceFunc, *mut c_void, Option<GDestroyNotify>) -> c_uint;
        glib::g_source_get_can_recurse: unsafe extern "C" fn(*mut c_void) -> c_int;
        glib::g_source_set_can_recurse: unsafe extern "C" fn(*mut c_void, c_int);
        glib::g_main_context_find_source_by_id: unsafe extern "C" fn(*mut c_void, c_uint) -> *mut c_void;
        glib::g_unix_fd_add_full:
            unsafe extern "C" fn(c_int, c_int, c_uint, GUnixFDSourceFunc, *mut c_void, Option<GDestroyNotify>) -> c_uint;
        glib::g_source_remove: unsafe extern "C" fn(c_uint) -> c_int;
    }
}

/// The message of a panic that was caught.
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "a panic without a message".to_string()
    }
}

/// Runs a callback GLib called. A panic ends here, logged: it cannot
/// unwind through the C frames of the library.
fn guarded<R>(default: R, call: impl FnOnce() -> R) -> R {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(value) => value,
        Err(payload) => {
            if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Control") {
                logger.log(None, &format!("Unhandled panic in a callback of GLib: {}", panic_message(&*payload)));
            }
            default
        }
    }
}

type OnceCallback = Option<Box<dyn FnOnce() + Send>>;
type SourceCallback = Box<dyn Fn() -> bool + Send>;
type UnixFdCallback = Box<dyn Fn(i32, GIOCondition) -> bool + Send>;

/// `s_onceSourceCb`: runs the callback and removes the source.
unsafe extern "C" fn once_source_cb(user_data: *mut c_void) -> c_int {
    // SAFETY: the data of the source is the `OnceCallback` the source was added with, which
    // lives until the destroy notification of the source.
    let callback = unsafe { &mut *user_data.cast::<OnceCallback>() }.take();
    if let Some(callback) = callback {
        guarded((), callback);
    }
    0
}

/// `s_onceSourceCb` for an idle source added without a destroy
/// notification: the source owns the state until it runs, which it always
/// does.
unsafe extern "C" fn once_source_owning_cb(user_data: *mut c_void) -> c_int {
    // SAFETY: the data is the box `g_idle_add_once` leaked for this one call.
    let callback = unsafe { Box::from_raw(user_data.cast::<OnceCallback>()) };
    if let Some(callback) = *callback {
        guarded((), callback);
    }
    0
}

unsafe extern "C" fn once_destroy_notify(user_data: *mut c_void) {
    // SAFETY: the data is the box the source was added with; GLib calls this once, when the
    // source is destroyed.
    drop(unsafe { Box::from_raw(user_data.cast::<OnceCallback>()) });
}

/// `s_sourceFuncDispatchCallback`.
unsafe extern "C" fn source_func_dispatch_callback(user_data: *mut c_void) -> c_int {
    // SAFETY: the data is the `SourceCallback` of the source, alive until its destroy
    // notification; it is only ever shared (a source that may recurse enters it again).
    let callback = unsafe { &*user_data.cast::<SourceCallback>() };
    guarded(false, || callback()) as c_int
}

/// `s_gcHandleDestroyNotify`.
unsafe extern "C" fn source_destroy_notify(user_data: *mut c_void) {
    // SAFETY: as `once_destroy_notify`.
    drop(unsafe { Box::from_raw(user_data.cast::<SourceCallback>()) });
}

/// `s_unixFdSourceCallback`.
unsafe extern "C" fn unix_fd_source_callback(fd: c_int, condition: c_uint, user_data: *mut c_void) -> c_int {
    // SAFETY: the data is the `UnixFdCallback` of the source, alive until its destroy
    // notification; it is only ever shared (the source of the connection to the X server
    // may recurse, and then enters it again).
    let callback = unsafe { &*user_data.cast::<UnixFdCallback>() };
    guarded(true, || callback(fd, GIOCondition::from_bits_retain(condition))) as c_int
}

unsafe extern "C" fn unix_fd_destroy_notify(user_data: *mut c_void) {
    // SAFETY: as `once_destroy_notify`.
    drop(unsafe { Box::from_raw(user_data.cast::<UnixFdCallback>()) });
}

/// `signal_generic` of the reference: the handler of a signal without
/// arguments.
pub type SignalGeneric = Box<dyn Fn(usize) -> bool>;
/// `signal_dialog_response`: the handler of the response of a dialog.
pub type SignalDialogResponse = Box<dyn Fn(usize, i32) -> bool>;

unsafe extern "C" fn signal_generic_trampoline(instance: *mut c_void, user_data: *mut c_void) -> c_int {
    // SAFETY: the data of the connection is the handler, alive until the closure of the
    // connection is finalized, which GLib does after the last call of it has returned.
    let handler = unsafe { &*user_data.cast::<SignalGeneric>() };
    guarded(false, || handler(instance as usize)) as c_int
}

unsafe extern "C" fn signal_generic_destroy(data: *mut c_void, _closure: *mut c_void) {
    // SAFETY: the data is the box the signal was connected with, freed once here.
    drop(unsafe { Box::from_raw(data.cast::<SignalGeneric>()) });
}

unsafe extern "C" fn signal_dialog_response_trampoline(
    instance: *mut c_void,
    response: c_int,
    user_data: *mut c_void,
) -> c_int {
    // SAFETY: as `signal_generic_trampoline`.
    let handler = unsafe { &*user_data.cast::<SignalDialogResponse>() };
    guarded(false, || handler(instance as usize, response)) as c_int
}

unsafe extern "C" fn signal_dialog_response_destroy(data: *mut c_void, _closure: *mut c_void) {
    // SAFETY: as `signal_generic_destroy`.
    drop(unsafe { Box::from_raw(data.cast::<SignalDialogResponse>()) });
}

/// A signal that was connected (`ConnectedSignal`). Disconnecting it frees
/// the handler once no call of it is running.
pub struct ConnectedSignal {
    glib: Glib,
    instance: usize,
    id: std::cell::Cell<c_ulong>,
}

impl ConnectedSignal {
    pub fn dispose(&self) {
        let id = self.id.replace(0);
        if id != 0 {
            // SAFETY: the instance is kept alive by the reference this object took, and the
            // identifier is the one the connection returned, used once.
            unsafe {
                (self.glib.api.g_signal_handler_disconnect)(self.instance as *mut c_void, id);
                (self.glib.api.g_object_unref)(self.instance as *mut c_void);
            }
        }
    }
}

/// A main loop of the default main context (`g_main_loop_new` and the
/// functions that take its result). GLib lets any thread ask a loop to
/// quit.
pub struct GMainLoop {
    glib: Glib,
    main_loop: usize,
}

impl GMainLoop {
    pub fn g_main_loop_run(&self) {
        // SAFETY: the loop is alive while this object is.
        unsafe { (self.glib.api.g_main_loop_run)(self.main_loop as *mut c_void) }
    }

    pub fn g_main_loop_quit(&self) {
        // SAFETY: the loop is alive while this object is; the function is safe to call from
        // any thread.
        unsafe { (self.glib.api.g_main_loop_quit)(self.main_loop as *mut c_void) }
    }
}

impl Drop for GMainLoop {
    fn drop(&mut self) {
        // SAFETY: gives back the one reference `g_main_loop_new` returned.
        unsafe { (self.glib.api.g_main_loop_unref)(self.main_loop as *mut c_void) }
    }
}

/// What a call on the thread of GLib answers with: a slot one thread
/// fills and a task of another awaits (the `TaskCompletionSource` of
/// `RunOnGlibThread`).
struct Slot<T> {
    value: Option<Result<T, String>>,
    waker: Option<Waker>,
}

/// The sending half of a [`GlibTask`].
pub struct GlibTaskCompletion<T>(Arc<Mutex<Slot<T>>>);

impl<T> GlibTaskCompletion<T> {
    /// Completes the task, unless it is completed already.
    pub fn try_set_result(&self, value: Result<T, String>) {
        let waker = {
            let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if slot.value.is_some() {
                return;
            }
            slot.value = Some(value);
            slot.waker.take()
        };
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

impl<T> Clone for GlibTaskCompletion<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

/// The future of a result that is computed on the thread of GLib.
pub struct GlibTask<T>(Arc<Mutex<Slot<T>>>);

impl<T> GlibTask<T> {
    pub fn new() -> (GlibTaskCompletion<T>, GlibTask<T>) {
        let slot = Arc::new(Mutex::new(Slot { value: None, waker: None }));
        (GlibTaskCompletion(slot.clone()), GlibTask(slot))
    }
}

impl<T> Future for GlibTask<T> {
    type Output = Result<T, String>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match slot.value.take() {
            Some(value) => Poll::Ready(value),
            None => {
                slot.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// GLib, loaded.
#[derive(Clone, Copy)]
pub struct Glib {
    api: &'static GlibApi,
}

impl Glib {
    /// The libraries, opened at the first call; the error of that call
    /// when they cannot be.
    pub fn try_get() -> Result<Glib, NativeLibraryError> {
        static API: OnceLock<Result<GlibApi, NativeLibraryError>> = OnceLock::new();
        let api = API.get_or_init(|| {
            let glib = NativeLibrary::open(GLIB_NAME)?;
            let gobject = NativeLibrary::open(GOBJECT_NAME)?;
            GlibApi::load(&glib, &gobject)
        });
        match api {
            Ok(api) => Ok(Glib { api }),
            Err(error) => Err(error.clone()),
        }
    }

    /// Frees a list.
    ///
    /// # Safety
    /// `list` is null or a list GLib allocated that nothing else frees.
    pub unsafe fn g_slist_free(&self, list: *mut GSList) {
        // SAFETY: the contract of this function.
        unsafe { (self.api.g_slist_free)(list) }
    }

    /// A main loop of the default main context.
    pub fn g_main_loop_new(&self, is_running: bool) -> GMainLoop {
        // SAFETY: a null context is the default one.
        let main_loop = unsafe { (self.api.g_main_loop_new)(std::ptr::null_mut(), is_running as c_int) };
        GMainLoop { glib: *self, main_loop: main_loop as usize }
    }

    pub fn g_idle_add_once(&self, cb: Box<dyn FnOnce() + Send>) {
        let data: *mut OnceCallback = Box::into_raw(Box::new(Some(cb)));
        // SAFETY: the source function takes the box back when it runs; an idle source
        // without a way to remove it always runs.
        unsafe { (self.api.g_idle_add)(once_source_owning_cb, data.cast()) };
    }

    pub fn g_timeout_add_once(&self, interval: u32, cb: Box<dyn FnOnce() + Send>) -> u32 {
        let data: *mut OnceCallback = Box::into_raw(Box::new(Some(cb)));
        // SAFETY: the data lives until the destroy notification, which frees it; the source
        // function returns 0, so the source is destroyed after its first call.
        unsafe {
            (self.api.g_timeout_add_full)(G_PRIORITY_DEFAULT, interval, once_source_cb, data.cast(), Some(once_destroy_notify))
        }
    }

    pub fn g_idle_add_full(&self, priority: i32, callback: Box<dyn Fn() -> bool + Send>) -> u32 {
        let data: *mut SourceCallback = Box::into_raw(Box::new(callback));
        // SAFETY: the data lives until the destroy notification, which frees it.
        unsafe {
            (self.api.g_idle_add_full)(priority, source_func_dispatch_callback, data.cast(), Some(source_destroy_notify))
        }
    }

    pub fn g_unix_fd_add_full(
        &self,
        priority: i32,
        fd: i32,
        condition: GIOCondition,
        cb: Box<dyn Fn(i32, GIOCondition) -> bool + Send>,
    ) -> u32 {
        let data: *mut UnixFdCallback = Box::into_raw(Box::new(cb));
        // SAFETY: the data lives until the destroy notification, which frees it. A
        // descriptor that is not open makes the source report an error condition, nothing
        // else.
        unsafe {
            (self.api.g_unix_fd_add_full)(
                priority,
                fd,
                condition.bits(),
                unix_fd_source_callback,
                data.cast(),
                Some(unix_fd_destroy_notify),
            )
        }
    }

    /// Removes a source of the default main context; whether there was
    /// one.
    pub fn g_source_remove(&self, tag: u32) -> bool {
        // SAFETY: takes an identifier; one that names no source is reported, not used.
        unsafe { (self.api.g_source_remove)(tag) != 0 }
    }

    /// `g_main_context_find_source_by_id` of the default context and
    /// `g_source_get_can_recurse`; `None` without such a source.
    pub fn g_source_get_can_recurse(&self, source_id: u32) -> Option<bool> {
        // SAFETY: the source that is found is used at once, on the thread that owns the
        // sources it looks up, before it can be destroyed.
        unsafe {
            let source = (self.api.g_main_context_find_source_by_id)(std::ptr::null_mut(), source_id);
            (!source.is_null()).then(|| (self.api.g_source_get_can_recurse)(source) != 0)
        }
    }

    /// `g_main_context_find_source_by_id` of the default context and
    /// `g_source_set_can_recurse`.
    pub fn g_source_set_can_recurse(&self, source_id: u32, can_recurse: bool) {
        // SAFETY: as `g_source_get_can_recurse`.
        unsafe {
            let source = (self.api.g_main_context_find_source_by_id)(std::ptr::null_mut(), source_id);
            if !source.is_null() {
                (self.api.g_source_set_can_recurse)(source, can_recurse as c_int);
            }
        }
    }

    /// Connects `handler` to the signal `name` of a GObject.
    ///
    /// # Safety
    /// `instance` is a live GObject, and the signal has the C signature of
    /// `trampoline` (possibly returning nothing).
    unsafe fn connect_signal(
        &self,
        instance: usize,
        name: &str,
        trampoline: *const c_void,
        data: *mut c_void,
        destroy: GClosureNotify,
    ) -> Result<ConnectedSignal, String> {
        let utf = CString::new(name).map_err(|_| format!("Unable to connect to signal {name}"))?;
        // SAFETY: the contract of this function; the data is freed by `destroy` when the
        // closure of the connection is finalized, also when the connection fails.
        let id = unsafe {
            (self.api.g_signal_connect_data)(instance as *mut c_void, utf.as_ptr(), trampoline, data, Some(destroy), 0)
        };
        if id == 0 {
            return Err(format!("Unable to connect to signal {name}"));
        }
        // SAFETY: the instance is alive (the contract); the reference is given back by
        // `ConnectedSignal::dispose`.
        unsafe { (self.api.g_object_ref)(instance as *mut c_void) };
        Ok(ConnectedSignal { glib: *self, instance, id: std::cell::Cell::new(id) })
    }

    /// `ConnectSignal<signal_generic>`.
    ///
    /// # Safety
    /// `instance` is a live GObject whose signal `name` passes the instance
    /// and the user data and nothing else.
    pub unsafe fn connect_signal_generic(
        &self,
        instance: usize,
        name: &str,
        handler: SignalGeneric,
    ) -> Result<ConnectedSignal, String> {
        let data = Box::into_raw(Box::new(handler));
        // SAFETY: the contract of this function.
        unsafe {
            self.connect_signal(instance, name, signal_generic_trampoline as *const c_void, data.cast(), signal_generic_destroy)
        }
    }

    /// `ConnectSignal<signal_dialog_response>`.
    ///
    /// # Safety
    /// `instance` is a live GObject whose signal `name` passes the
    /// instance, an integer and the user data.
    pub unsafe fn connect_signal_dialog_response(
        &self,
        instance: usize,
        name: &str,
        handler: SignalDialogResponse,
    ) -> Result<ConnectedSignal, String> {
        let data = Box::into_raw(Box::new(handler));
        // SAFETY: the contract of this function.
        unsafe {
            self.connect_signal(
                instance,
                name,
                signal_dialog_response_trampoline as *const c_void,
                data.cast(),
                signal_dialog_response_destroy,
            )
        }
    }

    /// Runs `action` on the thread that iterates the default main context
    /// and resolves to what it returned, or to the message of its panic.
    pub fn run_on_glib_thread<T: Send + 'static>(&self, action: impl FnOnce() -> T + Send + 'static) -> GlibTask<T> {
        let (tcs, task) = GlibTask::new();
        self.g_timeout_add_once(
            0,
            Box::new(move || match catch_unwind(AssertUnwindSafe(action)) {
                Ok(value) => tcs.try_set_result(Ok(value)),
                Err(payload) => tcs.try_set_result(Err(panic_message(&*payload))),
            }),
        );
        task
    }
}

#[cfg(test)]
pub(crate) mod tests {
    // Not from the reference. The tests need GLib and return at once without it (the
    // development machine); the CI job and the virtual machine have it.
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::MutexGuard;

    /// The default main context is one for the process: tests that run it
    /// take turns.
    pub(crate) fn main_context_lock() -> MutexGuard<'static, ()> {
        static LOCK: Mutex<()> = Mutex::new(());
        LOCK.lock().unwrap_or_else(PoisonError::into_inner)
    }

    #[test]
    fn the_libraries_load_or_say_which_is_missing() {
        match Glib::try_get() {
            Ok(_) => {}
            Err(error) => assert!(error.to_string().contains("libglib-2.0.so.0"), "{error}"),
        }
    }

    #[test]
    fn sources_run_in_the_order_of_their_priorities_and_once() {
        let Ok(glib) = Glib::try_get() else {
            return;
        };
        let _lock = main_context_lock();
        let main_loop = Arc::new(glib.g_main_loop_new(true));
        let order = Arc::new(Mutex::new(Vec::new()));

        let o = order.clone();
        glib.g_idle_add_once(Box::new(move || o.lock().unwrap().push("idle")));
        let o = order.clone();
        let calls = AtomicU32::new(0);
        glib.g_idle_add_full(
            G_PRIORITY_DEFAULT - 1,
            Box::new(move || {
                o.lock().unwrap().push("high");
                calls.fetch_add(1, Ordering::SeqCst) < 1
            }),
        );
        let (o, l) = (order.clone(), main_loop.clone());
        glib.g_timeout_add_once(
            30,
            Box::new(move || {
                o.lock().unwrap().push("timeout");
                l.g_main_loop_quit();
            }),
        );
        // A timeout that is removed never runs, and its state is freed.
        let dropped = Arc::new(AtomicU32::new(0));
        struct CountDrop(Arc<AtomicU32>);
        impl Drop for CountDrop {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let guard = CountDrop(dropped.clone());
        let tag = glib.g_timeout_add_once(1, Box::new(move || panic!("removed: {}", guard.0.load(Ordering::SeqCst))));
        assert!(glib.g_source_remove(tag));
        assert_eq!(dropped.load(Ordering::SeqCst), 1);

        main_loop.g_main_loop_run();
        assert_eq!(*order.lock().unwrap(), ["high", "high", "idle", "timeout"]);
    }

    #[test]
    fn a_descriptor_source_reports_its_condition_and_can_recurse_when_told() {
        let Ok(glib) = Glib::try_get() else {
            return;
        };
        let _lock = main_context_lock();
        let mut fds = [0i32; 2];
        // SAFETY: room for two descriptors.
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        let main_loop = Arc::new(glib.g_main_loop_new(true));
        let seen = Arc::new(Mutex::new(None));
        let (s, l) = (seen.clone(), main_loop.clone());
        let tag = glib.g_unix_fd_add_full(
            G_PRIORITY_DEFAULT,
            fds[0],
            GIOCondition::G_IO_IN,
            Box::new(move |fd, condition| {
                *s.lock().unwrap() = Some((fd, condition));
                l.g_main_loop_quit();
                false
            }),
        );
        assert_eq!(glib.g_source_get_can_recurse(tag), Some(false));
        glib.g_source_set_can_recurse(tag, true);
        assert_eq!(glib.g_source_get_can_recurse(tag), Some(true));
        // SAFETY: writes one byte of a local value to the pipe of this test.
        assert_eq!(unsafe { libc::write(fds[1], [1u8].as_ptr().cast(), 1) }, 1);
        main_loop.g_main_loop_run();
        assert_eq!(*seen.lock().unwrap(), Some((fds[0], GIOCondition::G_IO_IN)));
        // The callback returned false: the source is gone.
        assert_eq!(glib.g_source_get_can_recurse(tag), None);
        // SAFETY: closes the descriptors of this test.
        unsafe {
            libc::close(fds[0]);
            libc::close(fds[1]);
        }
    }

    #[test]
    fn a_call_on_the_thread_of_glib_answers_with_its_result_or_its_panic() {
        let Ok(glib) = Glib::try_get() else {
            return;
        };
        let _lock = main_context_lock();
        let main_loop = Arc::new(glib.g_main_loop_new(true));
        let ok = glib.run_on_glib_thread(|| 6 * 7);
        let failed = glib.run_on_glib_thread(|| -> i32 { panic!("no answer") });
        let l = main_loop.clone();
        glib.g_timeout_add_once(20, Box::new(move || l.g_main_loop_quit()));
        main_loop.g_main_loop_run();

        let block = |task: GlibTask<i32>| {
            let waker = Waker::noop();
            let mut task = std::pin::pin!(task);
            match task.as_mut().poll(&mut Context::from_waker(waker)) {
                Poll::Ready(value) => value,
                Poll::Pending => panic!("the call did not run"),
            }
        };
        assert_eq!(block(ok), Ok(42));
        assert_eq!(block(failed), Err("no answer".to_string()));
    }
}
