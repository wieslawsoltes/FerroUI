//! Material theme configuration used by the gallery and capture matrix.

use ferroui_base::media::Color;
use ferroui_base::styling::{IStyle, ThemeVariant};
use ferroui_base::Ref;
use std::rc::Rc;

use ferroui_themes_material::MaterialColorScheme;
use ferroui_themes_material::{DensityStyle, MaterialTheme as Theme};

/// The theme of the gallery.
pub type GalleryTheme = Theme;

/// Creates the theme.
pub fn create() -> Ref<GalleryTheme> {
    Theme::new()
}

/// The theme as a style of the application.
pub fn as_style(theme: &GalleryTheme) -> Rc<dyn IStyle> {
    theme.as_style()
}

/// The density of the controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    #[default]
    Normal,
    Compact,
}

impl Density {
    /// Every density, in the order of the toolbar and of the capture mode.
    pub const ALL: [Density; 2] = [Density::Normal, Density::Compact];

    /// The name of the density in the toolbar and in the options of the capture mode.
    pub fn name(self) -> &'static str {
        match self {
            Density::Normal => "normal",
            Density::Compact => "compact",
        }
    }

    /// The density with the name `name` (without regard to case).
    pub fn parse(name: &str) -> Option<Density> {
        Density::ALL
            .into_iter()
            .find(|density| density.name().eq_ignore_ascii_case(name.trim()))
    }
}

/// Sets the density style of the theme.
pub fn set_density(theme: &GalleryTheme, density: Density) {
    theme.set_density_style(match density {
        Density::Normal => DensityStyle::Normal,
        Density::Compact => DensityStyle::Compact,
    });
}

/// Rebuilds complete HCT color schemes for a seed; None restores official baseline colors.
pub fn set_accent(theme: &GalleryTheme, accent: Option<Color>) {
    let light = accent.map_or_else(MaterialColorScheme::expressive_light, |seed| {
        MaterialColorScheme::from_seed(seed, false)
    });
    let dark = accent.map_or_else(MaterialColorScheme::dark, |seed| {
        MaterialColorScheme::from_seed(seed, true)
    });
    theme.set_color_scheme(ThemeVariant::light(), &light);
    theme.set_color_scheme(ThemeVariant::dark(), &dark);
    theme.set_color_scheme(ThemeVariant::default(), &dark);
    let contrast = MaterialColorScheme::from_seed_color(
        accent.unwrap_or(Color::from_rgb(0x67, 0x50, 0xa4)),
        true,
        1.0,
    )
    .expect("finite contrast");
    theme.set_color_scheme(Theme::high_contrast(), &contrast);
}

/// The theme variants the gallery offers. `HighContrast` is a variant of its own key, which
/// selects the `HighContrast` theme dictionaries of the theme where the theme has them; where it
/// has none, the resources come from its default dictionaries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    Light,
    Dark,
    HighContrast,
}

impl Variant {
    /// Every variant, in the order of the toolbar.
    pub const ALL: [Variant; 3] = [Variant::Light, Variant::Dark, Variant::HighContrast];

    /// The name of the variant in the toolbar, in the options of the capture mode and in the
    /// names of the pictures (in lower case there).
    pub fn name(self) -> &'static str {
        match self {
            Variant::Light => "Light",
            Variant::Dark => "Dark",
            Variant::HighContrast => "HighContrast",
        }
    }

    /// The variant with the name `name` (without regard to case).
    pub fn parse(name: &str) -> Option<Variant> {
        Variant::ALL
            .into_iter()
            .find(|variant| variant.name().eq_ignore_ascii_case(name.trim()))
    }

    /// The theme variant of the framework.
    pub fn theme_variant(self) -> ThemeVariant {
        match self {
            Variant::Light => ThemeVariant::light(),
            Variant::Dark => ThemeVariant::dark(),
            Variant::HighContrast => Theme::high_contrast(),
        }
    }
}
