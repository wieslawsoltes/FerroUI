// The managed implementation needs a monotonic clock and a blocking wait,
// neither of which exists on bare WebAssembly.
#![cfg(not(target_family = "wasm"))]

use std::rc::Rc;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use crate::threading::{
    CancellationToken, DispatcherImplEvent, IControlledDispatcherImpl, IDispatcherImpl,
    IDispatcherImplWithExplicitBackgroundProcessing, IDispatcherImplWithPendingInput, IDispatcherSignal,
};

/// Supplies input events to a [`ManagedDispatcherImpl`] run loop.
pub trait IManagedDispatcherInputProvider {
    fn has_input(&self) -> bool;
    fn dispatch_next_input_event(&self);
}

#[derive(Default)]
struct State {
    signaled: bool,
    next_timer: Option<Duration>,
    background_processing_requested: bool,
    /// The auto-reset wake-up event.
    wakeup: bool,
}

struct Shared {
    state: Mutex<State>,
    wakeup: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn set_wakeup(&self, state: &mut State) {
        state.wakeup = true;
        self.wakeup.notify_all();
    }
}

impl IDispatcherSignal for Shared {
    fn signal(&self) {
        let mut state = self.lock();
        state.signaled = true;
        self.set_wakeup(&mut state);
    }
}

/// A dispatcher implementation with a run loop built on standard library
/// primitives. It is what a dispatcher uses until the platform installs its
/// own implementation, and what headless platforms can use directly.
///
/// The wake-up state is shared with other threads; everything else belongs
/// to the loop thread.
pub struct ManagedDispatcherImpl {
    input_provider: Option<Rc<dyn IManagedDispatcherInputProvider>>,
    shared: Arc<Shared>,
    clock: Instant,
    loop_thread: ThreadId,
    signaled: DispatcherImplEvent,
    timer: DispatcherImplEvent,
    ready_for_background_processing: DispatcherImplEvent,
}

impl ManagedDispatcherImpl {
    pub fn new(input_provider: Option<Rc<dyn IManagedDispatcherInputProvider>>) -> Self {
        Self {
            input_provider,
            shared: Arc::new(Shared { state: Mutex::new(State::default()), wakeup: Condvar::new() }),
            clock: Instant::now(),
            loop_thread: thread::current().id(),
            signaled: DispatcherImplEvent::new(),
            timer: DispatcherImplEvent::new(),
            ready_for_background_processing: DispatcherImplEvent::new(),
        }
    }
}

impl IDispatcherImpl for ManagedDispatcherImpl {
    fn current_thread_is_loop_thread(&self) -> bool {
        self.loop_thread == thread::current().id()
    }

    fn signal(&self) {
        IDispatcherSignal::signal(&*self.shared);
    }

    fn signal_handle(&self) -> Arc<dyn IDispatcherSignal> {
        self.shared.clone()
    }

    fn signaled(&self) -> &DispatcherImplEvent {
        &self.signaled
    }

    fn timer(&self) -> &DispatcherImplEvent {
        &self.timer
    }

    fn now(&self) -> i64 {
        self.clock.elapsed().as_millis() as i64
    }

    fn update_timer(&self, due_time_in_ms: Option<i64>) {
        let mut state = self.shared.lock();
        state.next_timer = due_time_in_ms.map(|due| Duration::from_millis(due.max(0) as u64));
        if !self.current_thread_is_loop_thread() {
            self.shared.set_wakeup(&mut state);
        }
    }

    fn as_pending_input(&self) -> Option<&dyn IDispatcherImplWithPendingInput> {
        Some(self)
    }

    fn as_explicit_background_processing(&self) -> Option<&dyn IDispatcherImplWithExplicitBackgroundProcessing> {
        Some(self)
    }

    fn as_controlled(&self) -> Option<&dyn IControlledDispatcherImpl> {
        Some(self)
    }
}

impl IDispatcherImplWithPendingInput for ManagedDispatcherImpl {
    fn can_query_pending_input(&self) -> bool {
        self.input_provider.is_some()
    }

    fn has_pending_input(&self) -> bool {
        self.input_provider.as_ref().is_some_and(|provider| provider.has_input())
    }
}

impl IDispatcherImplWithExplicitBackgroundProcessing for ManagedDispatcherImpl {
    fn ready_for_background_processing(&self) -> &DispatcherImplEvent {
        &self.ready_for_background_processing
    }

    fn request_background_processing(&self) {
        let mut state = self.shared.lock();
        state.background_processing_requested = true;
        self.shared.set_wakeup(&mut state);
    }
}

impl IControlledDispatcherImpl for ManagedDispatcherImpl {
    fn run_loop(&self, token: CancellationToken) {
        let registration = token.can_be_canceled().then(|| {
            let shared = self.shared.clone();
            token.register(move || {
                let mut state = shared.lock();
                shared.set_wakeup(&mut state);
            })
        });

        while !token.is_cancellation_requested() {
            let signaled = std::mem::take(&mut self.shared.lock().signaled);

            if signaled {
                self.signaled.raise();
                continue;
            }

            let fire_timer = {
                let mut state = self.shared.lock();
                if state.next_timer.is_some_and(|next| next < self.clock.elapsed()) {
                    state.next_timer = None;
                    true
                } else {
                    false
                }
            };

            if fire_timer {
                self.timer.raise();
                continue;
            }

            if let Some(provider) = &self.input_provider {
                if provider.has_input() {
                    provider.dispatch_next_input_event();
                    continue;
                }
            }

            let trigger_background_processing =
                std::mem::take(&mut self.shared.lock().background_processing_requested);

            if trigger_background_processing {
                self.ready_for_background_processing.raise();
                continue;
            }

            let mut state = self.shared.lock();
            match state.next_timer {
                Some(next_timer) => {
                    let wait_for = next_timer.saturating_sub(self.clock.elapsed());
                    if wait_for.as_millis() < 1 {
                        continue;
                    }
                    if !state.wakeup {
                        state = self
                            .shared
                            .wakeup
                            .wait_timeout(state, wait_for)
                            .unwrap_or_else(PoisonError::into_inner)
                            .0;
                    }
                }
                None => {
                    while !state.wakeup {
                        state = self.shared.wakeup.wait(state).unwrap_or_else(PoisonError::into_inner);
                    }
                }
            }
            state.wakeup = false;
        }

        if let Some(registration) = registration {
            registration.dispose();
        }
    }
}
