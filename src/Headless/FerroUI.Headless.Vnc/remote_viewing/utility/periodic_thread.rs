//! Port of `Utility/PeriodicThread.cs` of RemoteViewing (BSD-2-Clause,
//! Copyright (c) 2013 James F. Bellinger; `NOTICE.md` of this crate).

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// The two wait handles of the original: `requestUpdate` (an auto reset
/// event) and `requestExit` (a manual reset event), under one lock so that
/// the thread can wait for either.
#[derive(Default)]
struct Requests {
    update: bool,
    exit: bool,
}

#[derive(Default)]
struct Signals {
    requests: Mutex<Requests>,
    changed: Condvar,
}

impl Signals {
    fn requests(&self) -> MutexGuard<'_, Requests> {
        self.requests.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// `WaitHandle.WaitAny(requestUpdate, requestExit)`: 0 when the update
    /// was requested (and resets it), 1 when the exit was.
    fn wait_any(&self) -> usize {
        let mut requests = self.requests();
        loop {
            if requests.update {
                requests.update = false;
                return 0;
            }
            if requests.exit {
                return 1;
            }
            requests = self
                .changed
                .wait(requests)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// `requestExit.WaitOne(timeout)`: whether the exit was requested
    /// within `timeout`.
    fn wait_for_exit(&self, timeout: Duration) -> bool {
        let requests = self.requests();
        let (requests, _) = self
            .changed
            .wait_timeout_while(requests, timeout, |requests| !requests.exit)
            .unwrap_or_else(PoisonError::into_inner);
        requests.exit
    }

    fn set_update(&self) {
        self.requests().update = true;
        self.changed.notify_all();
    }

    fn set_exit(&self) {
        self.requests().exit = true;
        self.changed.notify_all();
    }
}

/// Represents a thread that invokes a specific action at a specific
/// interval.
#[derive(Default)]
pub(crate) struct PeriodicThread {
    signals: Mutex<Option<Arc<Signals>>>,
    request_thread: Mutex<Option<JoinHandle<()>>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl PeriodicThread {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Invokes the `action` at a frequency specified by
    /// `get_update_rate_func` (in Hz); optionally (`use_signal`) waiting for
    /// a signal ([`signal`](Self::signal)) to execute the action.
    ///
    /// The thread ends when the action panics (the exception of the
    /// original) or when the thread is stopped.
    pub(crate) fn start(
        &self,
        action: impl Fn() + Send + 'static,
        get_update_rate_func: impl Fn() -> f64 + Send + 'static,
        use_signal: bool,
    ) {
        let signals = Arc::new(Signals::default());
        *lock(&self.signals) = Some(signals.clone());

        let request_thread = thread::Builder::new()
            .name("VNC framebuffer updates".to_owned())
            .spawn(move || loop {
                let start_time = Instant::now();
                if use_signal && signals.wait_any() == 1 {
                    return;
                }

                // Boundary (#567): the top of the update thread; as in the reference, an exception
                // of the action ends the thread instead of taking the process down.
                if catch_unwind(AssertUnwindSafe(&action)).is_err() {
                    return;
                }

                let elapsed_time = start_time.elapsed().as_secs_f64();
                let seconds_to_wait = (1.0 / get_update_rate_func()) - elapsed_time;
                let timeout = (1000.0 * seconds_to_wait).round().clamp(0.0, 60000.0) as u64;
                if timeout > 0 && signals.wait_for_exit(Duration::from_millis(timeout)) {
                    return;
                }
            })
            .expect("the thread of the framebuffer updates starts");
        *lock(&self.request_thread) = Some(request_thread);
    }

    /// Signals the thread that an action should be performed.
    pub(crate) fn signal(&self) {
        if let Some(signals) = lock(&self.signals).as_ref() {
            signals.set_update();
        }
    }

    /// Stops this thread and waits for it to end.
    pub(crate) fn stop(&self) {
        let Some(request_thread) = lock(&self.request_thread).take() else {
            return;
        };

        if let Some(signals) = lock(&self.signals).as_ref() {
            signals.set_exit();
        }
        if request_thread.thread().id() != thread::current().id() {
            let _ = request_thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream.
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn the_action_runs_periodically_until_the_thread_is_stopped() {
        let count = Arc::new(AtomicUsize::new(0));
        let periodic = PeriodicThread::new();
        let counter = count.clone();
        periodic.start(
            move || {
                counter.fetch_add(1, Ordering::SeqCst);
            },
            || 100.0,
            false,
        );

        thread::sleep(Duration::from_millis(200));
        periodic.stop();
        let after_stop = count.load(Ordering::SeqCst);
        assert!(after_stop >= 2, "ran {after_stop} times");

        thread::sleep(Duration::from_millis(50));
        assert_eq!(after_stop, count.load(Ordering::SeqCst));
    }

    #[test]
    fn with_a_signal_the_action_waits_for_it() {
        let count = Arc::new(AtomicUsize::new(0));
        let periodic = PeriodicThread::new();
        let counter = count.clone();
        periodic.start(
            move || {
                counter.fetch_add(1, Ordering::SeqCst);
            },
            || 1000.0,
            true,
        );

        thread::sleep(Duration::from_millis(50));
        assert_eq!(0, count.load(Ordering::SeqCst));

        periodic.signal();
        thread::sleep(Duration::from_millis(50));
        assert_eq!(1, count.load(Ordering::SeqCst));

        periodic.stop();
    }

    #[test]
    fn a_panic_of_the_action_ends_the_thread() {
        let periodic = PeriodicThread::new();
        periodic.start(|| panic!("the action failed"), || 1000.0, false);
        // Joins the thread that has ended.
        periodic.stop();
    }
}
