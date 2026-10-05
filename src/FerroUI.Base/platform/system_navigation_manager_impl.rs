use crate::interactivity::RoutedEventArgs;
use crate::reactive::IDisposable;
use std::rc::Rc;

/// The system navigation of a platform: tells when the user asks to go back
/// (a hardware or system back button, a back gesture).
pub trait ISystemNavigationManagerImpl {
    /// Occurs when a back navigation has been requested. The handler
    /// receives the event arguments of the request; marking them as handled
    /// tells the platform that the request has been dealt with.
    ///
    /// The arguments are shared so that a handler can pass them on to a
    /// routed event raised through the dispatcher.
    ///
    /// Disposing the returned value removes the handler.
    fn back_requested(&self, handler: Rc<dyn Fn(&Rc<RoutedEventArgs>)>) -> Rc<dyn IDisposable>;
}
