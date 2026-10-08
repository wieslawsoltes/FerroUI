use super::ColorSlider;
use crate::{ColorComponent, ColorModel};
use ferroui_base::data::BindingMode;
use ferroui_base::media::{Color, Colors, HsvColor};
use ferroui_base::{ferro_properties, ferro_property, FerroProperty, StyledProperty, StyledPropertyOptions};

ferro_properties! { impl ColorSlider {
    ferro_property!(
        /// Defines the `Color` property.
        pub fn color_property() -> StyledProperty<Color> {
            FerroProperty::register_with::<ColorSlider, _>(
                "Color",
                StyledPropertyOptions::new(Colors::WHITE).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    ferro_property!(
        /// Defines the `ColorComponent` property.
        pub fn color_component_property() -> StyledProperty<ColorComponent> {
            FerroProperty::register::<ColorSlider, _>("ColorComponent", ColorComponent::Component1)
        }
    );

    ferro_property!(
        /// Defines the `ColorModel` property.
        pub fn color_model_property() -> StyledProperty<ColorModel> {
            FerroProperty::register::<ColorSlider, _>("ColorModel", ColorModel::Rgba)
        }
    );

    ferro_property!(
        /// Defines the `HsvColor` property.
        pub fn hsv_color_property() -> StyledProperty<HsvColor> {
            FerroProperty::register_with::<ColorSlider, _>(
                "HsvColor",
                StyledPropertyOptions::new(Colors::WHITE.to_hsv()).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    ferro_property!(
        /// Defines the `IsAlphaVisible` property.
        pub fn is_alpha_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorSlider, _>("IsAlphaVisible", false)
        }
    );

    ferro_property!(
        /// Defines the `IsPerceptive` property.
        pub fn is_perceptive_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorSlider, _>("IsPerceptive", true)
        }
    );

    ferro_property!(
        /// Defines the `IsRoundingEnabled` property.
        pub fn is_rounding_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorSlider, _>("IsRoundingEnabled", false)
        }
    );
} }

impl ColorSlider {
    /// Gets the currently selected color in the RGB color model.
    ///
    /// Use this property instead of [`hsv_color`](Self::hsv_color) when in
    /// [`ColorModel::Rgba`] to avoid loss of precision and color drifting.
    pub fn color(&self) -> Color {
        self.get_value(Self::color_property())
    }

    /// Sets [`color`](Self::color).
    pub fn set_color(&self, value: Color) {
        self.set_value(Self::color_property(), value)
    }

    /// Gets the color component represented by the slider.
    pub fn color_component(&self) -> ColorComponent {
        self.get_value(Self::color_component_property())
    }

    /// Sets [`color_component`](Self::color_component).
    pub fn set_color_component(&self, value: ColorComponent) {
        self.set_value(Self::color_component_property(), value)
    }

    /// Gets the active color model used by the slider.
    pub fn color_model(&self) -> ColorModel {
        self.get_value(Self::color_model_property())
    }

    /// Sets [`color_model`](Self::color_model).
    pub fn set_color_model(&self, value: ColorModel) {
        self.set_value(Self::color_model_property(), value)
    }

    /// Gets the currently selected color in the HSV color model.
    ///
    /// Use this property instead of [`color`](Self::color) when in
    /// [`ColorModel::Hsva`] to avoid loss of precision and color drifting.
    pub fn hsv_color(&self) -> HsvColor {
        self.get_value(Self::hsv_color_property())
    }

    /// Sets [`hsv_color`](Self::hsv_color).
    pub fn set_hsv_color(&self, value: HsvColor) {
        self.set_value(Self::hsv_color_property(), value)
    }

    /// Gets a value indicating whether the alpha component is visible and rendered.
    /// When false, this ensures that the gradient is always visible and never transparent regardless of
    /// the actual color. This property is ignored when the alpha component itself is being displayed.
    ///
    /// Setting to false means the alpha component is always forced to maximum for components other than
    /// [`color_component`](Self::color_component) during rendering. This doesn't change the value of the
    /// alpha component in the color – it is only for display.
    pub fn is_alpha_visible(&self) -> bool {
        self.get_value(Self::is_alpha_visible_property())
    }

    /// Sets [`is_alpha_visible`](Self::is_alpha_visible).
    pub fn set_is_alpha_visible(&self, value: bool) {
        self.set_value(Self::is_alpha_visible_property(), value)
    }

    /// Gets a value indicating whether the slider adapts rendering to improve user-perception
    /// over exactness.
    ///
    /// When true in the HSVA color model, this ensures that the gradient is always visible and
    /// never washed out regardless of the actual color. When true in the RGBA color model, this ensures
    /// the gradient always appears as red, green or blue.
    ///
    /// For example, with Hue in the HSVA color model, the Saturation and Value components are always forced
    /// to maximum values during rendering. In the RGBA color model, all components other than
    /// [`color_component`](Self::color_component) are forced to minimum values during rendering.
    ///
    /// Note this property will only adjust components other than [`color_component`](Self::color_component)
    /// during rendering. This also doesn't change the values of any components in the actual color – it is
    /// only for display.
    pub fn is_perceptive(&self) -> bool {
        self.get_value(Self::is_perceptive_property())
    }

    /// Sets [`is_perceptive`](Self::is_perceptive).
    pub fn set_is_perceptive(&self, value: bool) {
        self.set_value(Self::is_perceptive_property(), value)
    }

    /// Gets a value indicating whether rounding of color component values is enabled.
    ///
    /// This is applicable for the HSV color model only. The [`HsvColor`] struct uses double
    /// values while the [`Color`] struct uses byte. Only double types need rounding.
    pub fn is_rounding_enabled(&self) -> bool {
        self.get_value(Self::is_rounding_enabled_property())
    }

    /// Sets [`is_rounding_enabled`](Self::is_rounding_enabled).
    pub fn set_is_rounding_enabled(&self, value: bool) {
        self.set_value(Self::is_rounding_enabled_property(), value)
    }
}
