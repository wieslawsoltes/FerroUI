use std::rc::Rc;
use std::sync::Arc;

use super::CancellationToken;
use crate::utilities::HandlerList;

/// A multicast callback list used for the events exposed by dispatcher
/// platform implementations. Like the implementations themselves it belongs
/// to the loop thread.
///
/// Raising the event does not allocate.
pub struct DispatcherImplEvent<A = ()> {
    handlers: HandlerList<dyn Fn(A)>,
}

impl<A> Default for DispatcherImplEvent<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A> DispatcherImplEvent<A> {
    pub fn new() -> Self {
        Self { handlers: HandlerList::new() }
    }

    /// Subscribes `handler` and returns the token that removes it.
    pub fn add(&self, handler: Rc<dyn Fn(A)>) -> u64 {
        self.handlers.add(handler)
    }

    /// Removes the handler registered under `token`.
    pub fn remove(&self, token: u64) -> bool {
        self.handlers.remove(token)
    }

    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

impl<A: Clone> DispatcherImplEvent<A> {
    /// Invokes every subscribed handler with `arg`.
    pub fn invoke(&self, arg: A) {
        let snapshot = self.handlers.snapshot();
        for (_, handler) in snapshot.iter() {
            handler(arg.clone());
        }
    }
}

impl DispatcherImplEvent<()> {
    /// Invokes every subscribed handler.
    pub fn raise(&self) {
        self.invoke(());
    }
}

/// The one part of a dispatcher platform implementation that is used from
/// other threads: waking the loop thread up.
pub trait IDispatcherSignal: Send + Sync {
    /// Asynchronously triggers the [`IDispatcherImpl::signaled`] callback on
    /// the loop thread. Must not call back into the dispatcher.
    fn signal(&self);
}

/// The one part of an
/// [`IPlatformThreadingInterface`](crate::platform::IPlatformThreadingInterface)
/// that is used from other threads: waking the loop thread up.
pub trait IPlatformThreadingSignal: Send + Sync {
    /// Does what `IPlatformThreadingInterface::signal` does, from any
    /// thread.
    fn signal(&self, priority: super::DispatcherPriority);
}

/// The platform side of a [`Dispatcher`](super::Dispatcher).
///
/// An implementation belongs to its loop thread, like the rest of the object
/// model: it is held as `Rc<dyn IDispatcherImpl>`, every member is called on
/// the loop thread only, and it is free to hold handles that cannot leave the
/// thread. The single cross-thread entry point is the wake-up, which is
/// factored out into [`IDispatcherSignal`]: the dispatcher asks for it once
/// with [`signal_handle`](Self::signal_handle) and uses that handle when work
/// is queued from another thread.
///
/// Implementations must not call back into the dispatcher synchronously from
/// `signal`, `update_timer`, `now` or
/// [`request_background_processing`](IDispatcherImplWithExplicitBackgroundProcessing::request_background_processing):
/// the dispatcher calls them while holding its instance lock.
pub trait IDispatcherImpl {
    fn current_thread_is_loop_thread(&self) -> bool;

    /// Asynchronously triggers the [`signaled`](Self::signaled) callback.
    /// Called on the loop thread; other threads go through
    /// [`signal_handle`](Self::signal_handle).
    fn signal(&self);

    /// The thread-safe handle that does what [`signal`](Self::signal) does.
    fn signal_handle(&self) -> Arc<dyn IDispatcherSignal>;

    /// Raised on the loop thread in response to a signal.
    fn signaled(&self) -> &DispatcherImplEvent;

    /// Raised on the loop thread when the timer set by
    /// [`update_timer`](Self::update_timer) is due.
    fn timer(&self) -> &DispatcherImplEvent;

    /// Monotonic time in milliseconds.
    fn now(&self) -> i64;

    /// Sets (or clears) the time at which [`timer`](Self::timer) is raised.
    fn update_timer(&self, due_time_in_ms: Option<i64>);

    /// Returns `self` when the implementation can report pending input.
    fn as_pending_input(&self) -> Option<&dyn IDispatcherImplWithPendingInput> {
        None
    }

    /// Returns `self` when the implementation supports explicit background
    /// processing requests.
    fn as_explicit_background_processing(&self) -> Option<&dyn IDispatcherImplWithExplicitBackgroundProcessing> {
        None
    }

    /// Returns `self` when the implementation can run the event loop.
    fn as_controlled(&self) -> Option<&dyn IControlledDispatcherImpl> {
        None
    }
}

pub trait IDispatcherImplWithPendingInput: IDispatcherImpl {
    /// Checks if the dispatcher implementation can query pending input.
    fn can_query_pending_input(&self) -> bool;

    /// Checks if there is pending user input.
    fn has_pending_input(&self) -> bool;
}

pub trait IDispatcherImplWithExplicitBackgroundProcessing: IDispatcherImpl {
    /// Raised on the loop thread once input has been handled after a call to
    /// [`request_background_processing`](Self::request_background_processing).
    fn ready_for_background_processing(&self) -> &DispatcherImplEvent;

    fn request_background_processing(&self);
}

pub trait IControlledDispatcherImpl: IDispatcherImplWithPendingInput {
    /// Runs the event loop until `token` is canceled.
    fn run_loop(&self, token: CancellationToken);
}

/// A signal that wakes nobody up.
pub(crate) struct DetachedDispatcherSignal;

impl IDispatcherSignal for DetachedDispatcherSignal {
    fn signal(&self) {}
}

/// Stand-in used while a dispatcher has no usable platform implementation:
/// after shutdown in unit tests, and before the platform is initialized on
/// targets where the managed implementation is not available.
pub(crate) struct DetachedDispatcherImpl {
    signaled: DispatcherImplEvent,
    timer: DispatcherImplEvent,
}

impl DetachedDispatcherImpl {
    pub(crate) fn new() -> Self {
        Self { signaled: DispatcherImplEvent::new(), timer: DispatcherImplEvent::new() }
    }
}

impl IDispatcherImpl for DetachedDispatcherImpl {
    fn current_thread_is_loop_thread(&self) -> bool {
        true
    }

    fn signal(&self) {}

    fn signal_handle(&self) -> Arc<dyn IDispatcherSignal> {
        Arc::new(DetachedDispatcherSignal)
    }

    fn signaled(&self) -> &DispatcherImplEvent {
        &self.signaled
    }

    fn timer(&self) -> &DispatcherImplEvent {
        &self.timer
    }

    fn now(&self) -> i64 {
        0
    }

    fn update_timer(&self, _due_time_in_ms: Option<i64>) {}
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub(crate) use legacy::LegacyDispatcherImpl;

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
mod legacy {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::{DispatcherImplEvent, IDispatcherImpl, IDispatcherSignal};
    use super::IPlatformThreadingSignal;
    use crate::platform::IPlatformThreadingInterface;
    use crate::reactive::IDisposable;
    use crate::threading::DispatcherPriority;

    struct LegacySignal(Arc<dyn IPlatformThreadingSignal>);

    impl IDispatcherSignal for LegacySignal {
        fn signal(&self) {
            self.0.signal(DispatcherPriority::SEND);
        }
    }

    /// Adapts an [`IPlatformThreadingInterface`] to [`IDispatcherImpl`].
    pub(crate) struct LegacyDispatcherImpl {
        platform_threading: Rc<dyn IPlatformThreadingInterface>,
        timer_handle: Rc<RefCell<Option<Rc<dyn IDisposable>>>>,
        clock: Instant,
        signaled: Rc<DispatcherImplEvent>,
        timer: Rc<DispatcherImplEvent>,
    }

    impl LegacyDispatcherImpl {
        pub(crate) fn new(platform_threading: Rc<dyn IPlatformThreadingInterface>) -> Self {
            let signaled = Rc::new(DispatcherImplEvent::new());
            let forward = signaled.clone();
            platform_threading.signaled().add(Rc::new(move |_| forward.raise()));
            Self {
                platform_threading,
                timer_handle: Rc::new(RefCell::new(None)),
                clock: Instant::now(),
                signaled,
                timer: Rc::new(DispatcherImplEvent::new()),
            }
        }

        fn elapsed_milliseconds(&self) -> i64 {
            self.clock.elapsed().as_millis() as i64
        }
    }

    fn dispose_timer(slot: &RefCell<Option<Rc<dyn IDisposable>>>) {
        let handle = slot.borrow_mut().take();
        if let Some(handle) = handle {
            handle.dispose();
        }
    }

    impl IDispatcherImpl for LegacyDispatcherImpl {
        fn current_thread_is_loop_thread(&self) -> bool {
            self.platform_threading.current_thread_is_loop_thread()
        }

        fn signal(&self) {
            self.platform_threading.signal(DispatcherPriority::SEND);
        }

        fn signal_handle(&self) -> Arc<dyn IDispatcherSignal> {
            Arc::new(LegacySignal(self.platform_threading.signal_handle()))
        }

        fn signaled(&self) -> &DispatcherImplEvent {
            &self.signaled
        }

        fn timer(&self) -> &DispatcherImplEvent {
            &self.timer
        }

        fn now(&self) -> i64 {
            self.elapsed_milliseconds()
        }

        fn update_timer(&self, due_time_in_ms: Option<i64>) {
            dispose_timer(&self.timer_handle);

            if let Some(due_time) = due_time_in_ms {
                let interval = (due_time - self.elapsed_milliseconds()).max(1);
                let slot = self.timer_handle.clone();
                let timer = self.timer.clone();
                let handle = self.platform_threading.start_timer(
                    DispatcherPriority::SEND,
                    Duration::from_millis(interval as u64),
                    Rc::new(move || {
                        // OnTick
                        dispose_timer(&slot);
                        timer.raise();
                    }),
                );
                *self.timer_handle.borrow_mut() = Some(handle);
            }
        }
    }
}
