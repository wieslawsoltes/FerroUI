use super::{as_solid_color_brush, solid_color_brush};
use crate::helpers::round_digits;
use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingError;
use ferroui_base::media::{Color, HslColor, HsvColor};
use ferroui_base::utilities::span_helpers::{try_parse_int, NumberStyles};
use ferroui_base::{BoxedValue, FerroProperty};

/// Creates an accent color for a given base color value and step parameter.
///
/// This is a highly-specialized converter for the color picker.
#[derive(Debug, Default)]
pub struct AccentColorConverter;

impl AccentColorConverter {
    /// The amount to change the Value component for each accent color step.
    pub const VALUE_DELTA: f64 = 0.1;

    /// Initializes a new instance of the [`AccentColorConverter`] class.
    pub fn new() -> Self {
        Self
    }

    /// This does not account for perceptual differences and also does not match with
    /// system accent color calculation.
    ///
    /// Use the HSV representation as it's more perceptual.
    /// In most cases only the value is changed by a fixed percentage so the algorithm is reproducible.
    ///
    /// `hsv_color` is the base color to calculate the accent from; `accent_step`
    /// is the number of accent color steps to move (positive or negative).
    pub fn get_accent(hsv_color: HsvColor, accent_step: i32) -> HsvColor {
        if accent_step != 0 {
            let mut color_value = hsv_color.v;
            color_value += accent_step as f64 * AccentColorConverter::VALUE_DELTA;
            color_value = round_digits(color_value, 2, false);

            HsvColor::new(hsv_color.a, hsv_color.h, hsv_color.s, color_value)
        } else {
            hsv_color
        }
    }
}

impl IValueConverter for AccentColorConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let mut rgb_color: Option<Color> = None;
        let mut hsv_color: Option<HsvColor> = None;

        if let Some(value_color) = cast_value::<Color>(value) {
            rgb_color = Some(value_color);
        } else if let Some(value_hsl_color) = cast_value::<HslColor>(value) {
            rgb_color = Some(value_hsl_color.to_rgb());
        } else if let Some(value_hsv_color) = cast_value::<HsvColor>(value) {
            hsv_color = Some(value_hsv_color);
        } else if let Some(value_brush) = as_solid_color_brush(value) {
            rgb_color = Some(value_brush.color());
        } else {
            // Invalid color value provided
            return Ok(Some(FerroProperty::unset_value()));
        }

        // Get the value component delta
        let text = parameter.map(|parameter| ValueTypes::to_display_string(Some(parameter))).unwrap_or_default();
        let Some(accent_step) = try_parse_int(&text, NumberStyles::INTEGER) else {
            // Invalid parameter provided, unable to convert to integer
            return Ok(Some(FerroProperty::unset_value()));
        };

        if hsv_color.is_none() {
            if let Some(rgb_color) = rgb_color {
                hsv_color = Some(rgb_color.to_hsv());
            }
        }

        match hsv_color {
            Some(hsv_color) => Ok(Some(solid_color_brush(Self::get_accent(hsv_color, accent_step).to_rgb()))),
            None => Ok(Some(FerroProperty::unset_value())),
        }
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(FerroProperty::unset_value()))
    }
}
