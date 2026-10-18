//! `Converters/IndentConverter.cs`: converts the indent level of an expander cell to the left
//! padding of its content.

use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{BoxedValue, Thickness};
use std::rc::Rc;

/// Converts an indent level (an `i32`) to a [`Thickness`] with a left edge of 20 pixels per level.
#[derive(Debug, Default)]
pub struct IndentConverter;

impl IndentConverter {
    /// The shared instance of the converter (`IndentConverter.Instance`).
    pub fn instance() -> Rc<IndentConverter> {
        ferroui_base::ferro_thread_local! {
            static INSTANCE: Rc<IndentConverter> = Rc::new(IndentConverter);
        }
        INSTANCE.with(Rc::clone)
    }
}

impl IValueConverter for IndentConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(indent) = value.and_then(|value| value.downcast_ref::<i32>().copied()) {
            return Ok(Some(Rc::new(Thickness::new(20.0 * f64::from(indent), 0.0, 0.0, 0.0))));
        }

        Ok(Some(Rc::new(Thickness::default())))
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
