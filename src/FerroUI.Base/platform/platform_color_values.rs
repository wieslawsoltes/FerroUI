use crate::media::Color;

/// System theme variant or mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PlatformThemeVariant {
    #[default]
    Light = 0,
    Dark = 1,
}

/// System high contrast preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ColorContrastPreference {
    #[default]
    NoPreference = 0,
    High = 1,
}

/// The six shades of the accent color that a system computes from it: three
/// darker and three lighter ones, as Windows reports them
/// (`UIColorType::AccentDark3` to `AccentLight3` of `UISettings`).
///
/// A FerroUI addition: platforms that compute no shades report none, and a
/// theme computes its own from the accent color.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AccentColorShades {
    /// The first darker shade.
    pub dark1: Color,
    /// The second darker shade.
    pub dark2: Color,
    /// The darkest shade.
    pub dark3: Color,
    /// The first lighter shade.
    pub light1: Color,
    /// The second lighter shade.
    pub light2: Color,
    /// The lightest shade.
    pub light3: Color,
}

/// Information about current system color values, including information
/// about dark mode and accent colors.
///
/// It is a value: build one from [`PlatformColorValues::new`] with the
/// `with_*` methods.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlatformColorValues {
    theme_variant: PlatformThemeVariant,
    contrast_preference: ColorContrastPreference,
    accent_color1: Color,
    accent_color2: Color,
    accent_color3: Color,
    accent_shades: Option<AccentColorShades>,
}

impl PlatformColorValues {
    const DEFAULT_ACCENT: Color = Color::new(255, 0, 120, 215);
    const UNSET: Color = Color::new(0, 0, 0, 0);

    /// Creates the default color values: the light theme, no contrast
    /// preference and the default accent color.
    pub const fn new() -> Self {
        Self {
            theme_variant: PlatformThemeVariant::Light,
            contrast_preference: ColorContrastPreference::NoPreference,
            accent_color1: Self::DEFAULT_ACCENT,
            accent_color2: Self::UNSET,
            accent_color3: Self::UNSET,
            accent_shades: None,
        }
    }

    /// System theme variant or mode.
    pub const fn theme_variant(&self) -> PlatformThemeVariant {
        self.theme_variant
    }

    /// Returns the values with the given theme variant.
    pub const fn with_theme_variant(mut self, value: PlatformThemeVariant) -> Self {
        self.theme_variant = value;
        self
    }

    /// System high contrast preference.
    pub const fn contrast_preference(&self) -> ColorContrastPreference {
        self.contrast_preference
    }

    /// Returns the values with the given contrast preference.
    pub const fn with_contrast_preference(mut self, value: ColorContrastPreference) -> Self {
        self.contrast_preference = value;
        self
    }

    /// Primary system accent color.
    pub const fn accent_color1(&self) -> Color {
        self.accent_color1
    }

    /// Returns the values with the given primary accent color.
    pub const fn with_accent_color1(mut self, value: Color) -> Self {
        self.accent_color1 = value;
        self
    }

    /// Secondary system accent color. On some platforms can return the
    /// same value as [`accent_color1`](Self::accent_color1).
    pub fn accent_color2(&self) -> Color {
        if self.accent_color2 != Self::UNSET {
            self.accent_color2
        } else {
            self.accent_color1
        }
    }

    /// Returns the values with the given secondary accent color.
    pub const fn with_accent_color2(mut self, value: Color) -> Self {
        self.accent_color2 = value;
        self
    }

    /// Tertiary system accent color. On some platforms can return the same
    /// value as [`accent_color1`](Self::accent_color1).
    pub fn accent_color3(&self) -> Color {
        if self.accent_color3 != Self::UNSET {
            self.accent_color3
        } else {
            self.accent_color1
        }
    }

    /// Returns the values with the given tertiary accent color.
    pub const fn with_accent_color3(mut self, value: Color) -> Self {
        self.accent_color3 = value;
        self
    }

    /// The shades of the primary accent color as the system computes them,
    /// if the platform reports them (Windows does); `None` otherwise, and by
    /// default. A FerroUI addition.
    pub const fn accent_shades(&self) -> Option<AccentColorShades> {
        self.accent_shades
    }

    /// Returns the values with the given shades of the primary accent color,
    /// or without shades for `None`.
    pub const fn with_accent_shades(mut self, value: Option<AccentColorShades>) -> Self {
        self.accent_shades = value;
        self
    }
}

impl Default for PlatformColorValues {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_light_with_the_default_accent() {
        let values = PlatformColorValues::new();
        assert_eq!(values.theme_variant(), PlatformThemeVariant::Light);
        assert_eq!(values.contrast_preference(), ColorContrastPreference::NoPreference);
        assert_eq!(values.accent_color1(), Color::new(255, 0, 120, 215));
        assert_eq!(values, PlatformColorValues::default());
    }

    #[test]
    fn secondary_accents_fall_back_to_the_primary_one() {
        let primary = Color::new(255, 1, 2, 3);
        let values = PlatformColorValues::new().with_accent_color1(primary);
        assert_eq!(values.accent_color2(), primary);
        assert_eq!(values.accent_color3(), primary);

        let secondary = Color::new(255, 4, 5, 6);
        let tertiary = Color::new(255, 7, 8, 9);
        let values = values.with_accent_color2(secondary).with_accent_color3(tertiary);
        assert_eq!(values.accent_color1(), primary);
        assert_eq!(values.accent_color2(), secondary);
        assert_eq!(values.accent_color3(), tertiary);
    }

    #[test]
    fn accent_shades_are_reported_only_when_given() {
        let values = PlatformColorValues::new();
        assert_eq!(values.accent_shades(), None);

        let shades = AccentColorShades {
            dark1: Color::new(255, 0, 95, 184),
            dark2: Color::new(255, 0, 72, 145),
            dark3: Color::new(255, 0, 36, 82),
            light1: Color::new(255, 0, 145, 248),
            light2: Color::new(255, 76, 194, 255),
            light3: Color::new(255, 153, 235, 255),
        };
        let with_shades = values.with_accent_shades(Some(shades));
        assert_eq!(with_shades.accent_shades(), Some(shades));
        assert_ne!(with_shades, values);
        // The shades do not change the accent colors.
        assert_eq!(with_shades.accent_color1(), values.accent_color1());
        assert_eq!(with_shades.with_accent_shades(None), values);
    }

    #[test]
    fn values_compare_by_content() {
        let dark = PlatformColorValues::new()
            .with_theme_variant(PlatformThemeVariant::Dark)
            .with_contrast_preference(ColorContrastPreference::High);
        assert_eq!(dark.theme_variant(), PlatformThemeVariant::Dark);
        assert_eq!(dark.contrast_preference(), ColorContrastPreference::High);
        assert_ne!(dark, PlatformColorValues::new());
        assert_eq!(dark, PlatformColorValues::new().with_theme_variant(PlatformThemeVariant::Dark).with_contrast_preference(ColorContrastPreference::High));
    }
}
