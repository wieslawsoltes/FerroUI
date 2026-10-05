use super::{IControlledApplicationLifetime, ShutdownRequestedEventArgs};
use crate::{ShutdownMode, Window};
use ferroui_base::reactive::IDisposable;
use ferroui_base::Ref;
use std::rc::Rc;

/// Controls application lifetime in classic desktop style.
///
/// This contract is not meant to be implemented by applications.
pub trait IClassicDesktopStyleApplicationLifetime: IControlledApplicationLifetime {
    /// Tries to shut down the application. The
    /// [`shutdown_requested`](Self::shutdown_requested) event can be used to
    /// cancel the shutdown.
    ///
    /// `exit_code` is an integer exit code for an application; the default
    /// exit code is 0.
    fn try_shutdown(&self, exit_code: i32) -> bool;

    /// The arguments passed to the builder's
    /// `start_with_classic_desktop_lifetime` method.
    fn args(&self) -> Option<Vec<String>>;

    /// The shutdown mode. This property indicates whether the application
    /// is shut down explicitly or implicitly. If it is set to
    /// [`ShutdownMode::OnExplicitShutdown`] the application only closes if
    /// shutdown is called. The default is
    /// [`ShutdownMode::OnLastWindowClose`].
    fn shutdown_mode(&self) -> ShutdownMode;

    /// Sets the shutdown mode.
    fn set_shutdown_mode(&self, value: ShutdownMode);

    /// The main window of the application.
    fn main_window(&self) -> Option<Ref<Window>>;

    /// Sets the main window of the application.
    fn set_main_window(&self, value: Option<Ref<Window>>);

    /// The list of all open windows in the application.
    fn windows(&self) -> Vec<Ref<Window>>;

    /// Raised by the platform when an application shutdown is requested.
    /// Disposing the returned handle unsubscribes.
    ///
    /// Application shutdown can be requested for various reasons like an
    /// operating system shutdown: the end of the session (logout or
    /// shutdown), the quit menu of the application, or the quit command of
    /// the application icon. Cancelling the event args blocks the shutdown
    /// of the operating system where the platform allows it.
    ///
    /// This event provides a first chance to cancel application shutdown;
    /// if shutdown is not canceled at this point the application will try
    /// to close each non-owned open window, invoking the
    /// [`Window::closing`] event on each and allowing each window to cancel
    /// the shutdown of the application. Windows cannot however prevent the
    /// shutdown of the operating system.
    fn shutdown_requested(&self, handler: Rc<dyn Fn(&ShutdownRequestedEventArgs)>) -> Rc<dyn IDisposable>;
}
