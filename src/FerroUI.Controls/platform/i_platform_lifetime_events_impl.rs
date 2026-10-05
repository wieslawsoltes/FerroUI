use crate::application_lifetimes::ShutdownRequestedEventArgs;
use ferroui_base::reactive::IDisposable;
use std::rc::Rc;

/// The application lifetime events of the platform.
pub trait IPlatformLifetimeEventsImpl {
    /// Raised by the platform when a shutdown is requested.
    ///
    /// Disposing the returned value removes the handler.
    fn shutdown_requested(&self, handler: Rc<dyn Fn(&ShutdownRequestedEventArgs)>) -> Rc<dyn IDisposable>;
}
