use ferroui_base::utilities::CultureInfo;
use super::{as_brush, solid_color_brush};
use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::media::{Color, HslColor, HsvColor};
use ferroui_base::{BoxedValue, FerroProperty};
use std::rc::Rc;

/// Converts the given value into an [`IBrush`](ferroui_base::media::IBrush) when a conversion is possible.
#[derive(Debug, Default)]
pub struct ToBrushConverter;

impl ToBrushConverter {
    /// Initializes a new instance of the [`ToBrushConverter`] class.
    pub fn new() -> Self {
        Self
    }
}

impl IValueConverter for ToBrushConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        if let Some(brush) = as_brush(value) {
            return Ok(Some(Rc::new(brush)));
        } else if let Some(value_color) = cast_value::<Color>(value) {
            return Ok(Some(solid_color_brush(value_color)));
        } else if let Some(value_hsl_color) = cast_value::<HslColor>(value) {
            return Ok(Some(solid_color_brush(value_hsl_color.to_rgb())));
        } else if let Some(value_hsv_color) = cast_value::<HsvColor>(value) {
            return Ok(Some(solid_color_brush(value_hsv_color.to_rgb())));
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
