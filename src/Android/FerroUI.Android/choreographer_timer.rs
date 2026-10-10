//! The render timer: the frames of the choreographer of the system.
//!
//! Two threads, as in the reference: one with a looper of its own, on which
//! the choreographer calls back at every frame, and the render thread,
//! which waits for the frame and calls the tick.

use crate::interop::ndk::{looper_loop, looper_prepare, Choreographer, IFrameCallback};
use ferroui_base::rendering::{IRenderTimer, RenderTimerTick};
use std::sync::{Condvar, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

/// What the reference guards with its lock.
#[derive(Default)]
struct State {
    tick: Option<RenderTimerTick>,
    pending_callback: bool,
    last_time: i64,
}

/// An event that releases one waiter when set and resets itself.
#[derive(Default)]
struct AutoResetEvent {
    set: Mutex<bool>,
    condition: Condvar,
}

impl AutoResetEvent {
    fn set(&self) {
        *self.set.lock().unwrap_or_else(PoisonError::into_inner) = true;
        self.condition.notify_one();
    }

    fn wait_one(&self) {
        let mut set = self.set.lock().unwrap_or_else(PoisonError::into_inner);
        while !*set {
            set = self.condition.wait(set).unwrap_or_else(PoisonError::into_inner);
        }
        *set = false;
    }
}

/// The state both threads share. It lives as long as the process: the
/// choreographer holds its address between a post and a callback.
#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    /// The choreographer of the looper thread, once that thread has it.
    choreographer: Mutex<Option<Choreographer>>,
    choreographer_ready: Condvar,
    event: AutoResetEvent,
}

pub struct ChoreographerTimer {
    shared: &'static Shared,
}

impl Default for ChoreographerTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl ChoreographerTimer {
    pub fn new() -> Self {
        let shared: &'static Shared = Box::leak(Box::default());
        thread::Builder::new()
            .name("Choreographer Thread".to_string())
            .spawn(move || shared.looper_thread())
            .expect("failed to start the choreographer thread");
        thread::Builder::new()
            .name("Render Thread".to_string())
            .spawn(move || shared.render_loop())
            .expect("failed to start the render thread");
        Self { shared }
    }
}

impl Shared {
    fn looper_thread(&'static self) {
        looper_prepare();
        let choreographer = Choreographer::instance();
        *self.choreographer.lock().unwrap_or_else(PoisonError::into_inner) = choreographer;
        self.choreographer_ready.notify_all();
        if choreographer.is_none() {
            crate::log::error("The choreographer of the system is not available: nothing will be rendered.");
            return;
        }
        looper_loop();
    }

    /// The choreographer, waiting for the looper thread to have it.
    fn choreographer(&self) -> Choreographer {
        let mut choreographer = self.choreographer.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            if let Some(choreographer) = *choreographer {
                return choreographer;
            }
            choreographer = self.choreographer_ready.wait(choreographer).unwrap_or_else(PoisonError::into_inner);
        }
    }

    fn render_loop(&self) {
        loop {
            self.event.wait_one();
            let (time, tick) = {
                let state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
                (state.last_time, state.tick.clone())
            };
            if let Some(tick) = tick {
                tick(Duration::from_nanos(time.max(0) as u64));
            }
        }
    }

    /// `PostFrameCallbackIfNeeded`, with the lock held by the caller.
    fn post_frame_callback_if_needed(&'static self, state: &mut State) {
        if state.pending_callback {
            return;
        }

        if state.tick.is_none() {
            return;
        }

        state.pending_callback = true;

        self.choreographer().post_frame_callback(self);
    }
}

impl IFrameCallback for Shared {
    fn do_frame(&'static self, frame_time_nanos: i64) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.pending_callback = false;
        self.post_frame_callback_if_needed(&mut state);
        state.last_time = frame_time_nanos;
        self.event.set();
    }
}

impl IRenderTimer for ChoreographerTimer {
    fn tick(&self) -> Option<RenderTimerTick> {
        self.shared.state.lock().unwrap_or_else(PoisonError::into_inner).tick.clone()
    }

    fn set_tick(&self, value: Option<RenderTimerTick>) {
        let mut state = self.shared.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.tick = value;
        self.shared.post_frame_callback_if_needed(&mut state);
    }

    fn runs_in_background(&self) -> bool {
        true
    }
}
