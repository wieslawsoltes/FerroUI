//! The UI thread rule: UI objects belong to one thread, the one that called [`ferro_init`]. A call
//! of a UI function from another thread returns [`FERRO_WRONG_THREAD`], and [`ferro_post`] runs
//! work on the UI thread from any thread through its dispatcher.
//!
//! **The check costs one comparison.** The UI thread is remembered as a number that tells live
//! threads apart and that the calling thread reads without a call where the platform allows
//! ([`thread_token`]: the thread register on Apple arm64, `pthread_self` on other Unix systems,
//! `GetCurrentThreadId` on Windows), so the check reads no thread-local variable (a call into the
//! dynamic loader on macOS, a table lookup on Android). The state of the ABI that belongs to the
//! UI thread (the handle table, the resolved IDs) lives in statics behind [`UiCell`], which checks
//! the thread on every access: one comparison in place of a thread-local lookup and a borrow.

use crate::error::{guard, status, Error, Result};
use crate::{FerroPostDataFn, FerroPostFn, FerroStatus, FERRO_NOT_INITIALIZED, FERRO_NULL_POINTER, FERRO_WRONG_THREAD};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::cell::{Cell, UnsafeCell};
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::ThreadId;

/// The UI thread, once a thread has been made it.
struct UiThread {
    id: ThreadId,
    dispatcher: Arc<Dispatcher>,
}

static UI_THREAD: OnceLock<UiThread> = OnceLock::new();

/// The [`thread_token`] of the UI thread, `0` before there is one.
static UI_TOKEN: AtomicUsize = AtomicUsize::new(0);

/// A number that tells the calling thread apart from every other live thread, never `0`.
#[inline(always)]
pub(crate) fn thread_token() -> usize {
    #[cfg(all(target_arch = "aarch64", target_vendor = "apple"))]
    {
        // The base of the thread-specific data of the thread: one register read.
        let token: usize;
        // SAFETY: reads a register that user code may read; no memory, no flags.
        unsafe { std::arch::asm!("mrs {}, tpidrro_el0", out(reg) token, options(nomem, nostack, preserves_flags)) };
        token
    }
    #[cfg(all(unix, not(all(target_arch = "aarch64", target_vendor = "apple"))))]
    {
        extern "C" {
            fn pthread_self() -> usize;
        }
        // SAFETY: `pthread_self` has no preconditions.
        unsafe { pthread_self() }
    }
    #[cfg(windows)]
    {
        extern "system" {
            fn GetCurrentThreadId() -> u32;
        }
        // SAFETY: `GetCurrentThreadId` has no preconditions.
        unsafe { GetCurrentThreadId() as usize }
    }
    #[cfg(not(any(unix, windows)))]
    {
        // A target without threads of its own (WebAssembly without threads): one thread.
        1
    }
}

/// Whether the calling thread is the UI thread: one comparison.
#[inline(always)]
pub(crate) fn on_ui_thread() -> bool {
    let token = UI_TOKEN.load(Ordering::Relaxed);
    token != 0 && token == thread_token()
}

/// Fails unless the calling thread is the UI thread.
#[inline]
pub(crate) fn check() -> Result<()> {
    if on_ui_thread() {
        Ok(())
    } else {
        Err(check_slow())
    }
}

#[cold]
fn check_slow() -> Error {
    match UI_THREAD.get() {
        None => Error::new(FERRO_NOT_INITIALIZED, "no thread is the UI thread yet: call ferro_init or ferro_app_run"),
        Some(_) => Error::new(FERRO_WRONG_THREAD, "UI objects belong to the UI thread: post the work with ferro_post"),
    }
}

/// State of the UI thread in a static: every access checks that the calling thread is the UI
/// thread (one comparison, [`on_ui_thread`]) and that no access of the cell is running (the
/// closure of an access does not reach the same cell again), which is what makes the mutable
/// reference it hands out unique.
pub(crate) struct UiCell<T> {
    value: UnsafeCell<T>,
    busy: Cell<bool>,
}

// SAFETY: the value is reached only on the UI thread (`with` checks it), one access at a time.
unsafe impl<T> Sync for UiCell<T> {}

impl<T> UiCell<T> {
    pub(crate) const fn new(value: T) -> Self {
        UiCell { value: UnsafeCell::new(value), busy: Cell::new(false) }
    }

    /// Runs `access` with the value: an error off the UI thread. An access of the same cell
    /// inside `access` panics, as a second borrow of a `RefCell` does.
    #[inline]
    pub(crate) fn with<R>(&self, access: impl FnOnce(&mut T) -> R) -> Result<R> {
        if !on_ui_thread() {
            return Err(check_slow());
        }
        // Only the UI thread reaches here, so `busy` is not shared between threads.
        if self.busy.replace(true) {
            busy();
        }
        struct Release<'a>(&'a Cell<bool>);
        impl Drop for Release<'_> {
            #[inline]
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _release = Release(&self.busy);
        // SAFETY: on the UI thread, and no other access is running (`busy`): the reference is
        // the only one to the value until `access` returns.
        Ok(access(unsafe { &mut *self.value.get() }))
    }
}

#[cold]
#[inline(never)]
fn busy() -> ! {
    panic!("the state of the UI thread was reached again inside an access of it")
}

/// Makes the calling thread the UI thread (see [`ferro_init`]).
pub(crate) fn init() -> Result<()> {
    let current = std::thread::current().id();
    if let Some(ui) = UI_THREAD.get() {
        return if ui.id == current {
            UI_TOKEN.store(thread_token(), Ordering::Relaxed);
            Ok(())
        } else {
            Err(Error::new(FERRO_WRONG_THREAD, "another thread is the UI thread"))
        };
    }
    // The tables of types: every type, property and event of the framework can be found by name.
    ferroui_controls::register_types();
    // The dispatcher of the framework's UI thread: the first thread that has a dispatcher.
    let dispatcher = Dispatcher::ui_thread();
    if !dispatcher.check_access() {
        return Err(Error::new(FERRO_WRONG_THREAD, "the framework already runs its UI thread on another thread"));
    }
    if UI_THREAD.set(UiThread { id: current, dispatcher }).is_err() {
        // Another thread made itself the UI thread at the same moment: the framework gave the
        // dispatcher to one of the two, and the check above failed on the other.
        return Err(Error::new(FERRO_WRONG_THREAD, "another thread is the UI thread"));
    }
    UI_TOKEN.store(thread_token(), Ordering::Relaxed);
    Ok(())
}

/// Makes the calling thread the UI thread: UI objects can be created and used on it from now on,
/// and on no other thread. Registers the tables of types, so that every type, property and event
/// of the framework can be found by name. Calling it again on the UI thread does nothing; on
/// another thread it returns [`FERRO_WRONG_THREAD`]. [`ferro_app_run`](crate::ferro_app_run)
/// calls it, so an application that starts with `ferro_app_run` does not; a host that drives the
/// thread itself (tests, an application of its own) calls it first.
///
/// On macOS the UI thread of a desktop application is the main thread of the process.
#[no_mangle]
pub extern "C" fn ferro_init() -> FerroStatus {
    status(init)
}

/// Whether the calling thread is the UI thread. Any thread.
#[no_mangle]
pub extern "C" fn ferro_is_ui_thread() -> bool {
    guard(false, || check().is_ok())
}

/// The dispatcher of the UI thread.
fn ui_dispatcher() -> Result<&'static Arc<Dispatcher>> {
    UI_THREAD
        .get()
        .map(|ui| &ui.dispatcher)
        .ok_or_else(|| Error::new(FERRO_NOT_INITIALIZED, "no thread is the UI thread yet: call ferro_init or ferro_app_run"))
}

/// Runs `function(token)` on the UI thread, after the work already queued there, at the normal
/// priority of the dispatcher. Any thread, the UI thread included (the work then runs later, not
/// inside the call). Work that is still queued when the application ends does not run.
#[no_mangle]
pub extern "C" fn ferro_post(function: FerroPostFn, token: i64) -> FerroStatus {
    status(|| {
        let function = function.ok_or_else(|| Error::new(FERRO_NULL_POINTER, "the function is NULL"))?;
        ui_dispatcher()?.post(
            move || {
                // SAFETY: the caller gave a function of the `FerroPostFn` signature.
                unsafe { function(token) };
            },
            DispatcherPriority::NORMAL,
        );
        Ok(())
    })
}

/// A pointer that the caller hands over to the UI thread.
struct UserData(*mut c_void);

// SAFETY: the pointer is only passed back to the function of the caller, on the UI thread, which
// is what the caller asked for by posting it there.
unsafe impl Send for UserData {}

impl UserData {
    fn get(&self) -> *mut c_void {
        self.0
    }
}

/// [`ferro_post`] with a `void *` for the work in place of a token: runs `function(user_data)` on
/// the UI thread once. Work that is still queued when the application ends does not run, and its
/// user data is not released.
#[no_mangle]
pub extern "C" fn ferro_post_data(function: FerroPostDataFn, user_data: *mut c_void) -> FerroStatus {
    status(|| {
        let function = function.ok_or_else(|| Error::new(FERRO_NULL_POINTER, "the function is NULL"))?;
        let user_data = UserData(user_data);
        ui_dispatcher()?.post(
            move || {
                // SAFETY: the caller gave a function of the `FerroPostDataFn` signature, and the
                // user data that goes with it.
                unsafe { function(user_data.get()) };
            },
            DispatcherPriority::NORMAL,
        );
        Ok(())
    })
}

/// Runs the work queued on the UI thread (posted work, timers that are due, layout and the other
/// jobs of the dispatcher) and returns once the queue is empty: for a host that drives the UI
/// thread itself instead of [`ferro_app_run`](crate::ferro_app_run). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_run_jobs() -> FerroStatus {
    status(|| {
        check()?;
        ui_dispatcher()?.run_jobs(None);
        Ok(())
    })
}
