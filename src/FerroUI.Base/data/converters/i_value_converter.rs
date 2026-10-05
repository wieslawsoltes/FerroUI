use crate::data::core::ValueType;
use crate::data::BindingError;
use crate::BoxedValue;

/// Converts a binding value.
///
/// Values are untyped: `None` is null, and a converter may return the unset
/// marker ([`FerroProperty::unset_value`](crate::FerroProperty::unset_value))
/// to make the binding use its fallback value, the do-nothing marker
/// ([`BindingOperations::do_nothing`](crate::data::BindingOperations::do_nothing))
/// to leave the target untouched, or a boxed
/// [`BindingNotification`](crate::data::BindingNotification) to report an
/// error. Returning `Err` is the equivalent of throwing from the converter:
/// the binding logs the error and treats the value as unset.
///
/// There is no culture argument: formatting and parsing are invariant.
pub trait IValueConverter {
    /// Converts a value on its way from the binding source to the target.
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError>;

    /// Converts a value on its way from the binding target to the source.
    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError>;
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn IValueConverter {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
