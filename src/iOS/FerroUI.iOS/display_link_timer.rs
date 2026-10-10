//! The render timer of the platform: a display link on a thread of its
//! own, which ticks once per frame of the display.

use block2::RcBlock;
use ferroui_base::rendering::{IRenderTimer, RenderTimerTick};
use objc2::rc::{autoreleasepool, Retained};
use objc2::{define_class, msg_send, sel, AllocAnyThread, DefinedClass};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSNotificationName, NSObject, NSObjectProtocol, NSRunLoop,
    NSRunLoopCommonModes,
};
use objc2_quartz_core::CADisplayLink;
use objc2_ui_kit::{UIApplicationDidEnterBackgroundNotification, UIApplicationWillEnterForegroundNotification};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

/// What the timer and the target of its display link share.
struct TimerState {
    tick: Mutex<Option<RenderTimerTick>>,
    /// Set while the application is in the background, where nothing may be
    /// drawn.
    paused: AtomicBool,
    /// The number of times the display link fired.
    ticks: AtomicU64,
    st: Instant,
}

impl TimerState {
    fn on_link_tick(&self) {
        self.ticks.fetch_add(1, Ordering::SeqCst);
        if self.paused.load(Ordering::SeqCst) {
            return;
        }
        let tick = self.tick.lock().unwrap_or_else(PoisonError::into_inner).clone();
        if let Some(tick) = tick {
            tick(self.st.elapsed());
        }
    }
}

define_class!(
    // SAFETY: `NSObject` has no requirements on a subclass, and the class
    // does not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "FerroDisplayLinkTarget"]
    #[ivars = Arc<TimerState>]
    struct DisplayLinkTarget;

    impl DisplayLinkTarget {
        #[unsafe(method(onLinkTick:))]
        fn on_link_tick(&self, _link: &CADisplayLink) {
            // The run loop of a thread that is not the main thread drains
            // no pool between its turns.
            autoreleasepool(|_| self.ivars().on_link_tick());
        }
    }

    unsafe impl NSObjectProtocol for DisplayLinkTarget {}
);

impl DisplayLinkTarget {
    fn new(state: Arc<TimerState>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(state);
        // SAFETY: `init` of the superclass, on the object that was just
        // allocated and whose instance variables are set.
        unsafe { msg_send![super(this), init] }
    }
}

/// The render timer of the platform.
pub struct DisplayLinkTimer {
    state: Arc<TimerState>,
}

impl DisplayLinkTimer {
    /// Creates the timer and starts its thread.
    ///
    /// # Panics
    /// Panics when the thread cannot be started.
    pub fn new() -> Arc<DisplayLinkTimer> {
        let state = Arc::new(TimerState { tick: Mutex::new(None), paused: AtomicBool::new(false), ticks: AtomicU64::new(0), st: Instant::now() });

        // The reference creates the link on the thread that registers the
        // platform and adds it to the run loop of the timer thread. Here the
        // link is an object of the timer thread from the start: it is
        // created, scheduled and released there, and no other thread
        // touches it (DEVIATIONS.md, iOS backend).
        let thread_state = state.clone();
        let spawned = std::thread::Builder::new().name("DisplayLinkTimer".to_string()).spawn(move || {
            let target = DisplayLinkTarget::new(thread_state);
            // SAFETY: the target is an object that responds to the selector
            // (`onLinkTick:` above takes the link, the one argument a
            // display link passes); the link retains its target. The mode
            // is a constant of Foundation, and the run loop is the one of
            // this thread.
            unsafe {
                let link = CADisplayLink::displayLinkWithTarget_selector(&target, sel!(onLinkTick:));
                let run_loop = NSRunLoop::currentRunLoop();
                link.addToRunLoop_forMode(&run_loop, NSRunLoopCommonModes);
                run_loop.run();
            }
        });
        if let Err(error) = spawned {
            panic!("Unable to start the thread of the render timer: {error}");
        }

        // The reference pauses the link while the application is in the
        // background. A link is paused by a property of an object that
        // belongs to the timer thread, so the ticks are dropped instead:
        // no frame is drawn in the background either way, and the system
        // stops the link of a suspended application by itself.
        // SAFETY: the two names are constants of UIKit.
        let (entered_background, will_enter_foreground) =
            unsafe { (UIApplicationDidEnterBackgroundNotification, UIApplicationWillEnterForegroundNotification) };
        Self::observe(entered_background, state.clone(), true);
        Self::observe(will_enter_foreground, state.clone(), false);

        Arc::new(DisplayLinkTimer { state })
    }

    fn observe(name: &NSNotificationName, state: Arc<TimerState>, paused: bool) {
        let block = RcBlock::new(move |_notification: NonNull<NSNotification>| {
            state.paused.store(paused, Ordering::SeqCst);
        });
        // SAFETY: the block takes the one argument of a notification block
        // and ignores it, and what it captures is shared state that any
        // thread may write (an atomic flag), so the block may be called on
        // whichever thread posts the notification.
        let observer = unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(name),
                None,
                None,
                &block,
            )
        };
        // The timer lives as long as the process, and so does the
        // subscription.
        std::mem::forget(observer);
    }
}

impl DisplayLinkTimer {
    /// The number of times the display link fired, whether or not a frame
    /// was asked for. An addition of the port, for diagnostics.
    pub fn ticks(&self) -> u64 {
        self.state.ticks.load(Ordering::SeqCst)
    }

    /// Whether a tick handler is set: whether the render loop runs.
    pub fn has_tick(&self) -> bool {
        self.state.tick.lock().unwrap_or_else(PoisonError::into_inner).is_some()
    }
}

impl IRenderTimer for DisplayLinkTimer {
    fn tick(&self) -> Option<RenderTimerTick> {
        self.state.tick.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn set_tick(&self, value: Option<RenderTimerTick>) {
        *self.state.tick.lock().unwrap_or_else(PoisonError::into_inner) = value;
    }

    fn runs_in_background(&self) -> bool {
        true
    }
}
