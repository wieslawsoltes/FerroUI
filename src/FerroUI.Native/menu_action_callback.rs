use crate::interop::*;

/// Runs an action when native code activates a menu item.
pub(crate) struct MenuActionCallback {
    action: Box<dyn Fn()>,
}

impl MenuActionCallback {
    pub(crate) fn new(action: impl Fn() + 'static) -> Self {
        Self { action: Box::new(action) }
    }
}

impl IFrnActionCallbackImpl for MenuActionCallback {
    fn run(&self) {
        crate::callback_base::guard((), || (self.action)())
    }
}
