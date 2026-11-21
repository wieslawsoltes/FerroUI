//! Authored identities for the preview's known native theme variants.
use crate::components::FRAMEWORK;
use crate::ComponentResourceContext;
use serde::{Deserialize, Serialize};

pub const HIGH_CONTRAST_KEY: &str = "HighContrast";

/// The framework themes that export a static HighContrast markup member.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HighContrastProvider {
    WinUi,
    Material,
    MacOs,
}

impl HighContrastProvider {
    pub const ALL: [Self; 3] = [Self::WinUi, Self::Material, Self::MacOs];

    pub fn label(self) -> &'static str {
        match self {
            Self::WinUi => "WinUI",
            Self::Material => "Material",
            Self::MacOs => "macOS",
        }
    }

    pub fn theme_class(self) -> &'static str {
        match self {
            Self::WinUi => "WinUITheme",
            Self::Material => "MaterialTheme",
            Self::MacOs => "MacOSTheme",
        }
    }

    pub fn assembly(self) -> &'static str {
        match self {
            Self::WinUi => "FerroUI.Themes.WinUI",
            Self::Material => "FerroUI.Themes.Material",
            Self::MacOs => "FerroUI.Themes.MacOS",
        }
    }

    pub(crate) fn matches_namespace(self, namespace: &str) -> bool {
        namespace == FRAMEWORK
            || namespace.strip_prefix("using:") == Some(self.assembly())
            || namespace
                .strip_prefix("clr-namespace:")
                .and_then(|value| value.split_once(";assembly="))
                == Some((self.assembly(), self.assembly()))
    }
}

pub fn known_theme_key(value: &str) -> bool {
    matches!(value, "Default" | "Light" | "Dark" | HIGH_CONTRAST_KEY)
}

impl ComponentResourceContext {
    /// Resolve an authored native variant using the framework theme packages' inheritance.
    pub fn for_authored_theme(theme: &str) -> Self {
        let inherited_themes = if theme == HIGH_CONTRAST_KEY {
            vec!["Dark".into()]
        } else {
            Vec::new()
        };
        Self {
            theme: Some(theme.into()),
            inherited_themes,
        }
    }
}
