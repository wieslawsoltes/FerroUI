use crate::interop::*;

/// Answers a yes/no question of native code (is a menu item enabled?).
pub(crate) struct PredicateCallback {
    predicate: Box<dyn Fn() -> bool>,
}

impl PredicateCallback {
    pub(crate) fn new(predicate: impl Fn() -> bool + 'static) -> Self {
        Self { predicate: Box::new(predicate) }
    }
}

impl IFrnPredicateCallbackImpl for PredicateCallback {
    fn evaluate(&self) -> bool {
        crate::callback_base::guard(false, || (self.predicate)())
    }
}
