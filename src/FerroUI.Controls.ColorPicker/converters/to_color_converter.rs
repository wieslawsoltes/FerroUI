use ferroui_base::utilities::CultureInfo;
use super::as_solid_color_brush;
use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::media::{Color, HslColor, HsvColor};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{BoxedValue, FerroProperty};
use std::rc::Rc;

/// Converts the given value into a [`Color`] when a conversion is possible.
#[derive(Debug, Default)]
pub struct ToColorConverter;

impl ToColorConverter {
    /// Initializes a new instance of the [`ToColorConverter`] class.
    pub fn new() -> Self {
        Self
    }
}

impl IValueConverter for ToColorConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(value_color) = cast_value::<Color>(value) {
            return Ok(Some(Rc::new(value_color)));
        } else if let Some(value_hsl_color) = cast_value::<HslColor>(value) {
            return Ok(Some(Rc::new(value_hsl_color.to_rgb())));
        } else if let Some(value_hsv_color) = cast_value::<HsvColor>(value) {
            return Ok(Some(Rc::new(value_hsv_color.to_rgb())));
        } else if let Some(value_brush) = as_solid_color_brush(value) {
            // A brush may have an opacity set along with alpha transparency
            let color = value_brush.color();
            let alpha = color.a as f64 * value_brush.opacity();

            return Ok(Some(Rc::new(Color::new(
                MathUtilities::clamp(alpha, 0x00 as f64, 0xFF as f64) as u8,
                color.r,
                color.g,
                color.b,
            ))));
        }

        Ok(Some(FerroProperty::unset_value()))
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
