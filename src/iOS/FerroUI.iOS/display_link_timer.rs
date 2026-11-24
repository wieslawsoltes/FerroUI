//! The render timer of the platform: a display link on a thread of its
//! own, which ticks once per frame of the display, up to 120 Hz on
//! ProMotion displays, and is paused while the render loop is idle or the
//! application is in the background.

use crate::interop::{kCFRunLoopCommonModes, CFRunLoopGetCurrent, CFRunLoopPerformBlock, CFRunLoopWakeUp};
use crate::platform::Platform;
use block2::RcBlock;
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::rendering::{IRenderTimer, RenderTimerTick};
use objc2::rc::{autoreleasepool, Retained};
use objc2::{define_class, msg_send, sel, AllocAnyThread, ClassType, DefinedClass, MainThreadMarker};
use objc2_foundation::{
    NSBundle, NSNotification, NSNotificationCenter, NSNotificationName, NSNumber, NSObject, NSObjectProtocol,
    NSRunLoop, NSRunLoopCommonModes, NSString,
};
use objc2_quartz_core::{CADisplayLink, CAFrameRateRange};
use objc2_ui_kit::{
    UIApplicationDidEnterBackgroundNotification, UIApplicationWillEnterForegroundNotification, UIDevice, UIScreen,
    UIUserInterfaceIdiom,
};
use std::cell::RefCell;
use std::ffi::c_void;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::thread::Thread;
use std::time::Instant;

/// What the render loop, the main thread and the link thread share, under the lock of the timer.
struct LinkState {
    tick: Option<RenderTimerTick>,
    in_background: bool,
    paused: bool,
    /// The run loop of the timer thread (a `CFRunLoopRef`), once the link is on it.
    run_loop: Option<usize>,
}

/// What the timer and the target of its display link share.
struct TimerState {
    // Guards tick, in_background and paused, which the render loop, the main thread and the
    // link thread share.
    lock: Mutex<LinkState>,
    /// The number of times the display link fired.
    ticks: AtomicU64,
    st: Instant,
}

impl TimerState {
    fn lock(&self) -> MutexGuard<'_, LinkState> {
        self.lock.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn on_link_tick(&self) {
        self.ticks.fetch_add(1, Ordering::SeqCst);
        // A callback already scheduled when the link was paused can still arrive.
        let tick = {
            let state = self.lock();
            if state.paused {
                None
            } else {
                state.tick.clone()
            }
        };
        if let Some(tick) = tick {
            tick(self.st.elapsed());
        }
    }
}

ferroui_base::ferro_thread_local! {
    /// The display link, an object of the timer thread (set on that thread only).
    static LINK: RefCell<Option<Retained<CADisplayLink>>> = const { RefCell::new(None) };
}

/// Called with the lock held. The reference sets `CADisplayLink.Paused` here, from whichever
/// thread changed the state; the link of the port is an object of the timer thread, so the
/// change is applied there (`apply_link_state`), through the run loop of that thread, which may
/// be given work from any thread.
fn update_link_state(this: &Arc<TimerState>, state: &mut LinkState) {
    let paused = state.in_background || state.tick.is_none();
    if state.paused != paused {
        state.paused = paused;
        if let Some(run_loop) = state.run_loop {
            schedule_apply_link_state(this.clone(), run_loop);
        }
    }
}

fn schedule_apply_link_state(this: Arc<TimerState>, run_loop: usize) {
    let block = RcBlock::new(move || apply_link_state(&this));
    // SAFETY: the run loop is the one of the timer thread, which runs for the life of the
    // process; `CFRunLoopRef` may be given work and woken up from any thread. The block takes no
    // argument and returns nothing, as `CFRunLoopPerformBlock` calls it, and the function copies
    // it, so the reference given back here is the caller's own. The mode is a constant of Core
    // Foundation.
    unsafe {
        CFRunLoopPerformBlock(
            run_loop as *mut c_void,
            kCFRunLoopCommonModes,
            &*block as *const block2::Block<dyn Fn()> as *const c_void,
        );
        CFRunLoopWakeUp(run_loop as *mut c_void);
    }
}

/// On the timer thread: pauses or resumes the link as the state says.
fn apply_link_state(this: &TimerState) {
    let paused = this.lock().paused;
    LINK.with(|link| {
        if let Some(link) = link.borrow().as_ref() {
            link.setPaused(paused);
        }
    });
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

/// Render timer driven by a `CADisplayLink`, paused while the render loop is idle or the
/// application is in the background.
pub struct DisplayLinkTimer {
    state: Arc<TimerState>,
    timer_thread: Thread,
}

impl DisplayLinkTimer {
    /// Creates the timer and starts its thread. The options of the platform
    /// ([`Platform::options`]) are read here, so the platform has them first.
    ///
    /// # Panics
    /// Panics when the thread cannot be started.
    pub fn new() -> Arc<DisplayLinkTimer> {
        let state = Arc::new(TimerState {
            lock: Mutex::new(LinkState { tick: None, in_background: false, paused: true, run_loop: None }),
            ticks: AtomicU64::new(0),
            st: Instant::now(),
        });
        let enable_high_refresh_rate = Self::enable_high_refresh_rate();

        // The reference creates the link on the thread that registers the
        // platform and adds it to the run loop of the timer thread. Here the
        // link is an object of the timer thread from the start: it is
        // created, scheduled, paused and resumed there, and no other thread
        // touches it (DEVIATIONS.md, iOS backend).
        let thread_state = state.clone();
        let spawned = std::thread::Builder::new().name("DisplayLinkTimer".to_string()).spawn(move || {
            let target = DisplayLinkTarget::new(thread_state.clone());
            // SAFETY: the target is an object that responds to the selector
            // (`onLinkTick:` above takes the link, the one argument a
            // display link passes); the link retains its target. The mode
            // is a constant of Foundation, and the run loop is the one of
            // this thread.
            unsafe {
                let link = CADisplayLink::displayLinkWithTarget_selector(&target, sel!(onLinkTick:));
                link.setPaused(true);
                if enable_high_refresh_rate {
                    Self::request_high_refresh_rate(&link);
                }
                let run_loop = NSRunLoop::currentRunLoop();
                link.addToRunLoop_forMode(&run_loop, NSRunLoopCommonModes);
                LINK.with(|slot| *slot.borrow_mut() = Some(link));
                // From now on a change of the state is applied on this thread; the state of
                // before is applied once here.
                thread_state.lock().run_loop = Some(CFRunLoopGetCurrent() as usize);
                apply_link_state(&thread_state);
                run_loop.run();
            }
        });
        let timer_thread = match spawned {
            Ok(handle) => handle.thread().clone(),
            Err(error) => panic!("Unable to start the thread of the render timer: {error}"),
        };

        // The warning about a display kept at 60 Hz comes from the view, once it is in a window
        // and knows its screen (`warn_if_limited_to_60_hz`).
        let timer = Arc::new(DisplayLinkTimer { state, timer_thread });

        // SAFETY: the two names are constants of UIKit.
        let (entered_background, will_enter_foreground) =
            unsafe { (UIApplicationDidEnterBackgroundNotification, UIApplicationWillEnterForegroundNotification) };
        Self::observe(entered_background, timer.state.clone(), true);
        Self::observe(will_enter_foreground, timer.state.clone(), false);

        timer
    }

    /// The thread of the display link: where the timer ticks and, the
    /// timer running in the background, where the compositor renders.
    pub fn timer_thread(&self) -> &Thread {
        &self.timer_thread
    }

    /// `iOSPlatformOptions.EnableHighRefreshRate` of the options of the platform; false when the
    /// platform has no options yet (`Platform.Options is { EnableHighRefreshRate: true }`).
    /// Only on the main thread, which holds the options.
    fn enable_high_refresh_rate() -> bool {
        Platform::options().is_some_and(|options| options.enable_high_refresh_rate)
    }

    /// Whether display links ask for more than the default 60 Hz: the platform options enable it
    /// and display links have `preferredFrameRateRange` (iOS, tvOS and Mac Catalyst 15, which the
    /// reference asks for by version). Only on the main thread.
    pub(crate) fn uses_high_refresh_rate() -> bool {
        Self::enable_high_refresh_rate() && CADisplayLink::class().responds_to(sel!(setPreferredFrameRateRange:))
    }

    /// Lets `link` follow the display refresh rate instead of the 60 Hz default when
    /// [`uses_high_refresh_rate`](Self::uses_high_refresh_rate). On iPhone, this also needs
    /// CADisableMinimumFrameDurationOnPhone in the app's Info.plist. Only on the main thread.
    pub(crate) fn set_preferred_frame_rate_range(link: &CADisplayLink) {
        if Self::enable_high_refresh_rate() {
            Self::request_high_refresh_rate(link);
        }
    }

    /// Asks `link` for up to 120 Hz, where links have `preferredFrameRateRange`; the caller asks
    /// [`IosPlatformOptions::enable_high_refresh_rate`](crate::IosPlatformOptions) first.
    fn request_high_refresh_rate(link: &CADisplayLink) {
        if link.respondsToSelector(sel!(setPreferredFrameRateRange:)) {
            link.setPreferredFrameRateRange(CAFrameRateRange { minimum: 60.0, maximum: 120.0, preferred: 120.0 });
        }
    }

    /// Whether display links can actually run faster than 60 Hz on `screen`:
    /// [`uses_high_refresh_rate`](Self::uses_high_refresh_rate), a screen faster than 60 Hz and,
    /// on iPhone, CADisableMinimumFrameDurationOnPhone in Info.plist.
    pub(crate) fn can_exceed_60_hz(mtm: MainThreadMarker, screen: Option<&UIScreen>) -> bool {
        Self::uses_high_refresh_rate()
            && !Self::is_phone_without_high_refresh_rate_key(mtm)
            && screen.is_some_and(|screen| screen.maximumFramesPerSecond() > 60)
    }

    /// An iPhone whose application's Info.plist does not allow more than 60 Hz; read once.
    fn is_phone_without_high_refresh_rate_key(mtm: MainThreadMarker) -> bool {
        static PHONE_WITHOUT_KEY: OnceLock<bool> = OnceLock::new();
        *PHONE_WITHOUT_KEY.get_or_init(|| {
            if UIDevice::currentDevice(mtm).userInterfaceIdiom() != UIUserInterfaceIdiom::Phone {
                return false;
            }
            let key = NSString::from_str("CADisableMinimumFrameDurationOnPhone");
            let allowed = NSBundle::mainBundle()
                .objectForInfoDictionaryKey(&key)
                .and_then(|value| value.downcast::<NSNumber>().ok())
                .is_some_and(|value| value.boolValue());
            !allowed
        })
    }

    /// Warns once when rendering on `screen` could go above 60 Hz but iOS keeps it at 60 Hz.
    pub(crate) fn warn_if_limited_to_60_hz(mtm: MainThreadMarker, screen: Option<&UIScreen>) {
        static WARNED: AtomicBool = AtomicBool::new(false);
        if WARNED.load(Ordering::Relaxed)
            || !Self::enable_high_refresh_rate()
            || !screen.is_some_and(|screen| screen.maximumFramesPerSecond() > 60)
            || !Self::is_phone_without_high_refresh_rate_key(mtm)
        {
            return;
        }
        WARNED.store(true, Ordering::Relaxed);
        if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::IOS_PLATFORM) {
            logger.log(
                None,
                "Rendering is limited to 60 Hz on this ProMotion display. Set CADisableMinimumFrameDurationOnPhone \
                 to true in Info.plist, or set IosPlatformOptions::enable_high_refresh_rate to false.",
            );
        }
    }

    fn set_in_background(state: &Arc<TimerState>, in_background: bool) {
        let mut link_state = state.lock();
        link_state.in_background = in_background;
        update_link_state(state, &mut link_state);
    }

    fn observe(name: &NSNotificationName, state: Arc<TimerState>, in_background: bool) {
        let block = RcBlock::new(move |_notification: NonNull<NSNotification>| {
            Self::set_in_background(&state, in_background);
        });
        // SAFETY: the block takes the one argument of a notification block
        // and ignores it, and what it captures is shared state that any
        // thread may change (under the lock of the timer), so the block may
        // be called on whichever thread posts the notification.
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
        self.state.lock().tick.is_some()
    }
}

impl IRenderTimer for DisplayLinkTimer {
    fn tick(&self) -> Option<RenderTimerTick> {
        self.state.lock().tick.clone()
    }

    fn set_tick(&self, value: Option<RenderTimerTick>) {
        let mut link_state = self.state.lock();
        link_state.tick = value;
        update_link_state(&self.state, &mut link_state);
    }

    fn runs_in_background(&self) -> bool {
        true
    }
}
