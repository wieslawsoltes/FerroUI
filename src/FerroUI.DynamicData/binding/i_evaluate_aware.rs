//! `IEvaluateAware` (`Binding/IEvaluateAware.cs`).

use std::rc::Rc;

/// An object that evaluates itself again when it is refreshed (`IEvaluateAware`), which
/// `invoke_evaluate` calls.
pub trait IEvaluateAware {
    /// Evaluates the object again (`Evaluate`).
    fn evaluate(&self);
}

impl<E: IEvaluateAware + ?Sized> IEvaluateAware for Rc<E> {
    fn evaluate(&self) {
        (**self).evaluate();
    }
}
