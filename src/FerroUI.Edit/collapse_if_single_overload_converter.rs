use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{BoxedValue, FerroProperty};
use std::rc::Rc;

/// One-way visibility converter for overload navigation in custom templates.
#[derive(Clone, Copy, Debug, Default)]
pub struct CollapseIfSingleOverloadConverter;
impl IValueConverter for CollapseIfSingleOverloadConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let visible = value.and_then(|value| {
            value
                .downcast_ref::<usize>()
                .map(|value| *value >= 2)
                .or_else(|| value.downcast_ref::<i32>().map(|value| *value >= 2))
        });
        Ok(Some(
            visible
                .map(|value| Rc::new(value) as BoxedValue)
                .unwrap_or_else(FerroProperty::unset_value),
        ))
    }
    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message(
            "Overload visibility is a one-way conversion",
        ))
    }
}
