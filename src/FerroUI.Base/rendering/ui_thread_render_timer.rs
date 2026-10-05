use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use super::{
    DefaultRenderTimer, DefaultRenderTimerImpl, IRenderTimer, RenderTimerSubscription, RenderTimerTick,
};
use crate::reactive::IDisposable;
use crate::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};

/// The stopwatch of the timer, driven by the clock of the dispatcher the
/// timer ticks on. It starts the first time the timer is started.
#[derive(Default)]
struct Clock {
    started_at: Mutex<Option<i64>>,
}

impl Clock {
    fn elapsed(&self, dispatcher: &Dispatcher) -> Duration {
        let now = dispatcher.now();
        let mut started_at = self.started_at.lock().unwrap_or_else(PoisonError::into_inner);
        let started_at = *started_at.get_or_insert(now);
        Duration::from_millis(now.saturating_sub(started_at).max(0) as u64)
    }
}

thread_local! {
    /// The dispatcher timers of the running instances of this thread. They
    /// are `Rc`-based, so they stay on the thread they were started on; the
    /// thread-safe subscription only carries the key.
    static INSTANCES: RefCell<HashMap<u64, Rc<DispatcherTimer>>> = RefCell::new(HashMap::new());
}

static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

const MIN_INTERVAL: Duration = Duration::from_millis(1);

/// Sets the interval of the next tick when the tick callback returns,
/// however it returns.
struct RescheduleOnExit<'a> {
    timer: &'a DispatcherTimer,
    clock: &'a Clock,
    next_tick_at: Duration,
}

impl Drop for RescheduleOnExit<'_> {
    fn drop(&mut self) {
        let after_tick = self.clock.elapsed(self.timer.dispatcher());
        let interval = match self.next_tick_at.checked_sub(after_tick) {
            Some(interval) if interval >= MIN_INTERVAL => interval,
            // We are way overdue, but shouldn't cause starvation in other areas
            _ => MIN_INTERVAL,
        };
        self.timer.set_interval(interval);
    }
}

struct TimerInstance {
    id: u64,
    dispatcher: Arc<Dispatcher>,
}

impl TimerInstance {
    fn new(frames_per_second: i32, clock: Arc<Clock>, tick: RenderTimerTick) -> TimerInstance {
        let timer = DispatcherTimer::with_priority(DispatcherPriority::RENDER);
        let dispatcher = timer.dispatcher().clone();
        // Starts the stopwatch.
        clock.elapsed(&dispatcher);

        let interval = match Duration::try_from_secs_f64(1.0 / f64::from(frames_per_second)) {
            Ok(interval) => interval,
            Err(_) => panic!("frames_per_second must be a positive number."),
        };

        timer.tick(move |timer| {
            let ticked_at = clock.elapsed(timer.dispatcher());
            let _reschedule = RescheduleOnExit { timer, clock: &clock, next_tick_at: ticked_at + interval };
            tick(ticked_at);
        });
        // The first tick is not delayed: the timer starts with a zero
        // interval, the frame interval applies from the first tick on.
        timer.set_interval(Duration::ZERO);
        timer.start();

        let id = NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed);
        INSTANCES.with(|instances| instances.borrow_mut().insert(id, timer));
        TimerInstance { id, dispatcher }
    }

    fn stop(id: u64) {
        let timer = INSTANCES.try_with(|instances| instances.borrow_mut().remove(&id)).ok().flatten();
        if let Some(timer) = timer {
            timer.stop();
        }
    }
}

impl IDisposable for TimerInstance {
    fn dispose(&self) {
        let id = self.id;
        if self.dispatcher.check_access() {
            Self::stop(id);
        } else {
            // The dispatcher timer belongs to the thread it was started on.
            self.dispatcher.post(move || Self::stop(id), DispatcherPriority::SEND);
        }
    }
}

struct UiThreadRenderTimerImpl {
    clock: Arc<Clock>,
}

impl DefaultRenderTimerImpl for UiThreadRenderTimerImpl {
    fn runs_in_background(&self) -> bool {
        false
    }

    fn start_core(&self, timer: &DefaultRenderTimer, tick: RenderTimerTick) -> RenderTimerSubscription {
        Arc::new(TimerInstance::new(timer.frames_per_second(), self.clock.clone(), tick))
    }
}

/// Render timer that ticks on UI thread. Useful for debugging or
/// bootstrapping on new platforms.
///
/// The timer ticks on the dispatcher of the thread that starts it (the
/// thread that sets the tick callback), which is expected to be the UI
/// thread.
pub struct UiThreadRenderTimer {
    base: DefaultRenderTimer,
}

impl UiThreadRenderTimer {
    /// Initializes a new instance of the [`UiThreadRenderTimer`] class.
    ///
    /// `frames_per_second` is the number of frames per second at which the
    /// loop should run.
    pub fn new(frames_per_second: i32) -> Self {
        Self {
            base: DefaultRenderTimer::with_impl(
                frames_per_second,
                Box::new(UiThreadRenderTimerImpl { clock: Arc::new(Clock::default()) }),
            ),
        }
    }

    /// Gets the number of frames per second at which the loop runs.
    pub fn frames_per_second(&self) -> i32 {
        self.base.frames_per_second()
    }
}

impl IRenderTimer for UiThreadRenderTimer {
    fn tick(&self) -> Option<RenderTimerTick> {
        self.base.tick()
    }

    fn set_tick(&self, value: Option<RenderTimerTick>) {
        self.base.set_tick(value);
    }

    fn runs_in_background(&self) -> bool {
        self.base.runs_in_background()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::threading::{DispatcherImplEvent, IDispatcherImpl};
    use std::sync::atomic::AtomicI64;
    use std::thread::{self, ThreadId};

    /// A dispatcher implementation whose clock and events are driven by the
    /// test.
    struct ManualDispatcherImpl {
        loop_thread: ThreadId,
        now: Arc<AtomicI64>,
        next_timer: Mutex<Option<i64>>,
        signaled: DispatcherImplEvent,
        timer: DispatcherImplEvent,
    }

    impl ManualDispatcherImpl {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                loop_thread: thread::current().id(),
                now: Arc::new(AtomicI64::new(0)),
                next_timer: Mutex::new(None),
                signaled: DispatcherImplEvent::new(),
                timer: DispatcherImplEvent::new(),
            })
        }

        /// The due time of the only dispatcher timer of the thread.
        fn due_time(&self) -> Option<i64> {
            let timers = Dispatcher::snapshot_timers_for_unit_tests();
            assert!(timers.len() <= 1);
            timers.first().map(|timer| timer.due_time_in_ms())
        }

        /// Moves the clock to `now` and reports the timer as due.
        fn fire_timer_at(&self, now: i64) {
            self.now.store(now, Ordering::SeqCst);
            self.timer.raise();
        }
    }

    /// The dispatcher owns its implementation through an `Rc`; the test
    /// also drives the same implementation from tick callbacks.
    struct Forward(Arc<ManualDispatcherImpl>);

    impl IDispatcherImpl for Forward {
        fn current_thread_is_loop_thread(&self) -> bool {
            self.0.current_thread_is_loop_thread()
        }
        fn signal(&self) {
            self.0.signal()
        }
        fn signal_handle(&self) -> std::sync::Arc<dyn crate::threading::IDispatcherSignal> {
            self.0.signal_handle()
        }
        fn signaled(&self) -> &DispatcherImplEvent {
            self.0.signaled()
        }
        fn timer(&self) -> &DispatcherImplEvent {
            self.0.timer()
        }
        fn now(&self) -> i64 {
            self.0.now()
        }
        fn update_timer(&self, due_time_in_ms: Option<i64>) {
            self.0.update_timer(due_time_in_ms)
        }
    }

    impl IDispatcherImpl for ManualDispatcherImpl {
        fn current_thread_is_loop_thread(&self) -> bool {
            thread::current().id() == self.loop_thread
        }

        fn signal(&self) {}

        fn signal_handle(&self) -> std::sync::Arc<dyn crate::threading::IDispatcherSignal> {
            struct NoSignal;
            impl crate::threading::IDispatcherSignal for NoSignal {
                fn signal(&self) {}
            }
            std::sync::Arc::new(NoSignal)
        }

        fn signaled(&self) -> &DispatcherImplEvent {
            &self.signaled
        }

        fn timer(&self) -> &DispatcherImplEvent {
            &self.timer
        }

        fn now(&self) -> i64 {
            self.now.load(Ordering::SeqCst)
        }

        fn update_timer(&self, due_time_in_ms: Option<i64>) {
            *self.next_timer.lock().unwrap() = due_time_in_ms;
        }
    }

    #[test]
    fn ticks_on_the_dispatcher_with_the_dispatcher_clock() {
        let _scope = Dispatcher::unit_test_scope();
        let platform = ManualDispatcherImpl::new();
        let dispatcher = Dispatcher::new(Some(std::rc::Rc::new(Forward(platform.clone()))));
        platform.now.store(1000, Ordering::SeqCst);

        let timer = UiThreadRenderTimer::new(50);
        assert!(!timer.runs_in_background());
        assert_eq!(timer.frames_per_second(), 50);

        let ticks = Arc::new(Mutex::new(Vec::new()));
        let t = ticks.clone();
        let tick_now = platform.now.clone();
        let work = Arc::new(AtomicI64::new(5));
        let w = work.clone();
        timer.set_tick(Some(Arc::new(move |time| {
            t.lock().unwrap().push(time);
            // Rendering takes some time.
            tick_now.fetch_add(w.load(Ordering::SeqCst), Ordering::SeqCst);
        })));
        assert_eq!(DispatcherTimer::active_timers_count(), 1);
        assert!(ticks.lock().unwrap().is_empty());

        // The first tick is immediate and starts the stopwatch.
        dispatcher.run_jobs(None);
        assert_eq!(*ticks.lock().unwrap(), [Duration::ZERO]);
        // 20ms per frame, 5 of which were spent in the tick.
        assert_eq!(platform.due_time(), Some(1005 + 15));

        dispatcher.run_jobs(None);
        assert_eq!(ticks.lock().unwrap().len(), 1);

        platform.fire_timer_at(1021);
        dispatcher.run_jobs(None);
        assert_eq!(*ticks.lock().unwrap(), [Duration::ZERO, Duration::from_millis(21)]);
        assert_eq!(platform.due_time(), Some(1026 + 15));

        // An overdue tick is rescheduled with the minimum interval.
        work.store(100, Ordering::SeqCst);
        platform.fire_timer_at(1042);
        dispatcher.run_jobs(None);
        assert_eq!(ticks.lock().unwrap().len(), 3);
        assert_eq!(platform.due_time(), Some(1142 + 1));

        timer.set_tick(None);
        assert_eq!(DispatcherTimer::active_timers_count(), 0);
        assert_eq!(platform.due_time(), None);
        platform.fire_timer_at(2000);
        dispatcher.run_jobs(None);
        assert_eq!(ticks.lock().unwrap().len(), 3);

        // Restarting ticks immediately again, on the same stopwatch.
        work.store(0, Ordering::SeqCst);
        timer.set_tick(Some(Arc::new({
            let t = ticks.clone();
            move |time| t.lock().unwrap().push(time)
        })));
        dispatcher.run_jobs(None);
        assert_eq!(ticks.lock().unwrap()[3], Duration::from_millis(1000));
        timer.set_tick(None);
        assert_eq!(DispatcherTimer::active_timers_count(), 0);
    }

    #[test]
    fn can_be_stopped_from_another_thread() {
        let _scope = Dispatcher::unit_test_scope();
        let platform = ManualDispatcherImpl::new();
        let dispatcher = Dispatcher::new(Some(std::rc::Rc::new(Forward(platform.clone()))));

        let timer = Arc::new(UiThreadRenderTimer::new(60));
        timer.set_tick(Some(Arc::new(|_| {})));
        assert_eq!(DispatcherTimer::active_timers_count(), 1);

        let background = timer.clone();
        thread::spawn(move || background.set_tick(None)).join().unwrap();
        assert!(timer.tick().is_none());

        // The dispatcher timer is stopped by the job posted to its thread.
        dispatcher.run_jobs(None);
        assert_eq!(DispatcherTimer::active_timers_count(), 0);
    }
}
