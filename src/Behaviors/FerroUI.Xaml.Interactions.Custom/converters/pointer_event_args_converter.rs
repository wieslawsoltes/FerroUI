//! Port of `Converters/PointerEventArgsConverter.cs`.

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingError;
use ferroui_base::input::{
    PointerDeltaEventArgs, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs, PointerWheelEventArgs,
};
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{BoxedValue, FerroProperty, Point, Visual};
use std::rc::Rc;

/// Converter for [`PointerEventArgs`].
///
/// The converted value is the position of the pointer relative to the source of the event, or the
/// delta of a delta or wheel event, as a pair `(x, y)` of `f64` (the value tuple of the
/// reference).
#[derive(Debug, Default)]
pub struct PointerEventArgsConverter;

impl PointerEventArgsConverter {
    /// Gets the instance of [`PointerEventArgsConverter`].
    pub fn instance() -> Rc<PointerEventArgsConverter> {
        ferroui_base::ferro_thread_local! {
            static INSTANCE: Rc<PointerEventArgsConverter> = Rc::new(PointerEventArgsConverter);
        }
        INSTANCE.with(Rc::clone)
    }

    /// Provides a cached instance of the converter (the markup extension form,
    /// `{PointerEventArgsConverter}`).
    pub fn provide_value(&self, _service_provider: &Rc<dyn IServiceProvider>) -> Rc<dyn IValueConverter> {
        Self::instance()
    }

    /// The position of the pointer relative to the source of the event, if the source is a
    /// visual (`GetPosition(visual)`).
    fn position_of(e: &PointerEventArgs) -> Option<BoxedValue> {
        let source = e.source()?;
        let visual = source.cast::<Visual>()?;
        let Point { x, y } = e.get_position(Some(&*visual));
        Some(Rc::new((x, y)))
    }

    fn convert_args(e: &dyn IRoutedEventArgs) -> Option<BoxedValue> {
        if let Some(pointer_pressed_event_args) = e.downcast_ref::<PointerPressedEventArgs>() {
            return Self::position_of(pointer_pressed_event_args);
        }

        if let Some(pointer_released_event_args) = e.downcast_ref::<PointerReleasedEventArgs>() {
            return Self::position_of(pointer_released_event_args);
        }

        if let Some(pointer_delta_event_args) = e.downcast_ref::<PointerDeltaEventArgs>() {
            let delta = pointer_delta_event_args.delta();
            return Some(Rc::new((delta.x, delta.y)));
        }

        if let Some(pointer_wheel_event_args) = e.downcast_ref::<PointerWheelEventArgs>() {
            let delta = pointer_wheel_event_args.delta();
            return Some(Rc::new((delta.x, delta.y)));
        }

        if let Some(pointer_event_args) = e.downcast_ref::<PointerEventArgs>() {
            return Self::position_of(pointer_event_args);
        }

        None
    }
}

// A converter is a reference type: an instance equals itself.
impl PartialEq for PointerEventArgsConverter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl IValueConverter for PointerEventArgsConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        // The args reach a converter as the shared handle of the live args (what a trigger passes
        // to its actions).
        let converted = value
            .and_then(|value| value.downcast_ref::<Rc<dyn IRoutedEventArgs>>())
            .and_then(|e| Self::convert_args(&**e));

        Ok(Some(converted.unwrap_or_else(FerroProperty::unset_value)))
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(FerroProperty::unset_value()))
    }
}

/// Registers the nullable handle of the converter and its cast to the converter contract.
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<Rc<PointerEventArgsConverter>>();
    ValueTypes::register_cast::<Rc<PointerEventArgsConverter>, Rc<dyn IValueConverter>>(|c| c.clone());
}
