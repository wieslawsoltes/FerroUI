use ferroui_base::reactive::IDisposable;
use ferroui_controls::application_lifetimes::{
    ActivatableLifetimeBase, ActivatedEventArgs, ActivationKind, IActivatableLifetime,
};
use std::rc::Rc;

/// Activation of the application follows the visibility of the page.
pub struct BrowserActivatableLifetime {
    base: Rc<ActivatableLifetimeBase>,
}

impl Default for BrowserActivatableLifetime {
    fn default() -> Self {
        Self { base: ActivatableLifetimeBase::new() }
    }
}

impl BrowserActivatableLifetime {
    /// Creates the lifetime.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// The page reported its visibility state (`"visible"`, `"hidden"`).
    pub fn on_visibility_state_changed(&self, visibility_state: &str) {
        let visible = visibility_state == "visible";
        if visible {
            self.base.on_activated_kind(ActivationKind::Background);
        } else {
            self.base.on_deactivated_kind(ActivationKind::Background);
        }
    }
}

impl IActivatableLifetime for BrowserActivatableLifetime {
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
