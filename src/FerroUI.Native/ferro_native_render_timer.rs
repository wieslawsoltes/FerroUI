//! The render timer over the native display link.

use crate::interop::*;
use ferroui_base::rendering::{IRenderTimer, RenderTimerTick};
use ferroui_microcom::ComPtr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

/// What the timer shares with the callback the display link invokes.
struct Shared {
    stopwatch: Instant,
    tick: Mutex<Option<RenderTimerTick>>,
}

/// The callback registered with the native timer; runs on the display link
/// thread.
struct TickCallback(Arc<Shared>);

impl IFrnActionCallbackImpl for TickCallback {
    fn run(&self) {
        crate::callback_base::guard((), || {
            let tick = self.0.tick.lock().unwrap_or_else(PoisonError::into_inner).clone();
            if let Some(tick) = tick {
                tick(self.0.stopwatch.elapsed());
            }
        })
    }
}

/// A render timer that ticks with the display refresh.
pub struct FerroNativeRenderTimer {
    platform_render_timer: ComPtr<IFrnPlatformRenderTimer>,
    shared: Arc<Shared>,
    registered: AtomicBool,
}

// SAFETY: the render loop changes the tick from whichever thread adds or
// removes its last task, never concurrently. The native timer only wraps a
// display link: starting, stopping and querying it are thread-safe Core Video
// calls, and reference counting of native objects is atomic.
unsafe impl Send for FerroNativeRenderTimer {}
// SAFETY: see `Send`.
unsafe impl Sync for FerroNativeRenderTimer {}

impl FerroNativeRenderTimer {
    pub fn new(platform_render_timer: ComPtr<IFrnPlatformRenderTimer>) -> Self {
        Self {
            platform_render_timer,
            shared: Arc::new(Shared { stopwatch: Instant::now(), tick: Mutex::new(None) }),
            registered: AtomicBool::new(false),
        }
    }

    fn ensure_registered(&self) {
        if !self.registered.swap(true, Ordering::SeqCst) {
            let callback = IFrnActionCallback::from_impl(TickCallback(self.shared.clone()));
            let registration_result = self.platform_render_timer.register_tick(Some(&callback));
            if registration_result != 0 {
                panic!(
                    "The native backend was not able to start the render timer. Native error code is: {registration_result}"
                );
            }
        }
    }
}

impl IRenderTimer for FerroNativeRenderTimer {
    fn tick(&self) -> Option<RenderTimerTick> {
        self.shared.tick.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn set_tick(&self, value: Option<RenderTimerTick>) {
        match value {
            Some(value) => {
                *self.shared.tick.lock().unwrap_or_else(PoisonError::into_inner) = Some(value);
                self.ensure_registered();
                self.platform_render_timer.start();
            }
            None => {
                self.platform_render_timer.stop();
                *self.shared.tick.lock().unwrap_or_else(PoisonError::into_inner) = None;
            }
        }
    }

    fn runs_in_background(&self) -> bool {
        self.platform_render_timer.runs_in_background()
    }
}
