// The timer owns a thread and sleeps on it; neither exists on bare
// WebAssembly.
#![cfg(not(target_family = "wasm"))]

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use super::auto_reset_event::AutoResetEvent;
use super::{IRenderTimer, RenderTimerTick};

/// The state shared with the timer thread.
struct Shared {
    tick: Mutex<Option<RenderTimerTick>>,
    stopped: AtomicBool,
    wake_event: AutoResetEvent,
    st: Instant,
    desired_fps: AtomicI32,
    /// Set when the timer is dropped, to let the thread exit.
    released: AtomicBool,
}

/// A render timer that ticks on a thread of its own, sleeping between the
/// ticks.
pub struct SleepLoopRenderTimer {
    shared: Arc<Shared>,
    thread_started: Mutex<bool>,
}

impl SleepLoopRenderTimer {
    /// # Panics
    /// Panics when `fps` is less than 1.
    pub fn new(fps: i32) -> Self {
        let timer = Self {
            shared: Arc::new(Shared {
                tick: Mutex::new(None),
                stopped: AtomicBool::new(true),
                wake_event: AutoResetEvent::new(false),
                st: Instant::now(),
                desired_fps: AtomicI32::new(0),
                released: AtomicBool::new(false),
            }),
            thread_started: Mutex::new(false),
        };
        timer.set_desired_fps(fps);
        timer
    }

    pub fn desired_fps(&self) -> i32 {
        self.shared.desired_fps.load(Ordering::SeqCst)
    }

    /// # Panics
    /// Panics when `value` is less than 1.
    pub fn set_desired_fps(&self, value: i32) {
        if value < 1 {
            panic!("value ('{value}') must be greater than or equal to '1'. (Parameter 'value')");
        }
        self.shared.desired_fps.store(value, Ordering::SeqCst);
    }
}

impl Shared {
    fn loop_proc(&self) {
        let mut last_tick = self.st.elapsed();
        loop {
            if self.stopped.load(Ordering::SeqCst) {
                self.wake_event.wait_one();
            }
            if self.released.load(Ordering::SeqCst) {
                return;
            }

            let now = self.st.elapsed();
            let tick_interval = Duration::from_secs_f64(1.0 / f64::from(self.desired_fps.load(Ordering::SeqCst)));
            // Negative when the next tick is overdue.
            if let Some(time_till_next_tick) = (last_tick + tick_interval).checked_sub(now) {
                if time_till_next_tick.as_secs_f64() * 1000.0 > 1.0 {
                    self.wake_event.wait_one_timeout(time_till_next_tick);
                }
            }
            if self.released.load(Ordering::SeqCst) {
                return;
            }
            let now = self.st.elapsed();
            last_tick = now;

            let tick = self.tick.lock().unwrap_or_else(PoisonError::into_inner).clone();
            if let Some(tick) = tick {
                tick(now);
            }
        }
    }
}

impl IRenderTimer for SleepLoopRenderTimer {
    fn tick(&self) -> Option<RenderTimerTick> {
        self.shared.tick.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn set_tick(&self, value: Option<RenderTimerTick>) {
        match value {
            Some(value) => {
                *self.shared.tick.lock().unwrap_or_else(PoisonError::into_inner) = Some(value);
                self.shared.stopped.store(false, Ordering::SeqCst);
                let mut thread_started = self.thread_started.lock().unwrap_or_else(PoisonError::into_inner);
                if !*thread_started {
                    *thread_started = true;
                    let shared = self.shared.clone();
                    thread::Builder::new()
                        .name("SleepLoopRenderTimer".to_string())
                        .spawn(move || shared.loop_proc())
                        .expect("failed to start the render timer thread");
                } else {
                    self.shared.wake_event.set();
                }
            }
            None => {
                self.shared.stopped.store(true, Ordering::SeqCst);
                *self.shared.tick.lock().unwrap_or_else(PoisonError::into_inner) = None;
            }
        }
    }

    fn runs_in_background(&self) -> bool {
        true
    }
}

impl Drop for SleepLoopRenderTimer {
    fn drop(&mut self) {
        self.shared.released.store(true, Ordering::SeqCst);
        self.shared.stopped.store(true, Ordering::SeqCst);
        self.shared.wake_event.set();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    #[should_panic(expected = "must be greater than or equal to '1'")]
    fn fps_must_be_positive() {
        SleepLoopRenderTimer::new(0);
    }

    #[test]
    fn ticks_with_increasing_timestamps_until_stopped() {
        let timer = SleepLoopRenderTimer::new(500);
        assert!(timer.runs_in_background());
        assert_eq!(timer.desired_fps(), 500);
        timer.set_desired_fps(1000);
        assert_eq!(timer.desired_fps(), 1000);
        assert!(timer.tick().is_none());

        let (sender, receiver) = mpsc::channel();
        let sender = Mutex::new(sender);
        timer.set_tick(Some(Arc::new(move |time| {
            let _ = sender.lock().unwrap().send(time);
        })));
        assert!(timer.tick().is_some());

        let first = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        let second = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        assert!(second > first);

        timer.set_tick(None);
        assert!(timer.tick().is_none());
        // The callback (and with it the sender) is released by the timer and,
        // at the latest after the tick in flight, by the thread.
        loop {
            match receiver.recv_timeout(Duration::from_secs(10)) {
                Ok(_) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => panic!("the timer did not stop"),
            }
        }

        // Restarting wakes the sleeping thread up.
        let (sender, receiver) = mpsc::channel();
        let sender = Mutex::new(sender);
        timer.set_tick(Some(Arc::new(move |time| {
            let _ = sender.lock().unwrap().send(time);
        })));
        assert!(receiver.recv_timeout(Duration::from_secs(10)).unwrap() > second);
    }
}
