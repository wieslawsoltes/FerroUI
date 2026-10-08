use ferroui_base::reactive::IDisposable;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::application_lifetimes::{
    ActivatableLifetimeBase, ActivatedEventArgs, ActivationKind, IActivatableLifetime,
};
use ferroui_controls::platform::INativeApplicationCommands;
use std::rc::Rc;

/// The activatable lifetime of a macOS application: entering the background
/// hides the application, leaving it shows the application again.
pub struct MacOSActivatableLifetime {
    base: Rc<ActivatableLifetimeBase>,
}

impl Default for MacOSActivatableLifetime {
    fn default() -> Self {
        Self { base: ActivatableLifetimeBase::new() }
    }
}

impl MacOSActivatableLifetime {
    pub fn new() -> Self {
        Self::default()
    }

    /// Raises the activated event.
    pub fn on_activated(&self, kind: ActivationKind) {
        self.base.on_activated_kind(kind);
    }

    /// Raises the activated event with the given arguments.
    pub fn on_activated_with(&self, event_args: ActivatedEventArgs) {
        self.base.on_activated(event_args);
    }

    /// Raises the deactivated event.
    pub fn on_deactivated(&self, kind: ActivationKind) {
        self.base.on_deactivated_kind(kind);
    }
}

impl IActivatableLifetime for MacOSActivatableLifetime {
    fn activated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
        self.base.activated(handler)
    }

    fn deactivated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
        self.base.deactivated(handler)
    }

    fn try_leave_background(&self) -> bool {
        let native_application_commands = FerroLocator::current().get_service::<dyn INativeApplicationCommands>();
        if let Some(commands) = native_application_commands {
            commands.show_app();
        }

        true
    }

    fn try_enter_background(&self) -> bool {
        let native_application_commands = FerroLocator::current().get_service::<dyn INativeApplicationCommands>();
        if let Some(commands) = native_application_commands {
            commands.hide_app();
        }

        true
    }
}
