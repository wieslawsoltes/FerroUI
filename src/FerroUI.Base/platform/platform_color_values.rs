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

/// The system colours of a contrast theme: the colours WinUI reads with `GetSysColor` (its
/// `SystemColor*Color` resources) while high contrast is on.
///
/// A FerroUI addition beside the accent colours of [`PlatformColorValues`]: a platform that
/// reports them sets them with [`PlatformColorValues::with_system_colors`] (Windows: the colours
/// of the active contrast theme; macOS: the system colours under "Increase contrast"). It is a
/// value with private fields: build one from [`SystemColors::new`] or one of the schemes of
/// Windows ([`high_contrast_black`](Self::high_contrast_black),
/// [`high_contrast_white`](Self::high_contrast_white)) with the `with_*` methods, so that more
/// colours can be added later.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SystemColors {
    window: Color,
    window_text: Color,
    button_face: Color,
    button_text: Color,
    highlight: Color,
    highlight_text: Color,
    hotlight: Color,
    gray_text: Color,
}

impl SystemColors {
    /// A scheme of two colours: `window` is the background of windows, buttons and highlighted
    /// text; `window_text` the text, the highlight, the hyperlinks and the disabled text.
    pub const fn new(window: Color, window_text: Color) -> Self {
        Self {
            window,
            window_text,
            button_face: window,
            button_text: window_text,
            highlight: window_text,
            highlight_text: window,
            hotlight: window_text,
            gray_text: window_text,
        }
    }

    /// The colours of the "High Contrast Black" contrast theme of Windows.
    pub const fn high_contrast_black() -> Self {
        Self {
            window: Color::from_rgb(0x00, 0x00, 0x00),
            window_text: Color::from_rgb(0xFF, 0xFF, 0xFF),
            button_face: Color::from_rgb(0x00, 0x00, 0x00),
            button_text: Color::from_rgb(0xFF, 0xFF, 0xFF),
            highlight: Color::from_rgb(0x1A, 0xEB, 0xFF),
            highlight_text: Color::from_rgb(0x00, 0x00, 0x00),
            hotlight: Color::from_rgb(0xFF, 0xFF, 0x00),
            gray_text: Color::from_rgb(0x3F, 0xF2, 0x3F),
        }
    }

    /// The colours of the "High Contrast White" contrast theme of Windows.
    pub const fn high_contrast_white() -> Self {
        Self {
            window: Color::from_rgb(0xFF, 0xFF, 0xFF),
            window_text: Color::from_rgb(0x00, 0x00, 0x00),
            button_face: Color::from_rgb(0xFF, 0xFF, 0xFF),
            button_text: Color::from_rgb(0x00, 0x00, 0x00),
            highlight: Color::from_rgb(0x37, 0x00, 0x6E),
            highlight_text: Color::from_rgb(0xFF, 0xFF, 0xFF),
            hotlight: Color::from_rgb(0x00, 0x00, 0x9F),
            gray_text: Color::from_rgb(0x60, 0x00, 0x00),
        }
    }

    /// The background of windows (`COLOR_WINDOW`, `SystemColorWindowColor`).
    pub const fn window(&self) -> Color {
        self.window
    }

    /// Returns the colours with the given window background.
    pub const fn with_window(mut self, value: Color) -> Self {
        self.window = value;
        self
    }

    /// The text of windows (`COLOR_WINDOWTEXT`, `SystemColorWindowTextColor`).
    pub const fn window_text(&self) -> Color {
        self.window_text
    }

    /// Returns the colours with the given window text.
    pub const fn with_window_text(mut self, value: Color) -> Self {
        self.window_text = value;
        self
    }

    /// The face of buttons (`COLOR_BTNFACE`, `SystemColorButtonFaceColor`).
    pub const fn button_face(&self) -> Color {
        self.button_face
    }

    /// Returns the colours with the given button face.
    pub const fn with_button_face(mut self, value: Color) -> Self {
        self.button_face = value;
        self
    }

    /// The text of buttons (`COLOR_BTNTEXT`, `SystemColorButtonTextColor`).
    pub const fn button_text(&self) -> Color {
        self.button_text
    }

    /// Returns the colours with the given button text.
    pub const fn with_button_text(mut self, value: Color) -> Self {
        self.button_text = value;
        self
    }

    /// The background of selected items (`COLOR_HIGHLIGHT`, `SystemColorHighlightColor`).
    pub const fn highlight(&self) -> Color {
        self.highlight
    }

    /// Returns the colours with the given highlight.
    pub const fn with_highlight(mut self, value: Color) -> Self {
        self.highlight = value;
        self
    }

    /// The text of selected items (`COLOR_HIGHLIGHTTEXT`, `SystemColorHighlightTextColor`).
    pub const fn highlight_text(&self) -> Color {
        self.highlight_text
    }

    /// Returns the colours with the given highlight text.
    pub const fn with_highlight_text(mut self, value: Color) -> Self {
        self.highlight_text = value;
        self
    }

    /// The colour of hyperlinks (`COLOR_HOTLIGHT`, `SystemColorHotlightColor`).
    pub const fn hotlight(&self) -> Color {
        self.hotlight
    }

    /// Returns the colours with the given hyperlink colour.
    pub const fn with_hotlight(mut self, value: Color) -> Self {
        self.hotlight = value;
        self
    }

    /// The text of disabled items (`COLOR_GRAYTEXT`, `SystemColorGrayTextColor`).
    pub const fn gray_text(&self) -> Color {
        self.gray_text
    }

    /// Returns the colours with the given disabled text.
    pub const fn with_gray_text(mut self, value: Color) -> Self {
        self.gray_text = value;
        self
    }

    /// Whether the scheme is a light or a dark one: the perceived brightness of the window
    /// background (the frame of a window in the high contrast variant follows it).
    pub fn theme_variant(&self) -> PlatformThemeVariant {
        let window = self.window;
        let brightness = 299 * u32::from(window.r) + 587 * u32::from(window.g) + 114 * u32::from(window.b);
        if brightness < 128 * 1000 {
            PlatformThemeVariant::Dark
        } else {
            PlatformThemeVariant::Light
        }
    }
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
    system_colors: Option<SystemColors>,
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
            system_colors: None,
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

    /// The system colours of the active contrast theme, if the platform reports them (Windows
    /// while high contrast is on, macOS under "Increase contrast"); `None` otherwise, and by
    /// default. Values compare with their system colours, so an unchanged scheme is no change
    /// of the colour values. A FerroUI addition.
    pub const fn system_colors(&self) -> Option<SystemColors> {
        self.system_colors
    }

    /// Returns the values with the given system colours, or without them for `None`.
    pub const fn with_system_colors(mut self, value: Option<SystemColors>) -> Self {
        self.system_colors = value;
        self
    }

    /// Whether the system asks for high contrast: the contrast preference is
    /// [`High`](ColorContrastPreference::High).
    pub fn is_high_contrast(&self) -> bool {
        self.contrast_preference == ColorContrastPreference::High
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
        assert!(dark.is_high_contrast());
        assert!(!PlatformColorValues::new().is_high_contrast());
    }

    #[test]
    fn system_colors_are_reported_only_when_given_and_compare_by_content() {
        let values = PlatformColorValues::new().with_contrast_preference(ColorContrastPreference::High);
        assert_eq!(values.system_colors(), None);

        let black = SystemColors::high_contrast_black();
        let with_colors = values.with_system_colors(Some(black));
        assert_eq!(with_colors.system_colors(), Some(black));
        assert_ne!(with_colors, values);
        // The same scheme is the same value: an unchanged scheme raises no change.
        assert_eq!(with_colors, values.with_system_colors(Some(SystemColors::high_contrast_black())));
        assert_ne!(with_colors, values.with_system_colors(Some(SystemColors::high_contrast_white())));
        assert_eq!(with_colors.with_system_colors(None), values);
    }

    #[test]
    fn system_colors_are_built_from_two_colours_and_changed_one_by_one() {
        let window = Color::from_rgb(0x20, 0x20, 0x20);
        let text = Color::from_rgb(0xFF, 0xFA, 0xEF);
        let colors = SystemColors::new(window, text);
        assert_eq!((colors.window(), colors.button_face(), colors.highlight_text()), (window, window, window));
        assert_eq!((colors.window_text(), colors.button_text(), colors.highlight()), (text, text, text));
        assert_eq!((colors.hotlight(), colors.gray_text()), (text, text));

        let link = Color::from_rgb(0x75, 0xE9, 0xFC);
        let colors = colors
            .with_hotlight(link)
            .with_gray_text(Color::from_rgb(0xA6, 0xA6, 0xA6))
            .with_highlight(Color::from_rgb(0x8E, 0xE3, 0xF0))
            .with_highlight_text(Color::from_rgb(0x26, 0x30, 0x38))
            .with_button_face(Color::from_rgb(0x20, 0x20, 0x20))
            .with_button_text(Color::from_rgb(0xFF, 0xEE, 0x32))
            .with_window(Color::from_rgb(0x2D, 0x32, 0x36))
            .with_window_text(Color::from_rgb(0xFF, 0xFF, 0xFF));
        assert_eq!(colors.hotlight(), link);
        assert_eq!(colors.button_text(), Color::from_rgb(0xFF, 0xEE, 0x32));
        assert_eq!(colors.window(), Color::from_rgb(0x2D, 0x32, 0x36));
        assert_eq!(colors.highlight_text(), Color::from_rgb(0x26, 0x30, 0x38));
    }

    #[test]
    fn a_scheme_is_light_or_dark_by_its_window_background() {
        assert_eq!(SystemColors::high_contrast_black().theme_variant(), PlatformThemeVariant::Dark);
        assert_eq!(SystemColors::high_contrast_white().theme_variant(), PlatformThemeVariant::Light);
        let desert = SystemColors::new(Color::from_rgb(0xFF, 0xFA, 0xEF), Color::from_rgb(0x3D, 0x3D, 0x3D));
        assert_eq!(desert.theme_variant(), PlatformThemeVariant::Light);
        let dusk = SystemColors::new(Color::from_rgb(0x2D, 0x32, 0x36), Color::from_rgb(0xFF, 0xFF, 0xFF));
        assert_eq!(dusk.theme_variant(), PlatformThemeVariant::Dark);
    }
}
