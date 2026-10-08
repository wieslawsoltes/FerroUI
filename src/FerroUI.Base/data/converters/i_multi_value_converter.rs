use crate::data::core::ValueType;
use crate::data::BindingError;
use crate::utilities::CultureInfo;
use crate::BoxedValue;

/// Converts the values of a multi-binding to a single value.
///
/// See [`IValueConverter`](super::IValueConverter) for the conventions on
/// untyped values and errors.
pub trait IMultiValueConverter {
    /// Converts multi-binding values to a value for the target.
    fn convert(
        &self,
        values: &[Option<BoxedValue>],
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError>;
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn IMultiValueConverter {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
