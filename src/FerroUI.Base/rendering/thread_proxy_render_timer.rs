// The timer owns a thread; threads do not exist on bare WebAssembly.
#![cfg(not(all(target_family = "wasm", target_os = "unknown")))]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};
use std::thread;
use std::time::Instant;

use super::auto_reset_event::AutoResetEvent;
use super::{IRenderTimer, RenderTimerTick};

/// The state shared with the timer thread.
struct Shared {
    tick: Mutex<Option<RenderTimerTick>>,
    /// Started together with the thread.
    stopwatch: OnceLock<Instant>,
    auto_reset_event: AutoResetEvent,
    /// Set when the timer is dropped, to let the thread exit.
    released: AtomicBool,
}

impl Shared {
    fn render_timer_thread_func(&self) {
        loop {
            self.auto_reset_event.wait_one();
            if self.released.load(Ordering::SeqCst) {
                return;
            }

            let tick = self.tick.lock().unwrap_or_else(PoisonError::into_inner).clone();
            if let Some(tick) = tick {
                tick(self.stopwatch.get().map(Instant::elapsed).unwrap_or_default());
            }
        }
    }
}

/// A render timer that moves the ticks of another timer to a dedicated
/// thread.
pub struct ThreadProxyRenderTimer {
    inner: Arc<dyn IRenderTimer>,
    shared: Arc<Shared>,
    max_stack_size: usize,
    /// Guards `registered` and every change of the tick and of `active`.
    lock: Mutex<bool>,
    active: AtomicBool,
    inner_tick: RenderTimerTick,
}

impl ThreadProxyRenderTimer {
    /// The stack size of the timer thread used by [`new`](Self::new).
    pub const DEFAULT_MAX_STACK_SIZE: usize = 1024 * 1024;

    pub fn new(inner: Arc<dyn IRenderTimer>) -> Arc<ThreadProxyRenderTimer> {
        Self::with_max_stack_size(inner, Self::DEFAULT_MAX_STACK_SIZE)
    }

    pub fn with_max_stack_size(inner: Arc<dyn IRenderTimer>, max_stack_size: usize) -> Arc<ThreadProxyRenderTimer> {
        Arc::new_cyclic(|weak_self: &Weak<ThreadProxyRenderTimer>| {
            let this = weak_self.clone();
            ThreadProxyRenderTimer {
                inner,
                shared: Arc::new(Shared {
                    tick: Mutex::new(None),
                    stopwatch: OnceLock::new(),
                    auto_reset_event: AutoResetEvent::new(false),
                    released: AtomicBool::new(false),
                }),
                max_stack_size,
                lock: Mutex::new(false),
                active: AtomicBool::new(false),
                inner_tick: Arc::new(move |_| {
                    if let Some(this) = this.upgrade() {
                        this.inner_tick();
                    }
                }),
            }
        })
    }

    fn ensure_started(&self, registered: &mut bool) {
        if !*registered {
            *registered = true;
            let _ = self.shared.stopwatch.set(Instant::now());
            let shared = self.shared.clone();
            thread::Builder::new()
                .name("RenderTimerLoop".to_string())
                .stack_size(self.max_stack_size)
                .spawn(move || shared.render_timer_thread_func())
                .expect("failed to start the render timer thread");
        }
    }

    fn inner_tick(&self) {
        {
            let _guard = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
            if !self.active.load(Ordering::SeqCst) {
                self.inner.set_tick(None);
                return;
            }
        }
        self.shared.auto_reset_event.set();
    }
}

impl IRenderTimer for ThreadProxyRenderTimer {
    fn tick(&self) -> Option<RenderTimerTick> {
        self.shared.tick.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn set_tick(&self, value: Option<RenderTimerTick>) {
        let mut registered = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        match value {
            Some(value) => {
                *self.shared.tick.lock().unwrap_or_else(PoisonError::into_inner) = Some(value);
                self.active.store(true, Ordering::SeqCst);
                self.ensure_started(&mut registered);
                self.inner.set_tick(Some(self.inner_tick.clone()));
            }
            None => {
                // Don't clear the inner timer's tick here — may be on the wrong thread.
                // inner_tick will detect active=false and clear it on the correct thread.
                self.active.store(false, Ordering::SeqCst);
                *self.shared.tick.lock().unwrap_or_else(PoisonError::into_inner) = None;
            }
        }
    }

    fn runs_in_background(&self) -> bool {
        true
    }
}

impl Drop for ThreadProxyRenderTimer {
    fn drop(&mut self) {
        self.shared.released.store(true, Ordering::SeqCst);
        self.shared.auto_reset_event.set();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI32;
    use std::sync::mpsc;
    use std::thread::ThreadId;
    use std::time::Duration;

    #[derive(Default)]
    struct ManualRenderTimer {
        tick: Mutex<Option<RenderTimerTick>>,
        clear_count: AtomicI32,
    }

    impl ManualRenderTimer {
        fn trigger_tick(&self) {
            let tick = self.tick.lock().unwrap().clone();
            if let Some(tick) = tick {
                tick(Duration::ZERO);
            }
        }
    }

    impl IRenderTimer for ManualRenderTimer {
        fn tick(&self) -> Option<RenderTimerTick> {
            self.tick.lock().unwrap().clone()
        }

        fn set_tick(&self, value: Option<RenderTimerTick>) {
            if value.is_none() {
                self.clear_count.fetch_add(1, Ordering::SeqCst);
            }
            *self.tick.lock().unwrap() = value;
        }

        fn runs_in_background(&self) -> bool {
            false
        }
    }

    #[test]
    fn forwards_inner_ticks_to_its_own_thread() {
        let inner = Arc::new(ManualRenderTimer::default());
        let proxy = ThreadProxyRenderTimer::new(inner.clone());
        assert!(proxy.runs_in_background());
        assert!(proxy.tick().is_none());
        assert!(inner.tick().is_none());

        let (sender, receiver) = mpsc::channel::<(ThreadId, Duration)>();
        let sender = Mutex::new(sender);
        proxy.set_tick(Some(Arc::new(move |time| {
            let _ = sender.lock().unwrap().send((thread::current().id(), time));
        })));
        assert!(proxy.tick().is_some());
        assert!(inner.tick().is_some());

        inner.trigger_tick();
        let (first_thread, first_time) = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_ne!(first_thread, thread::current().id());

        inner.trigger_tick();
        let (second_thread, second_time) = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
        assert_eq!(first_thread, second_thread);
        assert!(second_time >= first_time);

        // Stopping does not touch the inner timer...
        proxy.set_tick(None);
        assert!(proxy.tick().is_none());
        assert!(inner.tick().is_some());
        assert_eq!(inner.clear_count.load(Ordering::SeqCst), 0);

        // ...its next tick does, on the inner timer's own thread.
        inner.trigger_tick();
        assert!(inner.tick().is_none());
        assert_eq!(inner.clear_count.load(Ordering::SeqCst), 1);
        assert!(matches!(
            receiver.recv_timeout(Duration::from_millis(50)),
            Err(mpsc::RecvTimeoutError::Disconnected | mpsc::RecvTimeoutError::Timeout)
        ));

        // Restarting subscribes to the inner timer again.
        let (sender, receiver) = mpsc::channel::<ThreadId>();
        let sender = Mutex::new(sender);
        proxy.set_tick(Some(Arc::new(move |_| {
            let _ = sender.lock().unwrap().send(thread::current().id());
        })));
        inner.trigger_tick();
        assert_eq!(receiver.recv_timeout(Duration::from_secs(10)).unwrap(), first_thread);
    }
}
