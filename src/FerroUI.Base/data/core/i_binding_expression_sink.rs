use crate::data::BindingExpressionBase;
use crate::{FerroObject, WeakRef};
use std::rc::{Rc, Weak};

/// Receives the notifications of a binding expression.
pub(crate) trait IBindingExpressionSink {
    /// Called when a binding expression's value or error state changes.
    fn on_changed(&self, instance: &Rc<dyn BindingExpressionBase>, has_value_changed: bool, has_error_changed: bool);

    /// Called when a binding expression completes.
    fn on_completed(&self, instance: &Rc<dyn BindingExpressionBase>);
}

/// A non-owning reference to a binding expression sink: the value store of
/// an object, or another sink (an observable adapter or a parent
/// expression).
#[derive(Clone)]
pub enum SinkRef {
    #[doc(hidden)]
    Store(WeakRef<FerroObject>),
    #[doc(hidden)]
    #[allow(private_interfaces)]
    Other(Weak<dyn IBindingExpressionSink>),
}

impl SinkRef {
    pub(crate) fn on_changed(
        &self,
        instance: &Rc<dyn BindingExpressionBase>,
        has_value_changed: bool,
        has_error_changed: bool,
    ) {
        match self {
            SinkRef::Store(owner) => {
                if let Some(owner) = owner.upgrade() {
                    owner.values().on_expression_changed(&owner, instance, has_value_changed, has_error_changed);
                }
            }
            SinkRef::Other(sink) => {
                if let Some(sink) = sink.upgrade() {
                    sink.on_changed(instance, has_value_changed, has_error_changed);
                }
            }
        }
    }

    pub(crate) fn on_completed(&self, instance: &Rc<dyn BindingExpressionBase>) {
        match self {
            SinkRef::Store(owner) => {
                if let Some(owner) = owner.upgrade() {
                    owner.values().on_expression_completed(&owner, instance);
                }
            }
            SinkRef::Other(sink) => {
                if let Some(sink) = sink.upgrade() {
                    sink.on_completed(instance);
                }
            }
        }
    }
}
