use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::reactive::IDisposable;
use crate::threading::{DispatcherImplEvent, DispatcherPriority};

pub use crate::threading::IPlatformThreadingSignal;

/// A running platform timer; disposing it stops the timer. See
/// [`IPlatformThreadingInterface::start_timer`].
pub type PlatformTimerHandle = Rc<dyn IDisposable>;

/// Provides platform-specific services relating to threading.
///
/// This is the legacy contract; platforms are expected to implement
/// [`IDispatcherImpl`](crate::threading::IDispatcherImpl) instead. Like that
/// contract it belongs to its loop thread, except for the handle returned by
/// [`signal_handle`](Self::signal_handle).
pub trait IPlatformThreadingInterface {
    /// Starts a timer.
    ///
    /// `tick` is called on the loop thread every `interval` until the
    /// returned value is disposed.
    fn start_timer(&self, priority: DispatcherPriority, interval: Duration, tick: Rc<dyn Fn()>) -> PlatformTimerHandle;

    fn signal(&self, priority: DispatcherPriority);

    /// The thread-safe handle that does what [`signal`](Self::signal) does.
    fn signal_handle(&self) -> Arc<dyn IPlatformThreadingSignal>;

    fn current_thread_is_loop_thread(&self) -> bool;

    /// Raised on the loop thread in response to a signal.
    fn signaled(&self) -> &DispatcherImplEvent<Option<DispatcherPriority>>;
}
