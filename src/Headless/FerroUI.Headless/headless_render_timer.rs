//! Port of `HeadlessRenderTimer.cs`.

use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::{
    DefaultRenderTimer, DefaultRenderTimerImpl, IRenderTimer, RenderTimerSubscription, RenderTimerTick,
};
use ferroui_base::threading::{Dispatcher, DispatcherPriority, DispatcherTimer};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

type ForceTick = Arc<dyn Fn() + Send + Sync>;

thread_local! {
    /// The dispatcher timers of the running instances of this thread. They
    /// are `Rc`-based, so they stay on the thread they were started on; the
    /// thread-safe subscription only carries the key (as the UI thread
    /// render timer of the base crate does).
    static INSTANCES: RefCell<HashMap<u64, Rc<DispatcherTimer>>> = RefCell::new(HashMap::new());
}

static NEXT_INSTANCE_ID: AtomicU64 = AtomicU64::new(1);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What `StartCore` returns: stops the dispatcher timer and forgets the
/// forced tick.
struct TimerInstance {
    id: u64,
    dispatcher: Arc<Dispatcher>,
    force_tick: Arc<Mutex<Option<ForceTick>>>,
}

impl TimerInstance {
    fn stop(id: u64) {
        let timer = INSTANCES.try_with(|instances| instances.borrow_mut().remove(&id)).ok().flatten();
        if let Some(timer) = timer {
            timer.stop();
        }
    }
}

impl IDisposable for TimerInstance {
    fn dispose(&self) {
        *lock(&self.force_tick) = None;
        let id = self.id;
        if self.dispatcher.check_access() {
            Self::stop(id);
        } else {
            // The dispatcher timer belongs to the thread it was started on.
            self.dispatcher.post(move || Self::stop(id), DispatcherPriority::SEND);
        }
    }
}

struct HeadlessRenderTimerImpl {
    force_tick: Arc<Mutex<Option<ForceTick>>>,
}

impl DefaultRenderTimerImpl for HeadlessRenderTimerImpl {
    fn runs_in_background(&self) -> bool {
        false
    }

    fn start_core(&self, timer: &DefaultRenderTimer, tick: RenderTimerTick) -> RenderTimerSubscription {
        let st = Instant::now();
        let forced = tick.clone();
        let force_tick: ForceTick = Arc::new(move || forced(st.elapsed()));
        *lock(&self.force_tick) = Some(force_tick);

        let dispatcher_timer = DispatcherTimer::with_priority(DispatcherPriority::UI_THREAD_RENDER);
        dispatcher_timer.set_interval(Duration::from_secs_f64(1.0 / f64::from(timer.frames_per_second())));
        dispatcher_timer.set_tag(Some(Rc::new(String::from("HeadlessRenderTimer"))));
        dispatcher_timer.tick(move |_| tick(st.elapsed()));
        dispatcher_timer.start();

        let dispatcher = dispatcher_timer.dispatcher().clone();
        let id = NEXT_INSTANCE_ID.fetch_add(1, Ordering::Relaxed);
        INSTANCES.with(|instances| instances.borrow_mut().insert(id, dispatcher_timer));

        Arc::new(TimerInstance { id, dispatcher, force_tick: self.force_tick.clone() })
    }
}

/// A render timer implementation for headless environments that uses a
/// `DispatcherTimer` to schedule ticks on the UI thread. Can be controlled
/// with the [`force_tick`](Self::force_tick) method.
pub(crate) struct HeadlessRenderTimer {
    base: DefaultRenderTimer,
    force_tick: Arc<Mutex<Option<ForceTick>>>,
}

impl HeadlessRenderTimer {
    pub(crate) fn new(frames_per_second: i32) -> Self {
        let force_tick = Arc::new(Mutex::new(None));
        Self {
            base: DefaultRenderTimer::with_impl(
                frames_per_second,
                Box::new(HeadlessRenderTimerImpl { force_tick: force_tick.clone() }),
            ),
            force_tick,
        }
    }

    pub(crate) fn force_tick(&self) {
        // The callback is taken out of the lock before it runs: a tick may stop the timer.
        let force_tick = lock(&self.force_tick).clone();
        if let Some(force_tick) = force_tick {
            force_tick();
        }
    }
}

impl IRenderTimer for HeadlessRenderTimer {
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
