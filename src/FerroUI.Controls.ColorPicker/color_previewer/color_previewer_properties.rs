use super::ColorPreviewer;
use ferroui_base::data::BindingMode;
use ferroui_base::media::{Colors, HsvColor};
use ferroui_base::{ferro_properties, ferro_property, FerroProperty, StyledProperty, StyledPropertyOptions};

ferro_properties! { impl ColorPreviewer {
    ferro_property!(
        /// Defines the `HsvColor` property.
        pub fn hsv_color_property() -> StyledProperty<HsvColor> {
            FerroProperty::register_with::<ColorPreviewer, _>(
                "HsvColor",
                StyledPropertyOptions::new(Colors::TRANSPARENT.to_hsv()).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    ferro_property!(
        /// Defines the `IsAccentColorsVisible` property.
        pub fn is_accent_colors_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorPreviewer, _>("IsAccentColorsVisible", true)
        }
    );
} }

impl ColorPreviewer {
    /// Gets the currently previewed color in the HSV color model.
    ///
    /// Only an HSV color is supported in this control to ensure there is never any
    /// loss of precision or color information. Accent colors, like the color spectrum,
    /// only operate with the HSV color model.
    pub fn hsv_color(&self) -> HsvColor {
        self.get_value(Self::hsv_color_property())
    }

    /// Sets [`hsv_color`](Self::hsv_color).
    pub fn set_hsv_color(&self, value: HsvColor) {
        self.set_value(Self::hsv_color_property(), value)
    }

    /// Gets a value indicating whether accent colors are visible along
    /// with the preview color.
    pub fn is_accent_colors_visible(&self) -> bool {
        self.get_value(Self::is_accent_colors_visible_property())
    }

    /// Sets [`is_accent_colors_visible`](Self::is_accent_colors_visible).
    pub fn set_is_accent_colors_visible(&self, value: bool) {
        self.set_value(Self::is_accent_colors_visible_property(), value)
    }
}
