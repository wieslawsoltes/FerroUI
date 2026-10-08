use super::ColorView;
use crate::color_palettes::IColorPalette;
use crate::{AlphaComponentPosition, ColorModel, ColorSpectrumComponents, ColorSpectrumShape, ColorViewTab};
use ferroui_base::collections::FerroList;
use ferroui_base::data::BindingMode;
use ferroui_base::media::{Color, Colors, HsvColor};
use ferroui_base::{ferro_properties, ferro_property, FerroObject, FerroProperty, StyledProperty, StyledPropertyOptions};
use std::rc::Rc;

ferro_properties! { impl ColorView {
    ferro_property!(
        /// Defines the `Color` property.
        pub fn color_property() -> StyledProperty<Color> {
            FerroProperty::register_with::<ColorView, _>(
                "Color",
                StyledPropertyOptions::new(Colors::WHITE)
                    .default_binding_mode(BindingMode::TwoWay)
                    .coerce(|instance: &FerroObject, value| ColorView::coerce_color(instance, value)),
            )
        }
    );

    ferro_property!(
        /// Defines the `ColorModel` property.
        pub fn color_model_property() -> StyledProperty<ColorModel> {
            FerroProperty::register::<ColorView, _>("ColorModel", ColorModel::Rgba)
        }
    );

    ferro_property!(
        /// Defines the `ColorSpectrumComponents` property.
        pub fn color_spectrum_components_property() -> StyledProperty<ColorSpectrumComponents> {
            FerroProperty::register::<ColorView, _>("ColorSpectrumComponents", ColorSpectrumComponents::HueSaturation)
        }
    );

    ferro_property!(
        /// Defines the `ColorSpectrumShape` property.
        pub fn color_spectrum_shape_property() -> StyledProperty<ColorSpectrumShape> {
            FerroProperty::register::<ColorView, _>("ColorSpectrumShape", ColorSpectrumShape::Box)
        }
    );

    ferro_property!(
        /// Defines the `HexInputAlphaPosition` property.
        pub fn hex_input_alpha_position_property() -> StyledProperty<AlphaComponentPosition> {
            FerroProperty::register::<ColorView, _>("HexInputAlphaPosition", AlphaComponentPosition::Leading) // By default match XAML and the WinUI control
        }
    );

    ferro_property!(
        /// Defines the `HsvColor` property.
        pub fn hsv_color_property() -> StyledProperty<HsvColor> {
            FerroProperty::register_with::<ColorView, _>(
                "HsvColor",
                StyledPropertyOptions::new(Colors::WHITE.to_hsv())
                    .default_binding_mode(BindingMode::TwoWay)
                    .coerce(|instance: &FerroObject, value| ColorView::coerce_hsv_color(instance, value)),
            )
        }
    );

    ferro_property!(
        /// Defines the `IsAccentColorsVisible` property.
        pub fn is_accent_colors_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorView, _>("IsAccentColorsVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsAlphaEnabled` property.
        pub fn is_alpha_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorView, _>("IsAlphaEnabled", true)
        }
    );

    ferro_property!(
        /// Defines the `IsAlphaVisible` property.
        pub fn is_alpha_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorView, _>("IsAlphaVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsColorComponentsVisible` property.
        pub fn is_color_components_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorView, _>("IsColorComponentsVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsColorModelVisible` property.
        pub fn is_color_model_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorView, _>("IsColorModelVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsColorPaletteVisible` property.
        pub fn is_color_palette_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorView, _>("IsColorPaletteVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsColorPreviewVisible` property.
        pub fn is_color_preview_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorView, _>("IsColorPreviewVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsColorSpectrumVisible` property.
        pub fn is_color_spectrum_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorView, _>("IsColorSpectrumVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsColorSpectrumSliderVisible` property.
        pub fn is_color_spectrum_slider_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorView, _>("IsColorSpectrumSliderVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsComponentSliderVisible` property.
        pub fn is_component_slider_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorView, _>("IsComponentSliderVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsComponentTextInputVisible` property.
        pub fn is_component_text_input_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorView, _>("IsComponentTextInputVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsHexInputVisible` property.
        pub fn is_hex_input_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<ColorView, _>("IsHexInputVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `MaxHue` property.
        pub fn max_hue_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorView, _>("MaxHue", 359)
        }
    );

    ferro_property!(
        /// Defines the `MaxSaturation` property.
        pub fn max_saturation_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorView, _>("MaxSaturation", 100)
        }
    );

    ferro_property!(
        /// Defines the `MaxValue` property.
        pub fn max_value_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorView, _>("MaxValue", 100)
        }
    );

    ferro_property!(
        /// Defines the `MinHue` property.
        pub fn min_hue_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorView, _>("MinHue", 0)
        }
    );

    ferro_property!(
        /// Defines the `MinSaturation` property.
        pub fn min_saturation_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorView, _>("MinSaturation", 0)
        }
    );

    ferro_property!(
        /// Defines the `MinValue` property.
        pub fn min_value_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorView, _>("MinValue", 0)
        }
    );

    ferro_property!(
        /// Defines the `PaletteColors` property.
        // Deviation (DEVIATIONS.md, Colour picker): upstream types it `IEnumerable<Color>?`.
        pub fn palette_colors_property() -> StyledProperty<Option<FerroList<Color>>> {
            FerroProperty::register::<ColorView, _>("PaletteColors", None)
        }
    );

    ferro_property!(
        /// Defines the `PaletteColumnCount` property.
        pub fn palette_column_count_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorView, _>("PaletteColumnCount", 4)
        }
    );

    ferro_property!(
        /// Defines the `Palette` property.
        pub fn palette_property() -> StyledProperty<Option<Rc<dyn IColorPalette>>> {
            FerroProperty::register::<ColorView, _>("Palette", None)
        }
    );

    ferro_property!(
        /// Defines the `SelectedIndex` property.
        pub fn selected_index_property() -> StyledProperty<i32> {
            FerroProperty::register::<ColorView, _>("SelectedIndex", ColorViewTab::Spectrum as i32)
        }
    );
} }

impl ColorView {
    /// See [`ColorSpectrum::color`](crate::primitives::ColorSpectrum::color).
    pub fn color(&self) -> Color {
        self.get_value(Self::color_property())
    }

    /// Sets [`color`](Self::color).
    pub fn set_color(&self, value: Color) {
        self.set_value(Self::color_property(), value)
    }

    /// See [`ColorSlider::color_model`](crate::primitives::ColorSlider::color_model).
    ///
    /// This property is only applicable to the components tab.
    /// The spectrum tab must always be in HSV and the palette tab contains only pre-defined colors.
    pub fn color_model(&self) -> ColorModel {
        self.get_value(Self::color_model_property())
    }

    /// Sets [`color_model`](Self::color_model).
    pub fn set_color_model(&self, value: ColorModel) {
        self.set_value(Self::color_model_property(), value)
    }

    /// See [`ColorSpectrum::components`](crate::primitives::ColorSpectrum::components).
    pub fn color_spectrum_components(&self) -> ColorSpectrumComponents {
        self.get_value(Self::color_spectrum_components_property())
    }

    /// Sets [`color_spectrum_components`](Self::color_spectrum_components).
    pub fn set_color_spectrum_components(&self, value: ColorSpectrumComponents) {
        self.set_value(Self::color_spectrum_components_property(), value)
    }

    /// See [`ColorSpectrum::shape`](crate::primitives::ColorSpectrum::shape).
    pub fn color_spectrum_shape(&self) -> ColorSpectrumShape {
        self.get_value(Self::color_spectrum_shape_property())
    }

    /// Sets [`color_spectrum_shape`](Self::color_spectrum_shape).
    pub fn set_color_spectrum_shape(&self, value: ColorSpectrumShape) {
        self.set_value(Self::color_spectrum_shape_property(), value)
    }

    /// Gets the position of the alpha component in the hexadecimal input box relative to
    /// all other color components.
    pub fn hex_input_alpha_position(&self) -> AlphaComponentPosition {
        self.get_value(Self::hex_input_alpha_position_property())
    }

    /// Sets [`hex_input_alpha_position`](Self::hex_input_alpha_position).
    pub fn set_hex_input_alpha_position(&self, value: AlphaComponentPosition) {
        self.set_value(Self::hex_input_alpha_position_property(), value)
    }

    /// See [`ColorSpectrum::hsv_color`](crate::primitives::ColorSpectrum::hsv_color).
    pub fn hsv_color(&self) -> HsvColor {
        self.get_value(Self::hsv_color_property())
    }

    /// Sets [`hsv_color`](Self::hsv_color).
    pub fn set_hsv_color(&self, value: HsvColor) {
        self.set_value(Self::hsv_color_property(), value)
    }

    /// See [`ColorPreviewer::is_accent_colors_visible`](crate::primitives::ColorPreviewer::is_accent_colors_visible).
    pub fn is_accent_colors_visible(&self) -> bool {
        self.get_value(Self::is_accent_colors_visible_property())
    }

    /// Sets [`is_accent_colors_visible`](Self::is_accent_colors_visible).
    pub fn set_is_accent_colors_visible(&self, value: bool) {
        self.set_value(Self::is_accent_colors_visible_property(), value)
    }

    /// Gets a value indicating whether the alpha component is enabled.
    /// When disabled (set to false) the alpha component will be fixed to maximum and
    /// editing controls disabled.
    pub fn is_alpha_enabled(&self) -> bool {
        self.get_value(Self::is_alpha_enabled_property())
    }

    /// Sets [`is_alpha_enabled`](Self::is_alpha_enabled).
    pub fn set_is_alpha_enabled(&self, value: bool) {
        self.set_value(Self::is_alpha_enabled_property(), value)
    }

    /// Gets a value indicating whether the alpha component editing controls
    /// (Slider(s) and TextBox) are visible. When hidden, the existing alpha component
    /// value is maintained.
    ///
    /// Note that [`is_component_text_input_visible`](Self::is_component_text_input_visible) also controls the alpha
    /// component TextBox visibility.
    pub fn is_alpha_visible(&self) -> bool {
        self.get_value(Self::is_alpha_visible_property())
    }

    /// Sets [`is_alpha_visible`](Self::is_alpha_visible).
    pub fn set_is_alpha_visible(&self, value: bool) {
        self.set_value(Self::is_alpha_visible_property(), value)
    }

    /// Gets a value indicating whether the color components tab/panel/page (subview) is visible.
    pub fn is_color_components_visible(&self) -> bool {
        self.get_value(Self::is_color_components_visible_property())
    }

    /// Sets [`is_color_components_visible`](Self::is_color_components_visible).
    pub fn set_is_color_components_visible(&self, value: bool) {
        self.set_value(Self::is_color_components_visible_property(), value)
    }

    /// Gets a value indicating whether the active color model indicator/selector is visible.
    pub fn is_color_model_visible(&self) -> bool {
        self.get_value(Self::is_color_model_visible_property())
    }

    /// Sets [`is_color_model_visible`](Self::is_color_model_visible).
    pub fn set_is_color_model_visible(&self, value: bool) {
        self.set_value(Self::is_color_model_visible_property(), value)
    }

    /// Gets a value indicating whether the color palette tab/panel/page (subview) is visible.
    pub fn is_color_palette_visible(&self) -> bool {
        self.get_value(Self::is_color_palette_visible_property())
    }

    /// Sets [`is_color_palette_visible`](Self::is_color_palette_visible).
    pub fn set_is_color_palette_visible(&self, value: bool) {
        self.set_value(Self::is_color_palette_visible_property(), value)
    }

    /// Gets a value indicating whether the color preview is visible.
    ///
    /// Note that accent color visibility is controlled separately by
    /// [`is_accent_colors_visible`](Self::is_accent_colors_visible).
    pub fn is_color_preview_visible(&self) -> bool {
        self.get_value(Self::is_color_preview_visible_property())
    }

    /// Sets [`is_color_preview_visible`](Self::is_color_preview_visible).
    pub fn set_is_color_preview_visible(&self, value: bool) {
        self.set_value(Self::is_color_preview_visible_property(), value)
    }

    /// Gets a value indicating whether the color spectrum tab/panel/page (subview) is visible.
    pub fn is_color_spectrum_visible(&self) -> bool {
        self.get_value(Self::is_color_spectrum_visible_property())
    }

    /// Sets [`is_color_spectrum_visible`](Self::is_color_spectrum_visible).
    pub fn set_is_color_spectrum_visible(&self, value: bool) {
        self.set_value(Self::is_color_spectrum_visible_property(), value)
    }

    /// Gets a value indicating whether the color spectrum's third component slider
    /// is visible.
    pub fn is_color_spectrum_slider_visible(&self) -> bool {
        self.get_value(Self::is_color_spectrum_slider_visible_property())
    }

    /// Sets [`is_color_spectrum_slider_visible`](Self::is_color_spectrum_slider_visible).
    pub fn set_is_color_spectrum_slider_visible(&self, value: bool) {
        self.set_value(Self::is_color_spectrum_slider_visible_property(), value)
    }

    /// Gets a value indicating whether color component sliders are visible.
    ///
    /// All color components are controlled by this property but alpha can also be
    /// controlled with [`is_alpha_visible`](Self::is_alpha_visible).
    pub fn is_component_slider_visible(&self) -> bool {
        self.get_value(Self::is_component_slider_visible_property())
    }

    /// Sets [`is_component_slider_visible`](Self::is_component_slider_visible).
    pub fn set_is_component_slider_visible(&self, value: bool) {
        self.set_value(Self::is_component_slider_visible_property(), value)
    }

    /// Gets a value indicating whether color component text inputs are visible.
    ///
    /// All color components are controlled by this property but alpha can also be
    /// controlled with [`is_alpha_visible`](Self::is_alpha_visible).
    pub fn is_component_text_input_visible(&self) -> bool {
        self.get_value(Self::is_component_text_input_visible_property())
    }

    /// Sets [`is_component_text_input_visible`](Self::is_component_text_input_visible).
    pub fn set_is_component_text_input_visible(&self, value: bool) {
        self.set_value(Self::is_component_text_input_visible_property(), value)
    }

    /// Gets a value indicating whether the hexadecimal color value text input
    /// is visible.
    pub fn is_hex_input_visible(&self) -> bool {
        self.get_value(Self::is_hex_input_visible_property())
    }

    /// Sets [`is_hex_input_visible`](Self::is_hex_input_visible).
    pub fn set_is_hex_input_visible(&self, value: bool) {
        self.set_value(Self::is_hex_input_visible_property(), value)
    }

    /// See [`ColorSpectrum::max_hue`](crate::primitives::ColorSpectrum::max_hue).
    pub fn max_hue(&self) -> i32 {
        self.get_value(Self::max_hue_property())
    }

    /// Sets [`max_hue`](Self::max_hue).
    pub fn set_max_hue(&self, value: i32) {
        self.set_value(Self::max_hue_property(), value)
    }

    /// See [`ColorSpectrum::max_saturation`](crate::primitives::ColorSpectrum::max_saturation).
    pub fn max_saturation(&self) -> i32 {
        self.get_value(Self::max_saturation_property())
    }

    /// Sets [`max_saturation`](Self::max_saturation).
    pub fn set_max_saturation(&self, value: i32) {
        self.set_value(Self::max_saturation_property(), value)
    }

    /// See [`ColorSpectrum::max_value`](crate::primitives::ColorSpectrum::max_value).
    pub fn max_value(&self) -> i32 {
        self.get_value(Self::max_value_property())
    }

    /// Sets [`max_value`](Self::max_value).
    pub fn set_max_value(&self, value: i32) {
        self.set_value(Self::max_value_property(), value)
    }

    /// See [`ColorSpectrum::min_hue`](crate::primitives::ColorSpectrum::min_hue).
    pub fn min_hue(&self) -> i32 {
        self.get_value(Self::min_hue_property())
    }

    /// Sets [`min_hue`](Self::min_hue).
    pub fn set_min_hue(&self, value: i32) {
        self.set_value(Self::min_hue_property(), value)
    }

    /// See [`ColorSpectrum::min_saturation`](crate::primitives::ColorSpectrum::min_saturation).
    pub fn min_saturation(&self) -> i32 {
        self.get_value(Self::min_saturation_property())
    }

    /// Sets [`min_saturation`](Self::min_saturation).
    pub fn set_min_saturation(&self, value: i32) {
        self.set_value(Self::min_saturation_property(), value)
    }

    /// See [`ColorSpectrum::min_value`](crate::primitives::ColorSpectrum::min_value).
    pub fn min_value(&self) -> i32 {
        self.get_value(Self::min_value_property())
    }

    /// Sets [`min_value`](Self::min_value).
    pub fn set_min_value(&self, value: i32) {
        self.set_value(Self::min_value_property(), value)
    }

    /// Gets the collection of individual colors in the palette.
    ///
    /// This is not commonly set manually. Instead, it should be set automatically by
    /// providing an [`IColorPalette`] to the [`palette`](Self::palette) property.
    ///
    /// Also note that this property is what should be bound in the control template.
    /// [`palette`](Self::palette) is too high-level to use on its own.
    pub fn palette_colors(&self) -> Option<FerroList<Color>> {
        self.get_value(Self::palette_colors_property())
    }

    /// Sets [`palette_colors`](Self::palette_colors).
    pub fn set_palette_colors(&self, value: Option<FerroList<Color>>) {
        self.set_value(Self::palette_colors_property(), value)
    }

    /// Gets the number of colors in each row (section) of the color palette.
    /// Within a standard palette, rows are shades and columns are colors.
    ///
    /// This is not commonly set manually. Instead, it should be set automatically by
    /// providing an [`IColorPalette`] to the [`palette`](Self::palette) property.
    ///
    /// Also note that this property is what should be bound in the control template.
    /// [`palette`](Self::palette) is too high-level to use on its own.
    pub fn palette_column_count(&self) -> i32 {
        self.get_value(Self::palette_column_count_property())
    }

    /// Sets [`palette_column_count`](Self::palette_column_count).
    pub fn set_palette_column_count(&self, value: i32) {
        self.set_value(Self::palette_column_count_property(), value)
    }

    /// Gets the color palette.
    ///
    /// This will automatically set both [`palette_colors`](Self::palette_colors) and
    /// [`palette_column_count`](Self::palette_column_count) overwriting any existing values.
    pub fn palette(&self) -> Option<Rc<dyn IColorPalette>> {
        self.get_value(Self::palette_property())
    }

    /// Sets [`palette`](Self::palette).
    pub fn set_palette(&self, value: Option<Rc<dyn IColorPalette>>) {
        self.set_value(Self::palette_property(), value)
    }

    /// Gets the index of the selected tab/panel/page (subview).
    ///
    /// When using the default control theme, this property is designed to be used with the
    /// [`ColorViewTab`] enum. The [`ColorViewTab`] enum defines the
    /// index values of each of the three standard tabs.
    /// Use like `set_selected_index(ColorViewTab::Palette as i32)`.
    pub fn selected_index(&self) -> i32 {
        self.get_value(Self::selected_index_property())
    }

    /// Sets [`selected_index`](Self::selected_index).
    pub fn set_selected_index(&self, value: i32) {
        self.set_value(Self::selected_index_property(), value)
    }
}
