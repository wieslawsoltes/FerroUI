use ferroui_base::utilities::CultureInfo;
use super::{solid_color_brush, ToColorConverter};
use crate::primitives::ColorHelper;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingError;
use ferroui_base::media::{Color, Colors};
use ferroui_base::{BoxedValue, FerroProperty};
use std::cell::Cell;

/// Gets a solid color brush, either black or white, depending on the luminance of the supplied color.
/// A default color supplied in the converter parameter may be returned if alpha is below the set threshold.
///
/// This is a highly-specialized converter for the color picker.
#[derive(Debug)]
pub struct ContrastBrushConverter {
    to_color_converter: ToColorConverter,
    alpha_threshold: Cell<u8>,
}

impl Default for ContrastBrushConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl ContrastBrushConverter {
    /// Initializes a new instance of the [`ContrastBrushConverter`] class.
    pub fn new() -> Self {
        Self { to_color_converter: ToColorConverter::new(), alpha_threshold: Cell::new(128) }
    }

    /// Gets the alpha channel threshold below which a default color is used instead of black/white.
    pub fn alpha_threshold(&self) -> u8 {
        self.alpha_threshold.get()
    }

    /// Sets [`alpha_threshold`](Self::alpha_threshold).
    pub fn set_alpha_threshold(&self, value: u8) {
        self.alpha_threshold.set(value);
    }
}

impl IValueConverter for ContrastBrushConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        target_type: ValueType,
        parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let comparison_color: Color;
        let mut default_color: Option<Color> = None;

        // Get the changing color to compare against
        let converted_value = self.to_color_converter.convert(value, target_type, parameter, culture)?;
        if let Some(value_color) = converted_value.as_ref().and_then(|value| value.downcast_ref::<Color>()) {
            comparison_color = *value_color;
        } else {
            // Invalid color value provided
            return Ok(Some(FerroProperty::unset_value()));
        }

        // Get the default color when transparency is high
        let converted_parameter = self.to_color_converter.convert(parameter, target_type, parameter, culture)?;
        if let Some(parameter_color) = converted_parameter.as_ref().and_then(|value| value.downcast_ref::<Color>()) {
            default_color = Some(*parameter_color);
        }

        match default_color {
            Some(default_color) if comparison_color.a < self.alpha_threshold() => {
                // If the transparency is less than the threshold, just use the default brush
                // This can commonly be something like the TextControlForeground brush
                Ok(Some(solid_color_brush(default_color)))
            }
            _ => {
                // Chose a white/black brush based on contrast to the base color
                if ColorHelper::get_relative_luminance(comparison_color) <= 0.5 {
                    // Dark color, return light for contrast
                    Ok(Some(solid_color_brush(Colors::WHITE)))
                } else {
                    // Bright color, return dark for contrast
                    Ok(Some(solid_color_brush(Colors::BLACK)))
                }
            }
        }
    }

    fn convert_back(
        &self,
        _value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
        culture: &CultureInfo,
    ) -> Result<Option<BoxedValue>, BindingError> {
        Ok(Some(FerroProperty::unset_value()))
    }
}
