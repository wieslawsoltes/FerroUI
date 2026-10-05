use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::{IRenderTimer, RenderTimerTick};
use crate::reactive::IDisposable;

/// The running timer returned by [`DefaultRenderTimerImpl::start_core`];
/// disposing it stops the ticks. It is shared between threads, unlike the
/// `Rc`-based disposables of the object model.
pub type RenderTimerSubscription = Arc<dyn IDisposable + Send + Sync>;

/// The overridable members of [`DefaultRenderTimer`].
pub trait DefaultRenderTimerImpl: Send + Sync {
    /// Indicates if the timer ticks on a non-UI thread.
    fn runs_in_background(&self) -> bool {
        true
    }

    /// Provides the implementation of starting the timer.
    ///
    /// `tick` is the method to call on each tick. This can be overridden by
    /// platform implementations to use a specialized timer implementation.
    fn start_core(&self, timer: &DefaultRenderTimer, tick: RenderTimerTick) -> RenderTimerSubscription;
}

/// Defines a default render timer that uses a standard timer.
///
/// Platform implementations can supply their own
/// [`DefaultRenderTimerImpl`] to use a specialized timer implementation.
pub struct DefaultRenderTimer {
    /// Shared with the callback handed to `start_core`.
    tick: Arc<Mutex<Option<RenderTimerTick>>>,
    subscription: Mutex<Option<RenderTimerSubscription>>,
    frames_per_second: i32,
    overrides: Box<dyn DefaultRenderTimerImpl>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl DefaultRenderTimer {
    /// Initializes a new instance of the [`DefaultRenderTimer`] class that
    /// ticks on a thread of its own.
    ///
    /// `frames_per_second` is the number of frames per second at which the
    /// loop should run.
    #[cfg(not(target_family = "wasm"))]
    pub fn new(frames_per_second: i32) -> Self {
        Self::with_impl(frames_per_second, Box::new(standard_timer::StandardTimerImpl))
    }

    /// Initializes a timer whose overridable members are provided by
    /// `overrides`.
    pub fn with_impl(frames_per_second: i32, overrides: Box<dyn DefaultRenderTimerImpl>) -> Self {
        Self { tick: Arc::new(Mutex::new(None)), subscription: Mutex::new(None), frames_per_second, overrides }
    }

    /// Gets the number of frames per second at which the loop runs.
    pub fn frames_per_second(&self) -> i32 {
        self.frames_per_second
    }

    fn internal_tick(&self) -> RenderTimerTick {
        let slot = self.tick.clone();
        Arc::new(move |tick_count| {
            let tick = lock(&slot).clone();
            if let Some(tick) = tick {
                tick(tick_count);
            }
        })
    }
}

impl IRenderTimer for DefaultRenderTimer {
    fn tick(&self) -> Option<RenderTimerTick> {
        lock(&self.tick).clone()
    }

    fn set_tick(&self, value: Option<RenderTimerTick>) {
        let mut subscription = lock(&self.subscription);
        match value {
            Some(value) => {
                *lock(&self.tick) = Some(value);
                if subscription.is_none() {
                    *subscription = Some(self.overrides.start_core(self, self.internal_tick()));
                }
            }
            None => {
                if let Some(subscription) = subscription.take() {
                    subscription.dispose();
                }
                *lock(&self.tick) = None;
            }
        }
    }

    fn runs_in_background(&self) -> bool {
        self.overrides.runs_in_background()
    }
}

#[cfg(not(target_family = "wasm"))]
mod standard_timer {
    use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};
    use std::thread;
    use std::time::{Duration, Instant};

    use super::{DefaultRenderTimer, DefaultRenderTimerImpl, RenderTimerSubscription};
    use crate::reactive::IDisposable;
    use crate::rendering::RenderTimerTick;

    /// The time since the first use of a standard timer in this process, in
    /// whole milliseconds.
    fn tick_count() -> Duration {
        static START: OnceLock<Instant> = OnceLock::new();
        Duration::from_millis(START.get_or_init(Instant::now).elapsed().as_millis() as u64)
    }

    struct PeriodicTimer {
        stopped: Mutex<bool>,
        wakeup: Condvar,
    }

    impl IDisposable for PeriodicTimer {
        fn dispose(&self) {
            *self.stopped.lock().unwrap_or_else(PoisonError::into_inner) = true;
            self.wakeup.notify_all();
        }
    }

    /// The base behaviour: a periodic timer on a thread of its own.
    pub(super) struct StandardTimerImpl;

    impl DefaultRenderTimerImpl for StandardTimerImpl {
        fn start_core(&self, timer: &DefaultRenderTimer, tick: RenderTimerTick) -> RenderTimerSubscription {
            let interval = match Duration::try_from_secs_f64(1.0 / f64::from(timer.frames_per_second())) {
                Ok(interval) => interval,
                Err(_) => panic!("frames_per_second must be a positive number."),
            };

            let state = Arc::new(PeriodicTimer { stopped: Mutex::new(false), wakeup: Condvar::new() });
            let thread_state = state.clone();
            // Start the clock before the first tick.
            tick_count();
            thread::Builder::new()
                .name("RenderTimer".to_string())
                .spawn(move || {
                    let mut due = Instant::now() + interval;
                    loop {
                        {
                            let mut stopped = thread_state.stopped.lock().unwrap_or_else(PoisonError::into_inner);
                            loop {
                                if *stopped {
                                    return;
                                }
                                let now = Instant::now();
                                if now >= due {
                                    break;
                                }
                                stopped = thread_state
                                    .wakeup
                                    .wait_timeout(stopped, due - now)
                                    .unwrap_or_else(PoisonError::into_inner)
                                    .0;
                            }
                        }

                        tick(tick_count());

                        due += interval;
                        let now = Instant::now();
                        if due < now {
                            // The callback took longer than the period.
                            due = now;
                        }
                    }
                })
                .expect("failed to start the render timer thread");
            state
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicI32, Ordering};
    use std::time::Duration;

    #[derive(Default)]
    struct Recorded {
        starts: AtomicI32,
        disposals: AtomicI32,
        core_tick: Mutex<Option<RenderTimerTick>>,
    }

    struct RecordingImpl(Arc<Recorded>);

    struct RecordingSubscription(Arc<Recorded>);

    impl IDisposable for RecordingSubscription {
        fn dispose(&self) {
            self.0.disposals.fetch_add(1, Ordering::SeqCst);
        }
    }

    impl DefaultRenderTimerImpl for RecordingImpl {
        fn runs_in_background(&self) -> bool {
            false
        }

        fn start_core(&self, timer: &DefaultRenderTimer, tick: RenderTimerTick) -> RenderTimerSubscription {
            assert_eq!(timer.frames_per_second(), 30);
            self.0.starts.fetch_add(1, Ordering::SeqCst);
            *lock(&self.0.core_tick) = Some(tick);
            Arc::new(RecordingSubscription(self.0.clone()))
        }
    }

    #[test]
    fn setting_tick_starts_once_and_clearing_it_disposes_the_subscription() {
        let recorded = Arc::new(Recorded::default());
        let timer = DefaultRenderTimer::with_impl(30, Box::new(RecordingImpl(recorded.clone())));
        assert!(!timer.runs_in_background());
        assert!(timer.tick().is_none());

        let first = Arc::new(AtomicI32::new(0));
        let second = Arc::new(AtomicI32::new(0));
        let f = first.clone();
        timer.set_tick(Some(Arc::new(move |time| {
            assert_eq!(time, Duration::from_millis(7));
            f.fetch_add(1, Ordering::SeqCst);
        })));
        assert_eq!(recorded.starts.load(Ordering::SeqCst), 1);
        assert!(timer.tick().is_some());

        let core_tick = lock(&recorded.core_tick).clone().unwrap();
        core_tick(Duration::from_millis(7));
        assert_eq!(first.load(Ordering::SeqCst), 1);

        // Replacing the callback keeps the running subscription.
        let s = second.clone();
        timer.set_tick(Some(Arc::new(move |_| {
            s.fetch_add(1, Ordering::SeqCst);
        })));
        assert_eq!(recorded.starts.load(Ordering::SeqCst), 1);
        core_tick(Duration::from_millis(7));
        assert_eq!((first.load(Ordering::SeqCst), second.load(Ordering::SeqCst)), (1, 1));

        timer.set_tick(None);
        assert_eq!(recorded.disposals.load(Ordering::SeqCst), 1);
        assert!(timer.tick().is_none());
        // A late tick of the stopped timer goes nowhere.
        core_tick(Duration::from_millis(7));
        assert_eq!(second.load(Ordering::SeqCst), 1);

        timer.set_tick(None);
        assert_eq!(recorded.disposals.load(Ordering::SeqCst), 1);

        timer.set_tick(Some(Arc::new(|_| {})));
        assert_eq!(recorded.starts.load(Ordering::SeqCst), 2);
    }

    #[cfg(not(target_family = "wasm"))]
    #[test]
    fn standard_timer_ticks_in_the_background_until_stopped() {
        let timer = DefaultRenderTimer::new(500);
        assert!(timer.runs_in_background());
        assert_eq!(timer.frames_per_second(), 500);

        let (sender, receiver) = std::sync::mpsc::channel();
        let sender = Mutex::new(sender);
        timer.set_tick(Some(Arc::new(move |time| {
            let _ = lock(&sender).send(time);
        })));

        let first = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        let second = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(second >= first);

        timer.set_tick(None);
        // Clearing the callback drops it, which closes the channel.
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            match receiver.recv_timeout(Duration::from_millis(50)) {
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                _ => assert!(std::time::Instant::now() < deadline, "the timer thread did not stop"),
            }
        }
    }
}
