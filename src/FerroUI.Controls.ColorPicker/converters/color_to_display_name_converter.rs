use ferroui_base::utilities::CultureInfo;
use super::as_solid_color_brush;
use crate::primitives::ColorHelper;
use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::media::{Color, HslColor, HsvColor};
use ferroui_base::{BoxedValue, FerroProperty};
use std::rc::Rc;

/// Gets the approximated display name for the color.
#[derive(Debug, Default)]
pub struct ColorToDisplayNameConverter;

impl ColorToDisplayNameConverter {
    /// Initializes a new instance of the [`ColorToDisplayNameConverter`] class.
    pub fn new() -> Self {
        Self
    }
}

impl IValueConverter for ColorToDisplayNameConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        _culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let color: Color;

        if let Some(value_color) = cast_value::<Color>(value) {
            color = value_color;
        } else if let Some(value_hsl_color) = cast_value::<HslColor>(value) {
            color = value_hsl_color.to_rgb();
        } else if let Some(value_hsv_color) = cast_value::<HsvColor>(value) {
            color = value_hsv_color.to_rgb();
        } else if let Some(value_brush) = as_solid_color_brush(value) {
            color = value_brush.color();
        } else {
            // Invalid color value provided
            return Ok(Some(FerroProperty::unset_value()));
        }

        // ColorHelper.ToDisplayName ignores the alpha component
        // This means fully transparent colors will be named as a real color
        // That undesirable behavior is specially overridden here
        if color.a == 0x00 {
            Ok(Some(FerroProperty::unset_value()))
        } else {
            Ok(Some(Rc::new(ColorHelper::to_display_name(color))))
        }
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
