//! Resolves host theme capabilities for scoped native preview surfaces.
use ferroui_base::media::{Color, IBrush, SolidColorBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::platform::PlatformThemeVariant;
use ferroui_base::styling::ThemeVariant;
use ferroui_controls::ThemeVariantScope;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreviewTheme {
    #[default]
    Light,
    Dark,
    Inherit,
    HighContrast,
}

impl PreviewTheme {
    /// Themes the native host can apply independently to a preview scope.
    pub fn available() -> Vec<Self> {
        let mut themes = vec![Self::Light, Self::Dark, Self::Inherit];
        if ThemeVariant::high_contrast_variant().is_some() {
            themes.push(Self::HighContrast);
        }
        themes
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::Inherit => "Application theme",
            Self::HighContrast => "High contrast",
        }
    }

    pub(super) fn resolve(self) -> Result<Option<ThemeVariant>, String> {
        Ok(match self {
            Self::Light => Some(ThemeVariant::light()),
            Self::Dark => Some(ThemeVariant::dark()),
            Self::Inherit => None,
            Self::HighContrast => {
                Some(ThemeVariant::high_contrast_variant().ok_or(
                    "Load a theme with a native High Contrast variant to preview this stage",
                )?)
            }
        })
    }

    pub(super) fn options(current: Self) -> Vec<Self> {
        let mut themes = Self::available();
        if !themes.contains(&current) {
            themes.push(current);
        }
        themes
    }
}

pub(super) fn preview_backing(scope: &ThemeVariantScope) -> Rc<dyn IBrush> {
    let variant = scope.actual_theme_variant();
    let high_contrast = variant.is_some() && variant == ThemeVariant::high_contrast_variant();
    if high_contrast {
        if let Some(color) = scope
            .try_find_resource(&"SystemColorWindowColor".into(), variant.as_ref())
            .and_then(|value| from_markup_value::<Color>(&value))
        {
            return SolidColorBrush::with_color(color).into();
        }
    }
    let dark = variant == Some(ThemeVariant::dark())
        || high_contrast
            && ThemeVariant::to_platform_theme_variant(variant.as_ref())
                == Some(PlatformThemeVariant::Dark);
    SolidColorBrush::parse(if dark { "#FF202020" } else { "#FFFFFFFF" })
        .unwrap()
        .into()
}
