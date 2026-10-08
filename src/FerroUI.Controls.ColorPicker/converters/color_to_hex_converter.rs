use super::as_solid_color_brush;
use crate::AlphaComponentPosition;
use ferroui_base::data::converters::{cast_value, IValueConverter};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingError;
use ferroui_base::media::{Color, HslColor, HsvColor};
use ferroui_base::utilities::span_helpers::{try_parse_uint, NumberStyles};
use ferroui_base::{BoxedValue, FerroProperty};
use std::cell::Cell;
use std::rc::Rc;

/// Converts a color to a hex string and vice versa.
#[derive(Debug)]
pub struct ColorToHexConverter {
    is_alpha_visible: Cell<bool>,
    alpha_position: Cell<AlphaComponentPosition>,
}

impl Default for ColorToHexConverter {
    fn default() -> Self {
        Self::new()
    }
}

impl ColorToHexConverter {
    /// Initializes a new instance of the [`ColorToHexConverter`] class.
    pub fn new() -> Self {
        Self { is_alpha_visible: Cell::new(true), alpha_position: Cell::new(AlphaComponentPosition::Leading) }
    }

    /// Gets a value indicating whether the alpha component is visible in the Hex formatted text.
    ///
    /// When hidden the existing alpha component value is maintained. Also when hidden the user is still
    /// able to input an 8-digit number with alpha. Alpha will be processed but then removed when displayed.
    ///
    /// Because this property only controls whether alpha is displayed (and it is still processed regardless)
    /// it is termed 'Visible' instead of 'Enabled'.
    pub fn is_alpha_visible(&self) -> bool {
        self.is_alpha_visible.get()
    }

    /// Sets [`is_alpha_visible`](Self::is_alpha_visible).
    pub fn set_is_alpha_visible(&self, value: bool) {
        self.is_alpha_visible.set(value);
    }

    /// Gets the position of a color's alpha component relative to all other components.
    pub fn alpha_position(&self) -> AlphaComponentPosition {
        self.alpha_position.get()
    }

    /// Sets [`alpha_position`](Self::alpha_position).
    pub fn set_alpha_position(&self, value: AlphaComponentPosition) {
        self.alpha_position.set(value);
    }

    /// Converts the given color to its hex color value string representation.
    ///
    /// - `color`: the color to represent as a hex value string.
    /// - `alpha_position`: the output position of the alpha component.
    /// - `include_alpha`: whether the alpha component will be included in the hex string
    ///   (upstream's default is `true`).
    /// - `include_symbol`: whether the hex symbol '#' will be added (upstream's default is `false`).
    pub fn to_hex_string(
        color: Color,
        alpha_position: AlphaComponentPosition,
        include_alpha: bool,
        include_symbol: bool,
    ) -> String {
        let int_color: u32;
        let mut hex_color: String;

        if include_alpha {
            if alpha_position == AlphaComponentPosition::Trailing {
                int_color = ((color.r as u32) << 24) | ((color.g as u32) << 16) | ((color.b as u32) << 8) | color.a as u32;
            } else {
                // Default is Leading alpha (same as XAML)
                int_color = ((color.a as u32) << 24) | ((color.r as u32) << 16) | ((color.g as u32) << 8) | color.b as u32;
            }

            hex_color = format!("{int_color:08X}");
        } else {
            // In this case the alpha position no longer matters
            // Both cases are calculated the same
            int_color = ((color.r as u32) << 16) | ((color.g as u32) << 8) | color.b as u32;
            hex_color = format!("{int_color:06X}");
        }

        if include_symbol {
            hex_color.insert(0, '#');
        }

        hex_color
    }

    /// Parses a hex color value string into a new [`Color`].
    ///
    /// `hex_color` is the hex color string to parse (with or without the
    /// '#' symbol); `alpha_position` is the input position of the alpha
    /// component. Returns the parsed color; otherwise, `None`.
    pub fn parse_hex_string(hex_color: &str, alpha_position: AlphaComponentPosition) -> Option<Color> {
        let mut hex_color = hex_color.trim().to_string();

        if !hex_color.starts_with('#') {
            hex_color.insert(0, '#');
        }

        Self::try_parse_hex_format(&hex_color, alpha_position)
    }

    /// Parses the given span of characters representing a hex color value into a new [`Color`].
    ///
    /// This is based on the Color.TryParseHexFormat() method.
    /// It is copied because it needs to be extended to handle alpha position.
    /// However, the alpha position enum is only available in the controls namespace with the ColorPicker control.
    fn try_parse_hex_format(s: &str, alpha_position: AlphaComponentPosition) -> Option<Color> {
        fn try_parse_core(input: &[char], alpha_position: AlphaComponentPosition) -> Option<Color> {
            let mut alpha_component = 0u32;

            if input.len() == 6 {
                if alpha_position == AlphaComponentPosition::Trailing {
                    alpha_component = 0x000000FF;
                } else {
                    alpha_component = 0xFF000000;
                }
            } else if input.len() != 8 {
                return None;
            }

            let text: String = input.iter().collect();
            let mut parsed = try_parse_uint(&text, NumberStyles::HEX_NUMBER)?;

            if alpha_component != 0 {
                if alpha_position == AlphaComponentPosition::Trailing {
                    parsed = (parsed << 8) | alpha_component;
                } else {
                    parsed |= alpha_component;
                }
            }

            if alpha_position == AlphaComponentPosition::Trailing {
                // #RRGGBBAA
                Some(Color::new(
                    (parsed & 0xFF) as u8,
                    ((parsed >> 24) & 0xFF) as u8,
                    ((parsed >> 16) & 0xFF) as u8,
                    ((parsed >> 8) & 0xFF) as u8,
                ))
            } else {
                // #AARRGGBB
                Some(Color::new(
                    ((parsed >> 24) & 0xFF) as u8,
                    ((parsed >> 16) & 0xFF) as u8,
                    ((parsed >> 8) & 0xFF) as u8,
                    (parsed & 0xFF) as u8,
                ))
            }
        }

        let input: Vec<char> = s.chars().skip(1).collect();

        // Handle shorthand cases like #FFF (RGB) or #FFFF (ARGB).
        if input.len() == 3 || input.len() == 4 {
            let mut extended = Vec::with_capacity(2 * input.len());

            for &c in &input {
                extended.push(c);
                extended.push(c);
            }

            return try_parse_core(&extended, alpha_position);
        }

        try_parse_core(&input, alpha_position)
    }
}

impl IValueConverter for ColorToHexConverter {
    fn convert(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let color: Color;
        let include_symbol = parameter.and_then(|parameter| parameter.downcast_ref::<bool>().copied()).unwrap_or(false);

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

        Ok(Some(Rc::new(Self::to_hex_string(color, self.alpha_position(), self.is_alpha_visible(), include_symbol))))
    }

    fn convert_back(
        &self,
        value: Option<&BoxedValue>,
        _target_type: ValueType,
        _parameter: Option<&BoxedValue>,
    ) -> Result<Option<BoxedValue>, BindingError> {
        let hex_value = value.map(|value| ValueTypes::to_display_string(Some(value))).unwrap_or_default();
        Ok(Some(match Self::parse_hex_string(&hex_value, self.alpha_position()) {
            Some(color) => Rc::new(color),
            None => FerroProperty::unset_value(),
        }))
    }
}
