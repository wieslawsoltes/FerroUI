use crate::interop::thread_proxy::{self, RunOnThread};
use crate::interop::timer_helper;
use ferroui_base::rendering::{IRenderTimer, RenderTimerTick};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::Duration;

/// A render timer that ticks with the animation frames of the thread it is
/// started on: the page, or the worker of a render thread.
pub struct BrowserRenderTimer {
    this: Weak<BrowserRenderTimer>,
    tick: Mutex<Option<RenderTimerTick>>,
    started: AtomicBool,
    runs_in_background: bool,
    /// The thread the frames are followed on, as
    /// [`thread_proxy::current_thread`] names it there; 0 until the timer is
    /// started, and always in a build without threads.
    thread: AtomicUsize,
    /// Whether a tick out of turn is on its way to that thread.
    out_of_turn_requested: AtomicBool,
    platform: TimerPlatform,
}

/// The calls the timer makes outside itself; replaced by recorders in the
/// tests.
#[derive(Clone, Copy)]
struct TimerPlatform {
    run_animation_frames: fn(),
    current_thread: fn() -> usize,
    run_on_thread: RunOnThread,
    now: fn() -> f64,
}

impl BrowserRenderTimer {
    /// Creates the timer. `is_background` tells whether its ticks arrive on
    /// a thread other than the UI thread: such a timer is started by the
    /// thread that renders, with
    /// [`start_on_this_thread`](Self::start_on_this_thread).
    pub fn new(is_background: bool) -> Arc<Self> {
        Self::with_platform(
            is_background,
            TimerPlatform {
                run_animation_frames: timer_helper::run_animation_frames,
                current_thread: thread_proxy::current_thread,
                run_on_thread: thread_proxy::run_on_thread,
                now: timer_helper::now,
            },
        )
    }

    fn with_platform(is_background: bool, platform: TimerPlatform) -> Arc<Self> {
        Arc::new_cyclic(|this| Self {
            this: this.clone(),
            tick: Mutex::new(None),
            started: AtomicBool::new(false),
            runs_in_background: is_background,
            thread: AtomicUsize::new(0),
            out_of_turn_requested: AtomicBool::new(false),
            platform,
        })
    }

    /// Starts following the animation frames on the current thread. Does
    /// nothing when the timer is started already.
    ///
    /// The thread has to live for as long as the timer is used: its frames
    /// arrive at the event loop of its worker, and so does a tick out of
    /// turn.
    pub fn start_on_this_thread(&self) {
        if !self.started.swap(true, Ordering::SeqCst) {
            self.thread.store((self.platform.current_thread)(), Ordering::SeqCst);
            let this = self.this.clone();
            timer_helper::add_animation_frame(Rc::new(move |timestamp| {
                if let Some(this) = this.upgrade() {
                    this.render_frame_callback(timestamp);
                }
            }));
            (self.platform.run_animation_frames)();
        }
    }

    fn render_frame_callback(&self, timestamp: f64) {
        let tick = self.tick.lock().unwrap_or_else(PoisonError::into_inner).clone();
        if let Some(tick) = tick {
            tick(Duration::from_secs_f64(timestamp.max(0.0) / 1000.0));
        }
    }
}

impl IRenderTimer for BrowserRenderTimer {
    fn tick(&self) -> Option<RenderTimerTick> {
        self.tick.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn set_tick(&self, value: Option<RenderTimerTick>) {
        // Upstream starts the loop here unless the platform runs with
        // threads, where only its render worker starts it. The condition
        // here is the timer's own: one that ticks in the background is
        // started by the thread that renders, and the thread that sets the
        // callback (the render loop does, on any thread) must not become
        // the one that ticks.
        if !self.runs_in_background {
            self.start_on_this_thread();
        }

        *self.tick.lock().unwrap_or_else(PoisonError::into_inner) = value;
    }

    fn runs_in_background(&self) -> bool {
        self.runs_in_background
    }

    /// Queues one tick for the thread the timer was started on, which runs
    /// it from its event loop without waiting for its next animation frame.
    /// Requests made before that tick has started are one request. The
    /// timestamp of the tick is read from the clock of the animation frames
    /// of that thread.
    ///
    /// Does nothing when the caller is the thread that ticks (it cannot be
    /// waiting for itself), while the timer is not started, while no
    /// callback is set, and in a build without threads.
    fn request_tick_out_of_turn(&self) {
        let thread = self.thread.load(Ordering::SeqCst);
        if thread == 0 || thread == (self.platform.current_thread)() {
            return;
        }
        if self.tick.lock().unwrap_or_else(PoisonError::into_inner).is_none() {
            return;
        }
        if self.out_of_turn_requested.swap(true, Ordering::SeqCst) {
            return;
        }
        let this = self.this.clone();
        let queued = (self.platform.run_on_thread)(
            thread,
            Box::new(move || {
                if let Some(this) = this.upgrade() {
                    // Cleared first: a request made from here on is for a
                    // frame after this one.
                    this.out_of_turn_requested.store(false, Ordering::SeqCst);
                    this.render_frame_callback((this.platform.now)());
                }
            }),
        );
        if !queued {
            self.out_of_turn_requested.store(false, Ordering::SeqCst);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interop::thread_proxy::ThreadWork;
    use std::cell::Cell;

    thread_local! {
        /// The frame loops started on the thread of a test.
        static FRAME_LOOPS: Cell<u32> = const { Cell::new(0) };
        /// The id a test gives its thread; 0 is what a build without threads
        /// reports.
        static THREAD: Cell<usize> = const { Cell::new(0) };
    }

    fn count_frame_loop() {
        FRAME_LOOPS.with(|loops| loops.set(loops.get() + 1));
    }

    fn frame_loops() -> u32 {
        FRAME_LOOPS.with(|loops| loops.get())
    }

    fn test_thread() -> usize {
        THREAD.with(|thread| thread.get())
    }

    fn refuse(_thread: usize, _work: ThreadWork) -> bool {
        false
    }

    fn quarter_of_a_second() -> f64 {
        250.0
    }

    fn platform(run_on_thread: RunOnThread) -> TimerPlatform {
        TimerPlatform {
            run_animation_frames: count_frame_loop,
            current_thread: test_thread,
            run_on_thread,
            now: quarter_of_a_second,
        }
    }

    #[test]
    fn frames_reach_the_tick_as_durations_and_the_loop_is_started_once() {
        let timer = BrowserRenderTimer::with_platform(false, platform(refuse));
        let seen = Arc::new(Mutex::new(Vec::new()));
        assert!(!timer.runs_in_background());
        assert!(timer.tick().is_none());

        // A frame before a tick is set goes nowhere.
        timer_helper::js_export_on_animation_frame(8.0);

        let log = seen.clone();
        timer.set_tick(Some(Arc::new(move |time| log.lock().unwrap().push(time))));
        assert_eq!(1, frame_loops());
        timer_helper::js_export_on_animation_frame(16.0);
        timer_helper::js_export_on_animation_frame(1500.5);

        timer.set_tick(None);
        timer_helper::js_export_on_animation_frame(2000.0);

        assert_eq!(1, frame_loops());
        assert_eq!(vec![Duration::from_millis(16), Duration::from_micros(1_500_500)], *seen.lock().unwrap());
    }

    #[test]
    fn a_background_timer_reports_itself_and_is_not_started_by_its_tick() {
        let timer = BrowserRenderTimer::with_platform(true, platform(refuse));
        let seen = Arc::new(Mutex::new(Vec::new()));
        assert!(timer.runs_in_background());

        let log = seen.clone();
        timer.set_tick(Some(Arc::new(move |time| log.lock().unwrap().push(time))));
        // The thread that set the tick is not the one that ticks.
        assert_eq!(0, frame_loops());
        timer_helper::js_export_on_animation_frame(16.0);
        assert!(seen.lock().unwrap().is_empty());

        timer.start_on_this_thread();
        timer.start_on_this_thread();
        assert_eq!(1, frame_loops());
        timer_helper::js_export_on_animation_frame(32.0);
        assert_eq!(vec![Duration::from_millis(32)], *seen.lock().unwrap());
    }

    #[test]
    fn a_tick_out_of_turn_is_queued_once_for_the_thread_that_ticks() {
        static QUEUED: Mutex<Vec<(usize, ThreadWork)>> = Mutex::new(Vec::new());
        fn queue(thread: usize, work: ThreadWork) -> bool {
            QUEUED.lock().unwrap().push((thread, work));
            true
        }
        fn take() -> Vec<(usize, ThreadWork)> {
            QUEUED.lock().unwrap().drain(..).collect()
        }
        // Two requests, made by a thread other than the one of the test.
        fn request_twice_from_another_thread(timer: &Arc<BrowserRenderTimer>) {
            let timer = timer.clone();
            std::thread::spawn(move || {
                THREAD.with(|thread| thread.set(12));
                timer.request_tick_out_of_turn();
                timer.request_tick_out_of_turn();
            })
            .join()
            .unwrap();
        }

        THREAD.with(|thread| thread.set(11));
        let timer = BrowserRenderTimer::with_platform(true, platform(queue));
        let seen = Arc::new(Mutex::new(Vec::new()));

        // Not started: there is no thread to ask.
        let log = seen.clone();
        timer.set_tick(Some(Arc::new(move |time| log.lock().unwrap().push(time))));
        request_twice_from_another_thread(&timer);
        assert!(take().is_empty());

        timer.start_on_this_thread();
        // The thread that ticks does not ask itself.
        timer.request_tick_out_of_turn();
        assert!(take().is_empty());

        // Two requests before the tick are one, for the thread that ticks;
        // the caller is not called back.
        request_twice_from_another_thread(&timer);
        let queued = take();
        assert_eq!(vec![11], queued.iter().map(|(thread, _)| *thread).collect::<Vec<usize>>());
        assert!(seen.lock().unwrap().is_empty());
        for (_, work) in queued {
            work();
        }
        assert_eq!(vec![Duration::from_millis(250)], *seen.lock().unwrap());

        // After the tick a request is queued again.
        request_twice_from_another_thread(&timer);
        assert_eq!(1, take().len());

        // Without a callback nothing is queued. The request above was
        // dropped without running, so its flag is cleared by hand.
        timer.out_of_turn_requested.store(false, Ordering::SeqCst);
        timer.set_tick(None);
        request_twice_from_another_thread(&timer);
        assert!(take().is_empty());
    }

    #[test]
    fn a_refused_tick_out_of_turn_can_be_asked_for_again() {
        THREAD.with(|thread| thread.set(21));
        let timer = BrowserRenderTimer::with_platform(true, platform(refuse));
        timer.set_tick(Some(Arc::new(|_: Duration| {})));
        timer.start_on_this_thread();

        std::thread::spawn({
            let timer = timer.clone();
            move || timer.request_tick_out_of_turn()
        })
        .join()
        .unwrap();

        assert!(!timer.out_of_turn_requested.load(Ordering::SeqCst));
    }
}
