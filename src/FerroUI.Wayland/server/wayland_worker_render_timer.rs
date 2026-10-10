//! The render timer of the worker (the port of
//! `WaylandWorker.RenderTimer.cs`).
//!
//! Since Wayland asks us nicely to NOT render when we want to, but instead
//! tells us when to, the render timer is not an actual timer, but something
//! that gets triggered by frame callbacks. So if we expect the render timer
//! to do something useful for e. g. a new surface, we need to wake it up
//! explicitly. For inevitable oversights there is a "fallback" timer that
//! ticks at 20FPS.
//!
//! The reference has this as a part of the worker class. Here it is a value
//! the worker owns, which reaches the rest of the worker through two
//! functions (set the wake-up descriptor; post a job to the queue), so that
//! it runs in tests without a connection.

use super::server_signaler::{ServerSignaler, SignalJob};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::rendering::{IRenderLoop, IRenderLoopTask};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// The target FPS for UI thread animations when Wayland compositor doesn't think
/// that we should be rendering yet
const THROTTLED_UI_THREAD_FPS: u32 = 20;

/// The interval after which a render loop that did not tick is woken.
pub const RENDER_LOOP_STARVATION_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / THROTTLED_UI_THREAD_FPS as u64);

/// The render loop of the worker: its tasks are rendered by the worker
/// thread whenever a tick is pending.
pub struct RenderLoopImpl {
    tasks: Mutex<Vec<Arc<dyn IRenderLoopTask>>>,
    in_tick: AtomicBool,
    wakeup_callback: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}

impl RenderLoopImpl {
    fn new() -> Self {
        Self { tasks: Mutex::new(Vec::new()), in_tick: AtomicBool::new(false), wakeup_callback: Mutex::new(None) }
    }

    /// Renders every task once. A tick that begins while another runs does nothing.
    pub fn do_tick(&self) {
        if self.in_tick.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire).is_err() {
            return;
        }
        // The flag is cleared when the tick ends, also by a panic of a task.
        struct InTick<'a>(&'a AtomicBool);
        impl Drop for InTick<'_> {
            fn drop(&mut self) {
                self.0.store(false, Ordering::Release);
            }
        }
        let _in_tick = InTick(&self.in_tick);

        let tasks_copy: Vec<Arc<dyn IRenderLoopTask>> = self.tasks.lock().unwrap_or_else(PoisonError::into_inner).clone();
        for task in &tasks_copy {
            // Whether the task wants another tick is not asked: a surface is rendered again
            // when its frame callback arrives, and the starvation timer covers the rest.
            let _ = task.render();
        }
    }
}

impl IRenderLoop for RenderLoopImpl {
    fn add(&self, i: Arc<dyn IRenderLoopTask>) {
        self.tasks.lock().unwrap_or_else(PoisonError::into_inner).push(i);
        self.wakeup();
    }

    fn remove(&self, i: &Arc<dyn IRenderLoopTask>) {
        let mut tasks = self.tasks.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(index) = tasks.iter().position(|task| Arc::ptr_eq(task, i)) {
            tasks.remove(index);
        }
    }

    fn runs_in_background(&self) -> bool {
        true
    }

    fn wakeup(&self) {
        let callback = self.wakeup_callback.lock().unwrap_or_else(PoisonError::into_inner).clone();
        if let Some(callback) = callback {
            callback();
        }
    }

    fn request_frame_out_of_turn(&self) {
        // A frame is a tick of the worker: the same wake-up.
        self.wakeup();
    }
}

/// What the starvation timer and the threads share.
struct Starvation {
    state: Mutex<StarvationState>,
    changed: Condvar,
}

struct StarvationState {
    /// When the render loop was last seen starved, on the clock of the timer.
    starved_since: Option<Duration>,
    /// Whether the timer is enabled (`Timer.Enabled`).
    enabled: bool,
    /// The worker is gone.
    shutdown: bool,
}

/// The render timer of a worker.
pub struct WaylandRenderTimer {
    render_loop_wakeup_pending: Arc<AtomicBool>,
    render_loop_wakeup_signaler: ServerSignaler,
    clock: Instant,
    starvation: Arc<Starvation>,
    render_loop: Arc<RenderLoopImpl>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

impl WaylandRenderTimer {
    /// `wake` sets the wake-up descriptor of the worker; `post_oob` puts a job into its
    /// out-of-band queue (and wakes it).
    pub fn new(wake: Arc<dyn Fn() + Send + Sync>, post_oob: impl Fn(SignalJob) + Send + Sync + 'static) -> Self {
        let render_loop_wakeup_pending = Arc::new(AtomicBool::new(false));
        let render_loop = Arc::new(RenderLoopImpl::new());

        // InitRenderTimer
        {
            let pending = render_loop_wakeup_pending.clone();
            let wake = wake.clone();
            *render_loop.wakeup_callback.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(move || {
                pending.store(true, Ordering::Release);
                wake();
            }));
        }

        let render_loop_wakeup_signaler = {
            let pending = render_loop_wakeup_pending.clone();
            ServerSignaler::new(post_oob, move || pending.store(true, Ordering::Release))
        };

        let starvation = Arc::new(Starvation {
            state: Mutex::new(StarvationState { starved_since: None, enabled: false, shutdown: false }),
            changed: Condvar::new(),
        });

        Self { render_loop_wakeup_pending, render_loop_wakeup_signaler, clock: Instant::now(), starvation, render_loop, wake }
    }

    /// Starts the thread of the starvation timer (`System.Timers.Timer` of the reference,
    /// which raises its event on a thread of the pool).
    pub fn start_starvation_timer(self: &Arc<Self>) {
        let starvation = self.starvation.clone();
        let this = Arc::downgrade(self);
        let spawned = std::thread::Builder::new().name("FerroWaylandRenderStarvation".to_string()).spawn(move || loop {
            {
                let mut state = starvation.state.lock().unwrap_or_else(PoisonError::into_inner);
                while !state.enabled && !state.shutdown {
                    state = starvation.changed.wait(state).unwrap_or_else(PoisonError::into_inner);
                }
                if state.shutdown {
                    return;
                }
                let (state, _) = starvation
                    .changed
                    .wait_timeout(state, RENDER_LOOP_STARVATION_INTERVAL)
                    .unwrap_or_else(PoisonError::into_inner);
                if state.shutdown {
                    return;
                }
                if !state.enabled {
                    continue;
                }
            }
            match this.upgrade() {
                Some(this) => this.on_render_loop_starved(),
                None => return,
            }
        });
        if let Err(error) = spawned {
            if let Some(logger) = Logger::try_get(LogEventLevel::Error, "Wayland") {
                logger.log_with_values(None, "Unable to start the render loop starvation timer: {Error}", &[&error]);
            }
        }
    }

    pub fn render_loop(&self) -> &Arc<RenderLoopImpl> {
        &self.render_loop
    }

    /// Asks for a tick of the render loop at the end of the current iteration of the worker.
    /// Sets a flag only: for the worker thread, or with a wake-up of its own.
    pub fn wakeup_render_loop(&self) {
        self.render_loop_wakeup_pending.store(true, Ordering::Release);
    }

    /// Asks for a tick of the render loop from any thread: through the queue of the worker.
    pub fn any_thread_wakeup_render_loop(&self) {
        self.render_loop_wakeup_signaler.signal();
    }

    /// What the worker does after every commit of the compositor of the UI thread
    /// (`Compositor.AfterCommit`). `had_pending_server_jobs` says whether jobs were posted
    /// with the batch.
    pub fn on_after_commit(&self, had_pending_server_jobs: bool) {
        if had_pending_server_jobs {
            self.any_thread_wakeup_render_loop();
        }

        {
            let mut state = self.starvation.state.lock().unwrap_or_else(PoisonError::into_inner);
            if state.starved_since.is_none() {
                state.starved_since = Some(self.clock.elapsed());
                state.enabled = true;
                self.starvation.changed.notify_all();
            }
        }
        (self.wake)();
    }

    fn on_render_loop_starved(&self) {
        let starved = {
            let state = self.starvation.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.starved_since.is_some_and(|since| since + RENDER_LOOP_STARVATION_INTERVAL < self.clock.elapsed())
        };
        if starved {
            self.any_thread_wakeup_render_loop();
        }
    }

    /// Whether a tick is pending.
    pub fn is_wakeup_pending(&self) -> bool {
        self.render_loop_wakeup_pending.load(Ordering::Acquire)
    }

    /// Ticks the render loop when a tick is pending. On the worker thread, with nothing of
    /// the worker borrowed: the tasks reach the worker's state.
    pub fn tick_render_loop_if_needed(&self) {
        if !self.render_loop_wakeup_pending.swap(false, Ordering::AcqRel) {
            return;
        }

        {
            let mut state = self.starvation.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.starved_since = None;
            state.enabled = false;
        }

        // The reference catches every exception of a tick, logs it and asks for another
        // tick; a panic of a task is the counterpart.
        let render_loop = self.render_loop.clone();
        if let Err(payload) = catch_unwind(AssertUnwindSafe(move || render_loop.do_tick())) {
            self.any_thread_wakeup_render_loop();
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|message| (*message).to_string()))
                .unwrap_or_else(|| "a panic without a message".to_string());
            if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
                logger.log_with_values(None, "Exception in render loop: {Error}", &[&message]);
            }
        }
    }
}

impl Drop for WaylandRenderTimer {
    fn drop(&mut self) {
        let mut state = self.starvation.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.shutdown = true;
        self.starvation.changed.notify_all();
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct Task {
        renders: AtomicUsize,
    }

    impl IRenderLoopTask for Task {
        fn render(&self) -> bool {
            self.renders.fetch_add(1, Ordering::SeqCst);
            false
        }
    }

    struct Fixture {
        timer: Arc<WaylandRenderTimer>,
        wakes: Arc<AtomicUsize>,
        queue: Arc<Mutex<Vec<SignalJob>>>,
    }

    impl Fixture {
        fn new() -> Self {
            let wakes = Arc::new(AtomicUsize::new(0));
            let queue: Arc<Mutex<Vec<SignalJob>>> = Arc::new(Mutex::new(Vec::new()));
            let timer = {
                let wakes = wakes.clone();
                let queue = queue.clone();
                Arc::new(WaylandRenderTimer::new(
                    Arc::new(move || {
                        wakes.fetch_add(1, Ordering::SeqCst);
                    }),
                    move |job| queue.lock().unwrap().push(job),
                ))
            };
            Self { timer, wakes, queue }
        }

        fn run_queue(&self) -> usize {
            let jobs: Vec<SignalJob> = std::mem::take(&mut *self.queue.lock().unwrap());
            let count = jobs.len();
            for job in jobs {
                job();
            }
            count
        }
    }

    #[test]
    fn adding_a_task_wakes_the_loop_and_a_tick_renders_it_once() {
        let fixture = Fixture::new();
        let task = Arc::new(Task { renders: AtomicUsize::new(0) });
        let as_task: Arc<dyn IRenderLoopTask> = task.clone();

        assert!(fixture.timer.render_loop().runs_in_background());
        fixture.timer.render_loop().add(as_task.clone());
        assert_eq!(fixture.wakes.load(Ordering::SeqCst), 1);
        assert!(fixture.timer.is_wakeup_pending());

        fixture.timer.tick_render_loop_if_needed();
        assert_eq!(task.renders.load(Ordering::SeqCst), 1);
        assert!(!fixture.timer.is_wakeup_pending());

        // Nothing pending: no tick.
        fixture.timer.tick_render_loop_if_needed();
        assert_eq!(task.renders.load(Ordering::SeqCst), 1);

        fixture.timer.render_loop().remove(&as_task);
        fixture.timer.wakeup_render_loop();
        fixture.timer.tick_render_loop_if_needed();
        assert_eq!(task.renders.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_wake_up_from_any_thread_goes_through_the_queue_of_the_worker() {
        let fixture = Fixture::new();
        fixture.timer.any_thread_wakeup_render_loop();
        fixture.timer.any_thread_wakeup_render_loop();
        assert!(!fixture.timer.is_wakeup_pending());
        assert_eq!(fixture.run_queue(), 1);
        assert!(fixture.timer.is_wakeup_pending());
    }

    #[test]
    fn a_commit_with_jobs_wakes_the_loop_and_every_commit_wakes_the_worker() {
        let fixture = Fixture::new();
        fixture.timer.on_after_commit(false);
        assert_eq!(fixture.wakes.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.run_queue(), 0);

        fixture.timer.on_after_commit(true);
        assert_eq!(fixture.wakes.load(Ordering::SeqCst), 2);
        assert_eq!(fixture.run_queue(), 1);
        assert!(fixture.timer.is_wakeup_pending());
    }

    #[test]
    fn a_loop_that_does_not_tick_after_a_commit_is_woken_by_the_timer() {
        let fixture = Fixture::new();
        fixture.timer.start_starvation_timer();
        fixture.timer.on_after_commit(false);

        // The timer fires after the interval and finds the loop starved for longer than it
        // at its second firing at the latest.
        let deadline = Instant::now() + Duration::from_secs(10);
        while fixture.queue.lock().unwrap().is_empty() {
            assert!(Instant::now() < deadline, "the starvation timer did not wake the loop");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(fixture.run_queue(), 1);
        assert!(fixture.timer.is_wakeup_pending());

        // A tick ends the starvation: the timer is quiet until the next commit.
        fixture.timer.tick_render_loop_if_needed();
        std::thread::sleep(RENDER_LOOP_STARVATION_INTERVAL * 4);
        assert_eq!(fixture.run_queue(), 0);
    }

    #[test]
    fn a_panic_of_a_task_is_caught_and_asks_for_another_tick() {
        struct Panics;
        impl IRenderLoopTask for Panics {
            fn render(&self) -> bool {
                panic!("a task failed")
            }
        }

        let fixture = Fixture::new();
        fixture.timer.render_loop().add(Arc::new(Panics));
        fixture.timer.tick_render_loop_if_needed();
        assert_eq!(fixture.run_queue(), 1);
        assert!(fixture.timer.is_wakeup_pending());
        // The loop can tick again: the tick that panicked did not leave it marked as running.
        fixture.timer.tick_render_loop_if_needed();
        assert_eq!(fixture.run_queue(), 1);
    }
}
