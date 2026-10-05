use super::{
    ControlledApplicationLifetimeExitEventArgs, ControlledApplicationLifetimeStartupEventArgs, IApplicationLifetime,
};
use ferroui_base::reactive::IDisposable;
use std::rc::Rc;

/// A lifetime whose end is controlled by the application.
pub trait IControlledApplicationLifetime: IApplicationLifetime {
    /// Raised when the application is starting. Disposing the returned
    /// handle unsubscribes.
    fn startup(&self, handler: Rc<dyn Fn(&ControlledApplicationLifetimeStartupEventArgs)>) -> Rc<dyn IDisposable>;

    /// Raised when the application is exiting. Disposing the returned handle
    /// unsubscribes.
    fn exit(&self, handler: Rc<dyn Fn(&ControlledApplicationLifetimeExitEventArgs)>) -> Rc<dyn IDisposable>;

    /// Shuts down the application and sets the exit code that is returned to
    /// the operating system.
    ///
    /// `exit_code` is an integer exit code for an application; the default
    /// exit code is 0.
    fn shutdown(&self, exit_code: i32);
}
