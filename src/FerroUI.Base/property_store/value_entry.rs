use crate::data::{BindingError, BindingValueType};
use crate::{BoxedValue, FerroProperty};
use std::any::Any;
use std::rc::Rc;

/// An entry in a value frame: a value, or a source of values, for a property.
///
/// The trait is public so that a binding expression defined in another crate
/// can be an entry of the value store; it is implemented with
/// [`impl_untyped_binding_expression!`](crate::impl_untyped_binding_expression),
/// not by hand.
pub trait IValueEntry {
    /// The property that this value applies to.
    fn property(&self) -> &'static FerroProperty;

    /// Checks whether the entry has a value. May subscribe the entry to its
    /// source.
    fn has_value(&self) -> bool;

    /// Writes the value into `out`, which must be an `&mut Option<T>` where
    /// `T` is the property's value type. Returns false if the entry has no
    /// value.
    fn try_get_value(&self, out: &mut dyn Any) -> bool;

    /// Gets the value as an untyped value.
    fn get_value_boxed(&self) -> Option<BoxedValue>;

    /// The data validation state, if the entry supports data validation.
    fn get_data_validation_state(&self) -> Option<(BindingValueType, Option<BindingError>)> {
        None
    }

    /// Called when the entry no longer contributes to the effective value.
    fn unsubscribe(&self);

    /// The entry as a binding expression, if it is one.
    fn as_binding_expression(self: Rc<Self>) -> Option<Rc<dyn crate::data::BindingExpressionBase>> {
        None
    }
}

/// Identity comparison of two value entries.
#[inline]
pub(crate) fn entry_ptr_eq(a: &Rc<dyn IValueEntry>, b: &Rc<dyn IValueEntry>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}
