// This source file is adapted from the WinUI project.
// (https://github.com/microsoft/microsoft-ui-xaml)

use super::ColorSpectrum;
use crate::{ColorComponent, ColorSpectrumComponents, ColorSpectrumShape};
use ferroui_base::data::BindingMode;
use ferroui_base::media::{Color, Colors, HsvColor};
use ferroui_base::{
    ferro_properties, ferro_property, DirectProperty, FerroProperty, StyledProperty,
    StyledPropertyOptions,
};

ferro_properties! { impl ColorSpectrum {
    ferro_property!(
        /// Defines the `Color` property.
        pub fn color_property() -> StyledProperty<Color> {
            FerroProperty::register_with::<ColorSpectrum, _>(
                "Color",
                StyledPropertyOptions::new(Colors::WHITE).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    ferro_property!(
        /// Defines the `Components` property.
        pub fn components_property() -> StyledProperty<ColorSpectrumComponents> {
            FerroProperty::register::<ColorSpectrum, _>("Components", ColorSpectrumComponents::HueSaturation)
        }
    );

    ferro_property!(
        /// Defines the `HsvColor` property.
        pub fn hsv_color_property() -> StyledProperty<HsvColor> {
            FerroProperty::register_with::<ColorSpectrum, _>(
                "HsvColor",
                StyledPropertyOptions::new(Colors::WHITE.to_hsv()).default_binding_mode(BindingMode::TwoWay),
            )
        }
    );

    ferro_property!(
        /// Defines the `MaxHue` property.
        pub fn max_hue_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorSpectrum, _>("MaxHue", 359)
        }
    );

    ferro_property!(
        /// Defines the `MaxSaturation` property.
        pub fn max_saturation_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorSpectrum, _>("MaxSaturation", 100)
        }
    );

    ferro_property!(
        /// Defines the `MaxValue` property.
        pub fn max_value_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorSpectrum, _>("MaxValue", 100)
        }
    );

    ferro_property!(
        /// Defines the `MinHue` property.
        pub fn min_hue_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorSpectrum, _>("MinHue", 0)
        }
    );

    ferro_property!(
        /// Defines the `MinSaturation` property.
        pub fn min_saturation_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorSpectrum, _>("MinSaturation", 0)
        }
    );

    ferro_property!(
        /// Defines the `MinValue` property.
        pub fn min_value_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorSpectrum, _>("MinValue", 0)
        }
    );

    ferro_property!(
        /// Defines the `Shape` property.
        pub fn shape_property() -> StyledProperty<ColorSpectrumShape> {
            FerroProperty::register::<ColorSpectrum, _>("Shape", ColorSpectrumShape::Box)
        }
    );

    ferro_property!(
        /// Defines the `ThirdComponent` property.
        pub fn third_component_property() -> DirectProperty<ColorSpectrum, ColorComponent> {
            FerroProperty::register_direct::<ColorSpectrum, _>(
                "ThirdComponent",
                |o| o.third_component(),
                None,
                // The unset value of the original is `default(ColorComponent)`.
                ColorComponent::Alpha,
            )
        }
    );
} }

impl ColorSpectrum {
    /// Gets the currently selected color in the RGB color model.
    ///
    /// For control authors, use [`hsv_color`](Self::hsv_color) instead to avoid loss
    /// of precision and color drifting.
    pub fn color(&self) -> Color {
        self.get_value(Self::color_property())
    }

    /// Sets [`color`](Self::color).
    pub fn set_color(&self, value: Color) {
        self.set_value(Self::color_property(), value)
    }

    /// Gets the two HSV color components displayed by the spectrum.
    ///
    /// Internally, the [`ColorSpectrum`] uses the HSV color model.
    pub fn components(&self) -> ColorSpectrumComponents {
        self.get_value(Self::components_property())
    }

    /// Sets [`components`](Self::components).
    pub fn set_components(&self, value: ColorSpectrumComponents) {
        self.set_value(Self::components_property(), value)
    }

    /// Gets the currently selected color in the HSV color model.
    ///
    /// This should be used in all cases instead of the [`color`](Self::color) property.
    /// Internally, the [`ColorSpectrum`] uses the HSV color model and using
    /// this property will avoid loss of precision and color drifting.
    pub fn hsv_color(&self) -> HsvColor {
        self.get_value(Self::hsv_color_property())
    }

    /// Sets [`hsv_color`](Self::hsv_color).
    pub fn set_hsv_color(&self, value: HsvColor) {
        self.set_value(Self::hsv_color_property(), value)
    }

    /// Gets the maximum value of the Hue component in the range from 0..359.
    /// This property must be greater than [`min_hue`](Self::min_hue).
    ///
    /// Internally, the [`ColorSpectrum`] uses the HSV color model.
    pub fn max_hue(&self) -> i32 {
        self.get_value(Self::max_hue_property())
    }

    /// Sets [`max_hue`](Self::max_hue).
    pub fn set_max_hue(&self, value: i32) {
        self.set_value(Self::max_hue_property(), value)
    }

    /// Gets the maximum value of the Saturation component in the range from 0..100.
    /// This property must be greater than [`min_saturation`](Self::min_saturation).
    ///
    /// Internally, the [`ColorSpectrum`] uses the HSV color model.
    pub fn max_saturation(&self) -> i32 {
        self.get_value(Self::max_saturation_property())
    }

    /// Sets [`max_saturation`](Self::max_saturation).
    pub fn set_max_saturation(&self, value: i32) {
        self.set_value(Self::max_saturation_property(), value)
    }

    /// Gets the maximum value of the Value component in the range from 0..100.
    /// This property must be greater than [`min_value`](Self::min_value).
    ///
    /// Internally, the [`ColorSpectrum`] uses the HSV color model.
    pub fn max_value(&self) -> i32 {
        self.get_value(Self::max_value_property())
    }

    /// Sets [`max_value`](Self::max_value).
    pub fn set_max_value(&self, value: i32) {
        self.set_value(Self::max_value_property(), value)
    }

    /// Gets the minimum value of the Hue component in the range from 0..359.
    /// This property must be less than [`max_hue`](Self::max_hue).
    ///
    /// Internally, the [`ColorSpectrum`] uses the HSV color model.
    pub fn min_hue(&self) -> i32 {
        self.get_value(Self::min_hue_property())
    }

    /// Sets [`min_hue`](Self::min_hue).
    pub fn set_min_hue(&self, value: i32) {
        self.set_value(Self::min_hue_property(), value)
    }

    /// Gets the minimum value of the Saturation component in the range from 0..100.
    /// This property must be less than [`max_saturation`](Self::max_saturation).
    ///
    /// Internally, the [`ColorSpectrum`] uses the HSV color model.
    pub fn min_saturation(&self) -> i32 {
        self.get_value(Self::min_saturation_property())
    }

    /// Sets [`min_saturation`](Self::min_saturation).
    pub fn set_min_saturation(&self, value: i32) {
        self.set_value(Self::min_saturation_property(), value)
    }

    /// Gets the minimum value of the Value component in the range from 0..100.
    /// This property must be less than [`max_value`](Self::max_value).
    ///
    /// Internally, the [`ColorSpectrum`] uses the HSV color model.
    pub fn min_value(&self) -> i32 {
        self.get_value(Self::min_value_property())
    }

    /// Sets [`min_value`](Self::min_value).
    pub fn set_min_value(&self, value: i32) {
        self.set_value(Self::min_value_property(), value)
    }

    /// Gets the displayed shape of the spectrum.
    pub fn shape(&self) -> ColorSpectrumShape {
        self.get_value(Self::shape_property())
    }

    /// Sets [`shape`](Self::shape).
    pub fn set_shape(&self, value: ColorSpectrumShape) {
        self.set_value(Self::shape_property(), value)
    }

    /// Gets the third HSV color component that is NOT displayed by the spectrum.
    /// This is automatically calculated from the [`components`](Self::components) property.
    ///
    /// This property should be used for any external color slider that represents the
    /// third component of the color. Note that this property uses the generic
    /// [`ColorComponent`] type instead of the more accurate [`HsvComponent`](crate::HsvComponent)
    /// to allow direct usage by the generalized color sliders.
    pub fn third_component(&self) -> ColorComponent {
        self.third_component.get()
    }

    pub(super) fn set_third_component(&self, value: ColorComponent) {
        self.set_and_raise_cell(Self::third_component_property(), &self.third_component, value);
    }
}
