use super::ActivatedEventArgs;
use ferroui_base::reactive::IDisposable;
use std::rc::Rc;

/// An interface for platforms that can activate and deactivate the
/// application, and move it into and out of the background.
pub trait IActivatableLifetime {
    /// Raised when the application is activated for various reasons, as
    /// described by the [`ActivationKind`](super::ActivationKind).
    /// Disposing the returned handle unsubscribes.
    fn activated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable>;

    /// Raised when the application is deactivated for various reasons, as
    /// described by the [`ActivationKind`](super::ActivationKind).
    /// Disposing the returned handle unsubscribes.
    fn deactivated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable>;

    /// Tells the application that it should attempt to leave its background
    /// state. Returns true if it will process the request; false otherwise.
    ///
    /// For example on macOS this would be `[NSApp unhide]`.
    fn try_leave_background(&self) -> bool;

    /// Tells the application that it should attempt to enter its background
    /// state. Returns true if it will process the request; false otherwise.
    ///
    /// For example on macOS this would be `[NSApp hide]`.
    fn try_enter_background(&self) -> bool;
}
