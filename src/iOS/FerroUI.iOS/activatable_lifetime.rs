//! The activatable lifetime of the platform: the activations the
//! application delegate reports.

use crate::ferro_app_delegate::IFerroAppDelegate;
use ferroui_base::reactive::IDisposable;
use ferroui_controls::application_lifetimes::{ActivatableLifetimeBase, ActivatedEventArgs, IActivatableLifetime};
use std::rc::Rc;

/// Raises the activation events of the application delegate.
pub struct ActivatableLifetime {
    base: Rc<ActivatableLifetimeBase>,
    // The subscriptions of the reference live as long as the delegate; the
    // handles are kept so that they do here.
    _subscriptions: [Rc<dyn IDisposable>; 2],
}

impl ActivatableLifetime {
    /// Creates the lifetime over the events of `app_delegate`.
    pub fn new(app_delegate: &dyn IFerroAppDelegate) -> Rc<Self> {
        let base = ActivatableLifetimeBase::new();
        let activated = app_delegate.activated({
            let base = base.clone();
            Rc::new(move |args: &ActivatedEventArgs| base.on_activated(args.clone()))
        });
        let deactivated = app_delegate.deactivated({
            let base = base.clone();
            Rc::new(move |args: &ActivatedEventArgs| base.on_deactivated(args.clone()))
        });
        Rc::new(Self { base, _subscriptions: [activated, deactivated] })
    }
}

impl IActivatableLifetime for ActivatableLifetime {
    fn activated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
        self.base.activated(handler)
    }

    fn deactivated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
        self.base.deactivated(handler)
    }

    fn try_leave_background(&self) -> bool {
        self.base.try_leave_background()
    }

    fn try_enter_background(&self) -> bool {
        self.base.try_enter_background()
    }
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests of this file.
    use super::*;
    use crate::ferro_app_delegate::AppDelegateEvents;
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::application_lifetimes::ActivationKind;
    use std::cell::RefCell;

    #[test]
    fn the_events_of_the_delegate_are_the_events_of_the_lifetime() {
        let _scope = Dispatcher::unit_test_scope();
        let delegate = AppDelegateEvents::new();
        let lifetime = ActivatableLifetime::new(&*delegate);

        let log = Rc::new(RefCell::new(Vec::new()));
        let _activated = lifetime.activated({
            let log = log.clone();
            Rc::new(move |args: &ActivatedEventArgs| log.borrow_mut().push(("activated", args.kind())))
        });
        let _deactivated = lifetime.deactivated({
            let log = log.clone();
            Rc::new(move |args: &ActivatedEventArgs| log.borrow_mut().push(("deactivated", args.kind())))
        });

        delegate.on_deactivated(ActivatedEventArgs::new(ActivationKind::Background));
        delegate.on_activated(ActivatedEventArgs::new(ActivationKind::Background));

        assert_eq!(
            vec![("deactivated", ActivationKind::Background), ("activated", ActivationKind::Background)],
            *log.borrow()
        );
    }
}
