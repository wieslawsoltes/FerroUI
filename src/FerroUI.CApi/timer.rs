//! Timers on the UI thread: a C function and an integer token called at an interval, on the
//! dispatcher of the UI thread (the `DispatcherTimer` of the framework), with the time since the
//! previous tick.

use crate::error::{status, status_or_value, Error, Result};
use crate::thread;
use crate::{FerroStatus, FerroTimerFn, FERRO_INVALID_ARGUMENT, FERRO_NOT_FOUND, FERRO_NULL_POINTER};
use ferroui_base::reactive::Subscription;
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Default)]
struct Timers {
    next: i64,
    running: HashMap<i64, Subscription>,
}

impl Drop for Timers {
    fn drop(&mut self) {
        // The thread ends with timers running (the application ended): the dispatcher they would
        // stop is gone with the thread, so they are left as they are.
        for (_, subscription) in self.running.drain() {
            std::mem::forget(subscription);
        }
    }
}

ferroui_base::ferro_thread_local! {
    static TIMERS: RefCell<Timers> = RefCell::new(Timers::default());
}

/// Takes the timer `id` out of the table; its subscription is dropped by the caller, with no
/// borrow of the table held.
fn take(id: i64) -> Option<Subscription> {
    TIMERS.with_borrow_mut(|timers| timers.running.remove(&id))
}

fn start(interval: i64, function: FerroTimerFn, token: i64) -> Result<i64> {
    thread::check()?;
    let function = function.ok_or_else(|| Error::new(FERRO_NULL_POINTER, "the function is NULL"))?;
    let interval = u64::try_from(interval).ok().filter(|interval| *interval > 0).ok_or_else(|| {
        Error::new(FERRO_INVALID_ARGUMENT, format!("the interval is {interval} ms; it is at least 1"))
    })?;
    let id = TIMERS.with_borrow_mut(|timers| {
        timers.next += 1;
        timers.next
    });
    let last = Cell::new(Instant::now());
    let subscription = DispatcherTimer::run(
        move || {
            let now = Instant::now();
            let elapsed = i64::try_from(now.duration_since(last.replace(now)).as_micros()).unwrap_or(i64::MAX);
            // SAFETY: the caller gave a function of the `FerroTimerFn` signature.
            let keep = unsafe { function(token, elapsed) } == 0;
            if !keep {
                // The timer stops itself (`DispatcherTimer::run`); its entry goes after this tick.
                Dispatcher::ui_thread().post(move || drop(take(id)), DispatcherPriority::NORMAL);
            }
            keep
        },
        Duration::from_millis(interval),
        DispatcherPriority::NORMAL,
    );
    TIMERS.with_borrow_mut(|timers| timers.running.insert(id, subscription));
    Ok(id)
}

/// Starts a timer on the UI thread: `function(token, elapsed)` is called every `interval`
/// milliseconds (at least 1), on the UI thread, with the time since the previous tick (or since
/// the start, for the first) in microseconds as `elapsed`. It returns `0` to go on ticking; any
/// other result stops the timer. Returns a positive timer ID for [`ferro_timer_stop`], or a
/// negative status. Timers tick while the main loop of [`ferro_app_run`](crate::ferro_app_run)
/// runs, or in [`ferro_run_jobs`](crate::ferro_run_jobs) once they are due. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_timer_start(interval: i64, function: FerroTimerFn, token: i64) -> i64 {
    status_or_value(|| start(interval, function, token))
}

/// Stops the timer `timer` ([`ferro_timer_start`]): its function is not called again.
/// [`FERRO_NOT_FOUND`] for an ID that is not a running timer (one that stopped itself is gone
/// after its last tick). It may be called from the function of the timer itself. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_timer_stop(timer: i64) -> FerroStatus {
    status(|| {
        thread::check()?;
        let subscription =
            take(timer).ok_or_else(|| Error::new(FERRO_NOT_FOUND, format!("{timer} is not a running timer")))?;
        subscription.dispose();
        Ok(())
    })
}
