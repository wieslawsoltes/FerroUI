//! Running an application: [`ferro_app_run`] sets up the platform of the operating system (the
//! desktop backends, with the Fluent theme), calls the start callback, and runs the main loop until
//! the last window shown with [`ferro_show`] closes or [`ferro_quit`] is called.

use crate::error::{status, Error, Result};
use crate::handles;
use crate::thread;
use crate::{FerroHandle, FerroStartFn, FerroStatus, FERRO_INVALID_ARGUMENT, FERRO_INVALID_STATE, FERRO_TYPE_MISMATCH};
use ferroui_base::threading::CancellationToken;
use ferroui_base::Ref;
use ferroui_controls::Window;
use std::cell::{Cell, RefCell};
use std::ffi::c_void;

/// The state of the application of the UI thread.
#[derive(Default)]
struct AppState {
    /// Whether `ferro_app_run` has started (it runs once per process).
    started: Cell<bool>,
    /// Ends the main loop.
    stop: RefCell<Option<CancellationToken>>,
    /// The windows shown with `ferro_show` that are open: held here until they close, so that a
    /// caller can release their handles once they are shown.
    open: RefCell<Vec<Ref<Window>>>,
}

ferroui_base::ferro_thread_local! {
    static APP: AppState = AppState::default();
}

fn stop_loop() {
    let stop = APP.with(|app| app.stop.borrow().clone());
    if let Some(stop) = stop {
        stop.cancel();
    }
}

/// The window of a handle.
fn window(handle: FerroHandle) -> Result<Ref<Window>> {
    thread::check()?;
    let object = handles::get(handle)?;
    object
        .cast::<Window>()
        .ok_or_else(|| Error::new(FERRO_TYPE_MISMATCH, format!("a {} is not a window", object.get_type().name())))
}

/// Shows the window `window` ([`ferro_create`](crate::ferro_create) of the type `Window`). While
/// it is open the library holds it (its handle can be released), and the main loop of [`ferro_app_run`] runs until the last
/// window shown this way closes. [`FERRO_INVALID_STATE`] before `ferro_app_run` has started.
/// UI thread only.
#[no_mangle]
pub extern "C" fn ferro_show(window: FerroHandle) -> FerroStatus {
    status(|| {
        let window = self::window(window)?;
        if !APP.with(|app| app.started.get()) {
            return Err(Error::new(FERRO_INVALID_STATE, "a window is shown once the application runs (ferro_app_run)"));
        }
        let known = APP.with(|app| app.open.borrow().iter().any(|open| open.ptr_eq(&window)));
        if !known {
            APP.with(|app| app.open.borrow_mut().push(window.clone()));
            let weak = window.downgrade();
            window
                .closed(move || {
                    let (closed, empty) = APP.with(|app| {
                        let mut open = app.open.borrow_mut();
                        let index = open.iter().position(|open| weak.points_to(open));
                        let closed = index.map(|index| open.remove(index));
                        (closed, open.is_empty())
                    });
                    // Dropped with no borrow of the state held.
                    drop(closed);
                    if empty {
                        stop_loop();
                    }
                })
                .detach();
        }
        ferroui_controls::WindowBaseImpl::show(&*window);
        Ok(())
    })
}

/// Closes the window `window`, as its close button does. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_close(window: FerroHandle) -> FerroStatus {
    status(|| {
        self::window(window)?.close();
        Ok(())
    })
}

/// Ends the main loop of [`ferro_app_run`] once the work at hand is done; the open windows are
/// left as they are. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_quit() -> FerroStatus {
    status(|| {
        thread::check()?;
        stop_loop();
        Ok(())
    })
}

/// Runs an application on the platform of the operating system (macOS, Windows or Linux, with
/// the Fluent theme): makes the calling thread the UI thread ([`ferro_init`](crate::ferro_init);
/// on macOS it must be the main thread), sets the application up, calls `on_start(token)`, and
/// runs the main loop until the last window shown with [`ferro_show`] closes or [`ferro_quit`]
/// is called; then returns [`FERRO_OK`](crate::FERRO_OK). `on_start` creates the windows and shows
/// them; if it returns a value other than `0` the loop does not run and the result is
/// [`FERRO_START_FAILED`](crate::FERRO_START_FAILED). `options` is reserved and must be `NULL` in
/// ABI version 1. Runs once per process ([`FERRO_INVALID_STATE`] the second time).
///
/// With the environment variable `FERROUI_SMOKE_EXIT_MS=<n>` the windows close after `n`
/// milliseconds, as for every sample of the repository (the smoke runs of continuous
/// integration).
///
/// A build without the desktop platform (the feature `desktop` of `ferroui-capi`) returns
/// [`FERRO_NOT_SUPPORTED`](crate::FERRO_NOT_SUPPORTED).
#[no_mangle]
pub extern "C" fn ferro_app_run(options: *const c_void, on_start: FerroStartFn, token: i64) -> FerroStatus {
    status(|| {
        if !options.is_null() {
            return Err(Error::new(FERRO_INVALID_ARGUMENT, "the options are reserved: pass NULL"));
        }
        let on_start =
            on_start.ok_or_else(|| Error::new(crate::FERRO_NULL_POINTER, "the start function is NULL"))?;
        thread::init()?;
        if APP.with(|app| app.started.replace(true)) {
            return Err(Error::new(FERRO_INVALID_STATE, "the application runs once per process"));
        }
        run(on_start, token)
    })
}

#[cfg(feature = "desktop")]
fn run(on_start: unsafe extern "C" fn(i64) -> i64, token: i64) -> Result<()> {
    use ferroui_controls::{AppBuilder, Application};
    use ferroui_desktop::AppBuilderDesktopExtensions;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut result = Ok(());
    AppBuilder::configure::<Application>().use_platform_detect().start(
        |app, _| {
            app.styles().add(ferroui_themes_fluent::FluentTheme::new().as_style());
            let stop = CancellationToken::new();
            APP.with(|state| *state.stop.borrow_mut() = Some(stop.clone()));
            // SAFETY: the caller gave a function of the `FerroStartFn` signature.
            let started = unsafe { on_start(token) };
            if started != 0 {
                result = Err(Error::new(
                    crate::FERRO_START_FAILED,
                    format!("the start function returned {started}"),
                ));
                return;
            }
            smoke_exit();
            app.run_with_token(&stop);
        },
        &args,
    );
    APP.with(|state| state.stop.borrow_mut().take());
    result
}

/// With `FERROUI_SMOKE_EXIT_MS=<n>`, the shown windows close after `n` milliseconds and the loop
/// ends.
#[cfg(feature = "desktop")]
fn smoke_exit() {
    use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
    let Some(ms) = std::env::var("FERROUI_SMOKE_EXIT_MS").ok().and_then(|value| value.parse::<u64>().ok()) else {
        return;
    };
    DispatcherTimer::run_once(
        || {
            let open: Vec<Ref<Window>> = APP.with(|app| app.open.borrow().clone());
            for window in open {
                window.close();
            }
            stop_loop();
        },
        std::time::Duration::from_millis(ms),
        DispatcherPriority::NORMAL,
    )
    .detach();
}

#[cfg(not(feature = "desktop"))]
fn run(on_start: unsafe extern "C" fn(i64) -> i64, token: i64) -> Result<()> {
    let _ = (on_start, token);
    APP.with(|app| app.started.set(false));
    Err(Error::new(
        crate::FERRO_NOT_SUPPORTED,
        "this build has no desktop platform: build ferroui-capi with the feature desktop",
    ))
}
