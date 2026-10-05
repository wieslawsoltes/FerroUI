use crate::interop::timer_helper;
use ferroui_base::rendering::{IRenderTimer, RenderTimerTick};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::Duration;

/// A render timer that ticks with the animation frames of the page.
pub struct BrowserRenderTimer {
    this: Weak<BrowserRenderTimer>,
    tick: Mutex<Option<RenderTimerTick>>,
    started: AtomicBool,
    runs_in_background: bool,
    run_animation_frames: fn(),
}

impl BrowserRenderTimer {
    /// Creates the timer. `is_background` tells whether its ticks arrive on
    /// a thread other than the UI thread.
    pub fn new(is_background: bool) -> Arc<Self> {
        Self::with_frames(is_background, timer_helper::run_animation_frames)
    }

    fn with_frames(is_background: bool, run_animation_frames: fn()) -> Arc<Self> {
        Arc::new_cyclic(|this| Self {
            this: this.clone(),
            tick: Mutex::new(None),
            started: AtomicBool::new(false),
            runs_in_background: is_background,
            run_animation_frames,
        })
    }

    /// Starts following the animation frames on the current thread. Does
    /// nothing when the timer is started already.
    pub fn start_on_this_thread(&self) {
        if !self.started.swap(true, Ordering::SeqCst) {
            let this = self.this.clone();
            timer_helper::add_animation_frame(Rc::new(move |timestamp| {
                if let Some(this) = this.upgrade() {
                    this.render_frame_callback(timestamp);
                }
            }));
            (self.run_animation_frames)();
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
        self.start_on_this_thread();

        *self.tick.lock().unwrap_or_else(PoisonError::into_inner) = value;
    }

    fn runs_in_background(&self) -> bool {
        self.runs_in_background
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicU32;

    static FRAME_LOOPS: AtomicU32 = AtomicU32::new(0);

    fn count_frame_loop() {
        FRAME_LOOPS.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn frames_reach_the_tick_as_durations_and_the_loop_is_started_once() {
        let timer = BrowserRenderTimer::with_frames(false, count_frame_loop);
        let seen = Arc::new(Mutex::new(Vec::new()));
        assert!(!timer.runs_in_background());
        assert!(timer.tick().is_none());

        // A frame before a tick is set goes nowhere.
        timer_helper::js_export_on_animation_frame(8.0);

        let log = seen.clone();
        timer.set_tick(Some(Arc::new(move |time| log.lock().unwrap().push(time))));
        let loops = FRAME_LOOPS.load(Ordering::SeqCst);
        assert!(loops >= 1);
        timer_helper::js_export_on_animation_frame(16.0);
        timer_helper::js_export_on_animation_frame(1500.5);

        timer.set_tick(None);
        timer_helper::js_export_on_animation_frame(2000.0);

        assert_eq!(loops, FRAME_LOOPS.load(Ordering::SeqCst));
        assert_eq!(vec![Duration::from_millis(16), Duration::from_micros(1_500_500)], *seen.lock().unwrap());
    }

    #[test]
    fn a_background_timer_reports_itself() {
        assert!(BrowserRenderTimer::with_frames(true, count_frame_loop).runs_in_background());
    }
}
