use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::Duration;

use super::{IRenderLoop, IRenderLoopTask, IRenderTimer, RenderTimerTick};
use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::threading::Dispatcher;

/// Provides factory methods for creating [`IRenderLoop`] instances.
pub struct RenderLoop;

impl RenderLoop {
    /// Creates an [`IRenderLoop`] from an [`IRenderTimer`].
    pub fn from_timer(timer: Arc<dyn IRenderTimer>) -> Arc<dyn IRenderLoop> {
        DefaultRenderLoop::new(timer)
    }
}

/// The state guarded by the timer lock.
#[derive(Default)]
struct TimerState {
    running: bool,
    wakeup_pending: bool,
}

/// Default implementation of the application render loop.
///
/// The render loop is responsible for advancing the animation timer and
/// updating the scene graph for visible windows. It owns the sleep/wake
/// state machine: setting the timer's tick to a callback to start the timer
/// and to `None` to stop it, under a lock so that timer implementations
/// never see concurrent changes.
pub struct DefaultRenderLoop {
    items: Mutex<Vec<Arc<dyn IRenderLoopTask>>>,
    /// Only used by the tick that owns `in_tick`.
    items_copy: Mutex<Vec<Arc<dyn IRenderLoopTask>>>,
    tick: RenderTimerTick,
    timer: Arc<dyn IRenderTimer>,
    timer_lock: Mutex<TimerState>,
    in_tick: AtomicBool,
    has_items: AtomicBool,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn same_task(a: &Arc<dyn IRenderLoopTask>, b: &Arc<dyn IRenderLoopTask>) -> bool {
    // Reference equality of the objects, whatever vtable the handles carry.
    std::ptr::eq(Arc::as_ptr(a) as *const (), Arc::as_ptr(b) as *const ())
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    if let Some(message) = payload.downcast_ref::<&'static str>() {
        message
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message
    } else {
        "unknown error"
    }
}

/// Clears the in-tick flag when the tick ends, however it ends.
struct InTickGuard<'a>(&'a AtomicBool);

impl Drop for InTickGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl DefaultRenderLoop {
    /// Initializes a new instance of the [`DefaultRenderLoop`] class.
    pub fn new(timer: Arc<dyn IRenderTimer>) -> Arc<DefaultRenderLoop> {
        Arc::new_cyclic(|weak_self: &Weak<DefaultRenderLoop>| {
            let this = weak_self.clone();
            DefaultRenderLoop {
                items: Mutex::new(Vec::new()),
                items_copy: Mutex::new(Vec::new()),
                tick: Arc::new(move |time| {
                    if let Some(this) = this.upgrade() {
                        this.timer_tick(time);
                    }
                }),
                timer,
                timer_lock: Mutex::new(TimerState::default()),
                in_tick: AtomicBool::new(false),
                has_items: AtomicBool::new(false),
            }
        })
    }

    /// The timer that drives the loop.
    pub fn timer(&self) -> &Arc<dyn IRenderTimer> {
        &self.timer
    }

    fn timer_tick(&self, _time: Duration) {
        if self.in_tick.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            let _in_tick = InTickGuard(&self.in_tick);

            if let Err(error) = catch_unwind(AssertUnwindSafe(|| self.timer_tick_core())) {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::VISUAL) {
                    logger.log_with_values(
                        Some(self as &dyn Any),
                        "Exception in render loop: {Error}",
                        &[&panic_message(&*error)],
                    );
                }
            }
        }
    }

    fn timer_tick_core(&self) {
        // Consume any pending wakeup — this tick will process its work.
        // Only wakeups arriving during task execution will keep the timer running.
        // Also drop late ticks that arrive after the timer was stopped.
        {
            let mut state = lock(&self.timer_lock);
            if !state.running {
                return;
            }
            state.wakeup_pending = false;
        }

        let mut items_copy = lock(&self.items_copy);
        {
            let items = lock(&self.items);
            items_copy.clear();
            items_copy.extend(items.iter().cloned());
        }

        let mut wants_next_tick = false;
        for item in items_copy.iter() {
            wants_next_tick |= item.render();
        }

        items_copy.clear();
        drop(items_copy);

        if !wants_next_tick {
            let mut state = lock(&self.timer_lock);
            if !state.running {
                // Already stopped by remove()
            } else if state.wakeup_pending {
                state.wakeup_pending = false;
            } else {
                state.running = false;
                self.timer.set_tick(None);
            }
        }
    }
}

impl IRenderLoop for DefaultRenderLoop {
    fn add(&self, i: Arc<dyn IRenderLoopTask>) {
        Dispatcher::ui_thread().verify_access();

        let should_start = {
            let mut items = lock(&self.items);
            items.push(i);
            items.len() == 1
        };

        if should_start {
            self.has_items.store(true, Ordering::SeqCst);
            self.wakeup();
        }
    }

    fn remove(&self, i: &Arc<dyn IRenderLoopTask>) {
        Dispatcher::ui_thread().verify_access();

        let should_stop = {
            let mut items = lock(&self.items);
            if let Some(index) = items.iter().position(|item| same_task(item, i)) {
                items.remove(index);
            }
            items.is_empty()
        };

        if should_stop {
            self.has_items.store(false, Ordering::SeqCst);
            let mut state = lock(&self.timer_lock);
            if state.running {
                state.running = false;
                state.wakeup_pending = false;
                self.timer.set_tick(None);
            }
        }
    }

    fn runs_in_background(&self) -> bool {
        self.timer.runs_in_background()
    }

    fn wakeup(&self) {
        let mut state = lock(&self.timer_lock);
        if self.has_items.load(Ordering::SeqCst) && !state.running {
            state.running = true;
            self.timer.set_tick(Some(self.tick.clone()));
        } else {
            state.wakeup_pending = true;
        }
    }

    fn request_frame_out_of_turn(&self) {
        // The timer knows how to tick early, if it can; the tick itself goes
        // through `timer_tick` like any other.
        self.timer.request_tick_out_of_turn();
    }
}

impl Drop for DefaultRenderLoop {
    fn drop(&mut self) {
        // The tick callback only holds a weak reference to the loop, so a
        // running timer does not keep the loop alive; stop it instead of
        // leaving it ticking into nothing.
        let state = self.timer_lock.get_mut().unwrap_or_else(PoisonError::into_inner);
        if state.running {
            state.running = false;
            self.timer.set_tick(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI32;

    /// A timer that ticks when told to.
    #[derive(Default)]
    struct ManualRenderTimer {
        tick: Mutex<Option<RenderTimerTick>>,
        set_count: AtomicI32,
        clear_count: AtomicI32,
        out_of_turn_count: AtomicI32,
        runs_in_background: bool,
    }

    impl ManualRenderTimer {
        fn trigger_tick(&self) {
            let tick = lock(&self.tick).clone();
            if let Some(tick) = tick {
                tick(Duration::ZERO);
            }
        }

        fn is_running(&self) -> bool {
            lock(&self.tick).is_some()
        }
    }

    impl IRenderTimer for ManualRenderTimer {
        fn tick(&self) -> Option<RenderTimerTick> {
            lock(&self.tick).clone()
        }

        fn set_tick(&self, value: Option<RenderTimerTick>) {
            match value {
                Some(_) => self.set_count.fetch_add(1, Ordering::SeqCst),
                None => self.clear_count.fetch_add(1, Ordering::SeqCst),
            };
            *lock(&self.tick) = value;
        }

        fn runs_in_background(&self) -> bool {
            self.runs_in_background
        }

        fn request_tick_out_of_turn(&self) {
            self.out_of_turn_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    type OnRender = Box<dyn Fn() + Send + Sync>;

    #[derive(Default)]
    struct Task {
        renders: AtomicI32,
        wants_next_tick: AtomicBool,
        on_render: Mutex<Option<OnRender>>,
    }

    impl Task {
        fn new(wants_next_tick: bool) -> Arc<Task> {
            Arc::new(Task { wants_next_tick: AtomicBool::new(wants_next_tick), ..Task::default() })
        }

        fn renders(&self) -> i32 {
            self.renders.load(Ordering::SeqCst)
        }
    }

    impl IRenderLoopTask for Task {
        fn render(&self) -> bool {
            self.renders.fetch_add(1, Ordering::SeqCst);
            if let Some(on_render) = &*lock(&self.on_render) {
                on_render();
            }
            self.wants_next_tick.load(Ordering::SeqCst)
        }
    }

    fn setup() -> (crate::threading::UnitTestDispatcherScope, Arc<ManualRenderTimer>, Arc<DefaultRenderLoop>) {
        let scope = Dispatcher::unit_test_scope();
        let timer = Arc::new(ManualRenderTimer::default());
        let render_loop = DefaultRenderLoop::new(timer.clone());
        (scope, timer, render_loop)
    }

    #[test]
    fn from_timer_reports_where_the_timer_runs() {
        let background = Arc::new(ManualRenderTimer { runs_in_background: true, ..Default::default() });
        assert!(RenderLoop::from_timer(background).runs_in_background());
        assert!(!RenderLoop::from_timer(Arc::new(ManualRenderTimer::default())).runs_in_background());
    }

    #[test]
    fn a_frame_out_of_turn_is_asked_of_the_timer_and_not_rendered_by_the_caller() {
        // Not from upstream.
        let (_scope, timer, render_loop) = setup();
        let task = Task::new(true);
        render_loop.add(task.clone());

        render_loop.request_frame_out_of_turn();
        assert_eq!(timer.out_of_turn_count.load(Ordering::SeqCst), 1);
        assert_eq!(task.renders(), 0);
    }

    #[test]
    fn timer_is_not_started_without_tasks() {
        let (_scope, timer, render_loop) = setup();
        assert!(!timer.is_running());
        render_loop.wakeup();
        assert!(!timer.is_running());
        assert_eq!(timer.set_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn first_task_starts_the_timer_and_last_removal_stops_it() {
        let (_scope, timer, render_loop) = setup();
        let first = Task::new(true);
        let second = Task::new(true);
        let first_handle: Arc<dyn IRenderLoopTask> = first.clone();
        let second_handle: Arc<dyn IRenderLoopTask> = second.clone();

        render_loop.add(first_handle.clone());
        assert!(timer.is_running());
        render_loop.add(second_handle.clone());
        assert_eq!(timer.set_count.load(Ordering::SeqCst), 1);

        timer.trigger_tick();
        assert_eq!((first.renders(), second.renders()), (1, 1));

        render_loop.remove(&first_handle);
        assert!(timer.is_running());
        timer.trigger_tick();
        assert_eq!((first.renders(), second.renders()), (1, 2));

        // Removing a task that is not registered changes nothing.
        render_loop.remove(&first_handle);
        assert!(timer.is_running());

        render_loop.remove(&second_handle);
        assert!(!timer.is_running());
        assert_eq!(timer.clear_count.load(Ordering::SeqCst), 1);

        // Adding again restarts the timer.
        render_loop.add(first_handle);
        assert!(timer.is_running());
        assert_eq!(timer.set_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn timer_keeps_running_while_a_task_wants_the_next_tick() {
        let (_scope, timer, render_loop) = setup();
        let idle = Task::new(false);
        let busy = Task::new(true);
        render_loop.add(idle.clone());
        render_loop.add(busy.clone());

        timer.trigger_tick();
        timer.trigger_tick();
        assert!(timer.is_running());
        assert_eq!((idle.renders(), busy.renders()), (2, 2));

        busy.wants_next_tick.store(false, Ordering::SeqCst);
        timer.trigger_tick();
        assert!(!timer.is_running());
        assert_eq!((idle.renders(), busy.renders()), (3, 3));
    }

    #[test]
    fn timer_stops_when_no_task_wants_the_next_tick_and_wakeup_restarts_it() {
        let (_scope, timer, render_loop) = setup();
        let task = Task::new(false);
        render_loop.add(task.clone());
        assert!(timer.is_running());

        timer.trigger_tick();
        assert_eq!(task.renders(), 1);
        assert!(!timer.is_running());
        assert_eq!(timer.clear_count.load(Ordering::SeqCst), 1);

        render_loop.wakeup();
        assert!(timer.is_running());
        assert_eq!(timer.set_count.load(Ordering::SeqCst), 2);
        timer.trigger_tick();
        assert_eq!(task.renders(), 2);
        assert!(!timer.is_running());
    }

    #[test]
    fn wakeup_before_a_tick_is_consumed_by_that_tick() {
        let (_scope, timer, render_loop) = setup();
        let task = Task::new(false);
        render_loop.add(task.clone());

        // The timer is already running, so this only records a pending wakeup.
        render_loop.wakeup();
        timer.trigger_tick();
        assert!(!timer.is_running());
    }

    #[test]
    fn wakeup_during_a_tick_keeps_the_timer_running_for_one_more_tick() {
        let (_scope, timer, render_loop) = setup();
        let task = Task::new(false);
        let weak_loop = Arc::downgrade(&render_loop);
        let wake = Arc::new(AtomicBool::new(true));
        let w = wake.clone();
        *lock(&task.on_render) = Some(Box::new(move || {
            if w.load(Ordering::SeqCst) {
                weak_loop.upgrade().unwrap().wakeup();
            }
        }));
        render_loop.add(task.clone());

        timer.trigger_tick();
        assert!(timer.is_running());
        // The timer was never restarted, it just was not stopped.
        assert_eq!(timer.set_count.load(Ordering::SeqCst), 1);

        wake.store(false, Ordering::SeqCst);
        timer.trigger_tick();
        assert!(!timer.is_running());
        assert_eq!(task.renders(), 2);
    }

    #[test]
    fn late_ticks_after_the_timer_was_stopped_are_dropped() {
        let (_scope, timer, render_loop) = setup();
        let task = Task::new(true);
        let handle: Arc<dyn IRenderLoopTask> = task.clone();
        render_loop.add(handle.clone());
        let late_tick = timer.tick().unwrap();
        render_loop.remove(&handle);
        assert!(!timer.is_running());

        late_tick(Duration::ZERO);
        assert_eq!(task.renders(), 0);
    }

    #[test]
    fn tick_is_not_reentrant() {
        let (_scope, timer, render_loop) = setup();
        let task = Task::new(true);
        let nested_timer = timer.clone();
        *lock(&task.on_render) = Some(Box::new(move || nested_timer.trigger_tick()));
        render_loop.add(task.clone());

        timer.trigger_tick();
        assert_eq!(task.renders(), 1);
        timer.trigger_tick();
        assert_eq!(task.renders(), 2);
    }

    #[test]
    fn tick_is_skipped_while_another_thread_is_ticking() {
        let (_scope, timer, render_loop) = setup();
        let task = Task::new(true);
        let (entered_tx, entered_rx) = std::sync::mpsc::channel::<()>();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let entered_tx = Mutex::new(entered_tx);
        let release_rx = Mutex::new(release_rx);
        let block = Arc::new(AtomicBool::new(true));
        let b = block.clone();
        *lock(&task.on_render) = Some(Box::new(move || {
            if b.swap(false, Ordering::SeqCst) {
                lock(&entered_tx).send(()).unwrap();
                lock(&release_rx).recv().unwrap();
            }
        }));
        render_loop.add(task.clone());

        let background_timer = timer.clone();
        let background = std::thread::spawn(move || background_timer.trigger_tick());
        entered_rx.recv().unwrap();

        // The other thread is inside the tick: this one is dropped.
        timer.trigger_tick();
        assert_eq!(task.renders(), 1);

        release_tx.send(()).unwrap();
        background.join().unwrap();
        timer.trigger_tick();
        assert_eq!(task.renders(), 2);
    }

    #[test]
    fn tasks_can_be_removed_while_rendering() {
        let (_scope, timer, render_loop) = setup();
        let first = Task::new(true);
        let second = Task::new(true);
        let second_handle: Arc<dyn IRenderLoopTask> = second.clone();
        let weak_loop = Arc::downgrade(&render_loop);
        let to_remove = second_handle.clone();
        *lock(&first.on_render) = Some(Box::new(move || weak_loop.upgrade().unwrap().remove(&to_remove)));
        render_loop.add(first.clone());
        render_loop.add(second_handle);

        // The tick works on a copy of the list, so the removed task is still
        // rendered by the tick that removes it.
        timer.trigger_tick();
        assert_eq!((first.renders(), second.renders()), (1, 1));
        timer.trigger_tick();
        assert_eq!((first.renders(), second.renders()), (2, 1));
    }

    #[test]
    fn a_panicking_task_does_not_break_the_loop() {
        let (_scope, timer, render_loop) = setup();
        let task = Task::new(true);
        let fail = Arc::new(AtomicBool::new(true));
        let f = fail.clone();
        *lock(&task.on_render) = Some(Box::new(move || {
            if f.swap(false, Ordering::SeqCst) {
                panic!("render failed");
            }
        }));
        render_loop.add(task.clone());

        timer.trigger_tick();
        assert_eq!(task.renders(), 1);
        assert!(timer.is_running());

        timer.trigger_tick();
        assert_eq!(task.renders(), 2);
    }

    #[test]
    fn dropping_a_running_loop_stops_the_timer() {
        let (_scope, timer, render_loop) = setup();
        render_loop.add(Task::new(true));
        assert!(timer.is_running());
        drop(render_loop);
        assert!(!timer.is_running());
    }
}
