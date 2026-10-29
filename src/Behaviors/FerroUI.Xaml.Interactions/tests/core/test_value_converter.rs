//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/TestValueConverter.cs` of
//! Xaml.Behaviors.

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::BoxedValue;
use std::rc::Rc;

/// Converts routed event args to the name of the type of their source.
pub(crate) struct TestValueConverter;

impl TestValueConverter {
    /// `TestValueConverter.Instance`.
    pub(crate) fn instance() -> Rc<dyn IValueConverter> {
        Rc::new(TestValueConverter)
    }
}

impl IValueConverter for TestValueConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(args) = value.and_then(|value| value.downcast_ref::<Rc<dyn IRoutedEventArgs>>()) {
            let name = args.as_routed_event_args().source().map(|source| source.get_type().name().to_string());
            return Ok(name.map(|name| Rc::new(name) as BoxedValue));
        }

        Err(BindingError::message("Invalid value type"))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Err(BindingError::message("The method or operation is not implemented."))
    }
}
