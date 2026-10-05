use crate::data::BindingExpressionBase;
use crate::{BoxedValue, DoNothingType, FerroObject, FerroProperty};
use std::rc::Rc;

/// Static members related to bindings.
pub struct BindingOperations;

impl BindingOperations {
    /// The marker value that tells a binding target to do nothing: the
    /// binding value is ignored.
    pub fn do_nothing() -> BoxedValue {
        thread_local! {
            static DO_NOTHING: BoxedValue = Rc::new(DoNothingType);
        }
        DO_NOTHING.with(Rc::clone)
    }

    /// Whether `value` is the do-nothing marker.
    #[inline]
    pub fn is_do_nothing(value: Option<&BoxedValue>) -> bool {
        value.is_some_and(|v| v.is::<DoNothingType>())
    }

    /// Whether `value` is the unset marker.
    #[inline]
    pub fn is_unset(value: Option<&BoxedValue>) -> bool {
        value.is_some_and(|v| v.is::<crate::UnsetValueType>())
    }

    /// Retrieves the binding expression that is currently active on the
    /// specified property, if any.
    pub fn get_binding_expression_base(
        target: &FerroObject,
        property: &'static FerroProperty,
    ) -> Option<Rc<dyn BindingExpressionBase>> {
        target.get_expression(property)
    }
}
