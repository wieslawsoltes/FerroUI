//! Port of `ColorPaletteResources.cs` and `ColorPaletteResources.Properties.cs`.

use crate::accents::SystemAccentColors;
use ferroui_base::controls::{ResourceKey, ResourceProvider, ResourceProviderImpl, ResourceValue};
use ferroui_base::media::Color;
use ferroui_base::metadata::into_markup_value;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, DirectProperty, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref,
};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// Represents a specialized resource dictionary that contains color
/// resources used by FluentTheme elements.
///
/// This class can only be used in the palettes of a
/// [`FluentTheme`](crate::FluentTheme).
#[repr(C)]
pub struct ColorPaletteResources {
    base: ResourceProvider,
    colors: RefCell<HashMap<String, Color>>,
    has_accent_color: Cell<bool>,
    accent_color: Cell<Color>,
    accent_color_dark1: Cell<Color>,
    accent_color_dark2: Cell<Color>,
    accent_color_dark3: Cell<Color>,
    accent_color_light1: Cell<Color>,
    accent_color_light2: Cell<Color>,
    accent_color_light3: Cell<Color>,
}

ferro_class!(ColorPaletteResources: ResourceProvider);
ferro_class_info!(ColorPaletteResources {
    new: ColorPaletteResources::new,
    markup: {
        properties: [
            AltHigh: Color { get: ColorPaletteResources::alt_high, set: ColorPaletteResources::set_alt_high },
            AltLow: Color { get: ColorPaletteResources::alt_low, set: ColorPaletteResources::set_alt_low },
            AltMedium: Color { get: ColorPaletteResources::alt_medium, set: ColorPaletteResources::set_alt_medium },
            AltMediumHigh: Color { get: ColorPaletteResources::alt_medium_high, set: ColorPaletteResources::set_alt_medium_high },
            AltMediumLow: Color { get: ColorPaletteResources::alt_medium_low, set: ColorPaletteResources::set_alt_medium_low },
            BaseHigh: Color { get: ColorPaletteResources::base_high, set: ColorPaletteResources::set_base_high },
            BaseLow: Color { get: ColorPaletteResources::base_low, set: ColorPaletteResources::set_base_low },
            BaseMedium: Color { get: ColorPaletteResources::base_medium, set: ColorPaletteResources::set_base_medium },
            BaseMediumHigh: Color { get: ColorPaletteResources::base_medium_high, set: ColorPaletteResources::set_base_medium_high },
            BaseMediumLow: Color { get: ColorPaletteResources::base_medium_low, set: ColorPaletteResources::set_base_medium_low },
            ChromeAltLow: Color { get: ColorPaletteResources::chrome_alt_low, set: ColorPaletteResources::set_chrome_alt_low },
            ChromeBlackHigh: Color { get: ColorPaletteResources::chrome_black_high, set: ColorPaletteResources::set_chrome_black_high },
            ChromeBlackLow: Color { get: ColorPaletteResources::chrome_black_low, set: ColorPaletteResources::set_chrome_black_low },
            ChromeBlackMedium: Color { get: ColorPaletteResources::chrome_black_medium, set: ColorPaletteResources::set_chrome_black_medium },
            ChromeBlackMediumLow: Color { get: ColorPaletteResources::chrome_black_medium_low, set: ColorPaletteResources::set_chrome_black_medium_low },
            ChromeDisabledHigh: Color { get: ColorPaletteResources::chrome_disabled_high, set: ColorPaletteResources::set_chrome_disabled_high },
            ChromeDisabledLow: Color { get: ColorPaletteResources::chrome_disabled_low, set: ColorPaletteResources::set_chrome_disabled_low },
            ChromeGray: Color { get: ColorPaletteResources::chrome_gray, set: ColorPaletteResources::set_chrome_gray },
            ChromeHigh: Color { get: ColorPaletteResources::chrome_high, set: ColorPaletteResources::set_chrome_high },
            ChromeLow: Color { get: ColorPaletteResources::chrome_low, set: ColorPaletteResources::set_chrome_low },
            ChromeMedium: Color { get: ColorPaletteResources::chrome_medium, set: ColorPaletteResources::set_chrome_medium },
            ChromeMediumLow: Color { get: ColorPaletteResources::chrome_medium_low, set: ColorPaletteResources::set_chrome_medium_low },
            ChromeWhite: Color { get: ColorPaletteResources::chrome_white, set: ColorPaletteResources::set_chrome_white },
            ErrorText: Color { get: ColorPaletteResources::error_text, set: ColorPaletteResources::set_error_text },
            ListLow: Color { get: ColorPaletteResources::list_low, set: ColorPaletteResources::set_list_low },
            ListMedium: Color { get: ColorPaletteResources::list_medium, set: ColorPaletteResources::set_list_medium },
            RegionColor: Color { get: ColorPaletteResources::region_color, set: ColorPaletteResources::set_region_color },
        ],
    },
});

impl FerroObjectImpl for ColorPaletteResources {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if change.property() == Self::accent_property().as_property() {
            let accent_color = this.accent_color.get();
            this.has_accent_color.set(accent_color != Color::default());

            if this.has_accent_color.get() {
                let (dark1, dark2, dark3, light1, light2, light3) =
                    SystemAccentColors::calculate_accent_shades(accent_color);
                this.accent_color_dark1.set(dark1);
                this.accent_color_dark2.set(dark2);
                this.accent_color_dark3.set(dark3);
                this.accent_color_light1.set(light1);
                this.accent_color_light2.set(light2);
                this.accent_color_light3.set(light3);
            }
            this.raise_resources_changed();
        }
    }
}

impl ResourceProviderImpl for ColorPaletteResources {
    fn has_resources(this: &Self) -> bool {
        this.has_accent_color.get() || !this.colors.borrow().is_empty()
    }

    fn try_get_resource(this: &Self, key: &ResourceKey, _theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        let key = key.as_str()?;
        let accent = |color: &Cell<Color>| this.has_accent_color.get().then(|| into_markup_value(color.get()));

        match key {
            SystemAccentColors::ACCENT_KEY => accent(&this.accent_color),
            SystemAccentColors::ACCENT_DARK1_KEY => accent(&this.accent_color_dark1),
            SystemAccentColors::ACCENT_DARK2_KEY => accent(&this.accent_color_dark2),
            SystemAccentColors::ACCENT_DARK3_KEY => accent(&this.accent_color_dark3),
            SystemAccentColors::ACCENT_LIGHT1_KEY => accent(&this.accent_color_light1),
            SystemAccentColors::ACCENT_LIGHT2_KEY => accent(&this.accent_color_light2),
            SystemAccentColors::ACCENT_LIGHT3_KEY => accent(&this.accent_color_light3),
            _ => this.colors.borrow().get(key).map(|color| into_markup_value(*color)),
        }
    }
}

ferro_properties! {
    impl ColorPaletteResources {
        /// Defines the `Accent` property.
        pub fn accent_property() -> DirectProperty<ColorPaletteResources, Color> {
            FerroProperty::register_direct::<ColorPaletteResources, _>(
                "Accent",
                |r| r.accent(),
                Some(|r, v| r.set_accent(v)),
                Color::default(),
            )
        }
    }
}

impl ColorPaletteResources {
    /// Field initialisation; see [`ResourceProvider::construct`].
    pub fn construct() -> Self {
        Self {
            base: ResourceProvider::construct(),
            colors: RefCell::new(HashMap::new()),
            has_accent_color: Cell::new(false),
            accent_color: Cell::new(Color::default()),
            accent_color_dark1: Cell::new(Color::default()),
            accent_color_dark2: Cell::new(Color::default()),
            accent_color_dark3: Cell::new(Color::default()),
            accent_color_light1: Cell::new(Color::default()),
            accent_color_light2: Cell::new(Color::default()),
            accent_color_light3: Cell::new(Color::default()),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn get_color(&self, key: &str) -> Color {
        self.colors.borrow().get(key).copied().unwrap_or_default()
    }

    fn set_color(&self, key: &str, value: Color) {
        if value == Color::default() {
            self.colors.borrow_mut().remove(key);
        } else {
            self.colors.borrow_mut().insert(key.to_string(), value);
        }
    }

    /// The Accent color value.
    pub fn accent(&self) -> Color {
        self.accent_color.get()
    }

    pub fn set_accent(&self, value: Color) {
        self.set_and_raise_cell(Self::accent_property(), &self.accent_color, value);
    }

    /// The AltHigh color value.
    pub fn alt_high(&self) -> Color {
        self.get_color("SystemAltHighColor")
    }

    pub fn set_alt_high(&self, value: Color) {
        self.set_color("SystemAltHighColor", value)
    }

    /// The AltLow color value.
    pub fn alt_low(&self) -> Color {
        self.get_color("SystemAltLowColor")
    }

    pub fn set_alt_low(&self, value: Color) {
        self.set_color("SystemAltLowColor", value)
    }

    /// The AltMedium color value.
    pub fn alt_medium(&self) -> Color {
        self.get_color("SystemAltMediumColor")
    }

    pub fn set_alt_medium(&self, value: Color) {
        self.set_color("SystemAltMediumColor", value)
    }

    /// The AltMediumHigh color value.
    pub fn alt_medium_high(&self) -> Color {
        self.get_color("SystemAltMediumHighColor")
    }

    pub fn set_alt_medium_high(&self, value: Color) {
        self.set_color("SystemAltMediumHighColor", value)
    }

    /// The AltMediumLow color value.
    pub fn alt_medium_low(&self) -> Color {
        self.get_color("SystemAltMediumLowColor")
    }

    pub fn set_alt_medium_low(&self, value: Color) {
        self.set_color("SystemAltMediumLowColor", value)
    }

    /// The BaseHigh color value.
    pub fn base_high(&self) -> Color {
        self.get_color("SystemBaseHighColor")
    }

    pub fn set_base_high(&self, value: Color) {
        self.set_color("SystemBaseHighColor", value)
    }

    /// The BaseLow color value.
    pub fn base_low(&self) -> Color {
        self.get_color("SystemBaseLowColor")
    }

    pub fn set_base_low(&self, value: Color) {
        self.set_color("SystemBaseLowColor", value)
    }

    /// The BaseMedium color value.
    pub fn base_medium(&self) -> Color {
        self.get_color("SystemBaseMediumColor")
    }

    pub fn set_base_medium(&self, value: Color) {
        self.set_color("SystemBaseMediumColor", value)
    }

    /// The BaseMediumHigh color value.
    pub fn base_medium_high(&self) -> Color {
        self.get_color("SystemBaseMediumHighColor")
    }

    pub fn set_base_medium_high(&self, value: Color) {
        self.set_color("SystemBaseMediumHighColor", value)
    }

    /// The BaseMediumLow color value.
    pub fn base_medium_low(&self) -> Color {
        self.get_color("SystemBaseMediumLowColor")
    }

    pub fn set_base_medium_low(&self, value: Color) {
        self.set_color("SystemBaseMediumLowColor", value)
    }

    /// The ChromeAltLow color value.
    pub fn chrome_alt_low(&self) -> Color {
        self.get_color("SystemChromeAltLowColor")
    }

    pub fn set_chrome_alt_low(&self, value: Color) {
        self.set_color("SystemChromeAltLowColor", value)
    }

    /// The ChromeBlackHigh color value.
    pub fn chrome_black_high(&self) -> Color {
        self.get_color("SystemChromeBlackHighColor")
    }

    pub fn set_chrome_black_high(&self, value: Color) {
        self.set_color("SystemChromeBlackHighColor", value)
    }

    /// The ChromeBlackLow color value.
    pub fn chrome_black_low(&self) -> Color {
        self.get_color("SystemChromeBlackLowColor")
    }

    pub fn set_chrome_black_low(&self, value: Color) {
        self.set_color("SystemChromeBlackLowColor", value)
    }

    /// The ChromeBlackMedium color value.
    pub fn chrome_black_medium(&self) -> Color {
        self.get_color("SystemChromeBlackMediumColor")
    }

    pub fn set_chrome_black_medium(&self, value: Color) {
        self.set_color("SystemChromeBlackMediumColor", value)
    }

    /// The ChromeBlackMediumLow color value.
    pub fn chrome_black_medium_low(&self) -> Color {
        self.get_color("SystemChromeBlackMediumLowColor")
    }

    pub fn set_chrome_black_medium_low(&self, value: Color) {
        self.set_color("SystemChromeBlackMediumLowColor", value)
    }

    /// The ChromeDisabledHigh color value.
    pub fn chrome_disabled_high(&self) -> Color {
        self.get_color("SystemChromeDisabledHighColor")
    }

    pub fn set_chrome_disabled_high(&self, value: Color) {
        self.set_color("SystemChromeDisabledHighColor", value)
    }

    /// The ChromeDisabledLow color value.
    pub fn chrome_disabled_low(&self) -> Color {
        self.get_color("SystemChromeDisabledLowColor")
    }

    pub fn set_chrome_disabled_low(&self, value: Color) {
        self.set_color("SystemChromeDisabledLowColor", value)
    }

    /// The ChromeGray color value.
    pub fn chrome_gray(&self) -> Color {
        self.get_color("SystemChromeGrayColor")
    }

    pub fn set_chrome_gray(&self, value: Color) {
        self.set_color("SystemChromeGrayColor", value)
    }

    /// The ChromeHigh color value.
    pub fn chrome_high(&self) -> Color {
        self.get_color("SystemChromeHighColor")
    }

    pub fn set_chrome_high(&self, value: Color) {
        self.set_color("SystemChromeHighColor", value)
    }

    /// The ChromeLow color value.
    pub fn chrome_low(&self) -> Color {
        self.get_color("SystemChromeLowColor")
    }

    pub fn set_chrome_low(&self, value: Color) {
        self.set_color("SystemChromeLowColor", value)
    }

    /// The ChromeMedium color value.
    pub fn chrome_medium(&self) -> Color {
        self.get_color("SystemChromeMediumColor")
    }

    pub fn set_chrome_medium(&self, value: Color) {
        self.set_color("SystemChromeMediumColor", value)
    }

    /// The ChromeMediumLow color value.
    pub fn chrome_medium_low(&self) -> Color {
        self.get_color("SystemChromeMediumLowColor")
    }

    pub fn set_chrome_medium_low(&self, value: Color) {
        self.set_color("SystemChromeMediumLowColor", value)
    }

    /// The ChromeWhite color value.
    pub fn chrome_white(&self) -> Color {
        self.get_color("SystemChromeWhiteColor")
    }

    pub fn set_chrome_white(&self, value: Color) {
        self.set_color("SystemChromeWhiteColor", value)
    }

    /// The ErrorText color value.
    pub fn error_text(&self) -> Color {
        self.get_color("SystemErrorTextColor")
    }

    pub fn set_error_text(&self, value: Color) {
        self.set_color("SystemErrorTextColor", value)
    }

    /// The ListLow color value.
    pub fn list_low(&self) -> Color {
        self.get_color("SystemListLowColor")
    }

    pub fn set_list_low(&self, value: Color) {
        self.set_color("SystemListLowColor", value)
    }

    /// The ListMedium color value.
    pub fn list_medium(&self) -> Color {
        self.get_color("SystemListMediumColor")
    }

    pub fn set_list_medium(&self, value: Color) {
        self.set_color("SystemListMediumColor", value)
    }

    /// The RegionColor color value.
    pub fn region_color(&self) -> Color {
        self.get_color("SystemRegionColor")
    }

    pub fn set_region_color(&self, value: Color) {
        self.set_color("SystemRegionColor", value)
    }
}
