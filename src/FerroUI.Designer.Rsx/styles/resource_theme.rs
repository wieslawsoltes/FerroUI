//! Rust spellings for theme variants that the isolated native preview can resolve.
use super::resources::literal_key;
use crate::animation::syntax as rust;
use ferroui_designer_tools::component_palette::PaletteLayer;
use ferroui_designer_tools::theme_variants::HIGH_CONTRAST_KEY;
use syn::Expr;

const THEME: &str = "::ferroui_base::styling::ThemeVariant";

pub(super) fn read(expression: &Expr) -> Option<PaletteLayer> {
    let Expr::Call(call) = expression else {
        return None;
    };
    let path = rust::path(&call.func)?;
    match (path.as_str(), call.args.len()) {
        ("ferroui_base::styling::ThemeVariant::default", 0) => Some(PaletteLayer::Default),
        ("ferroui_base::styling::ThemeVariant::light", 0) => Some(PaletteLayer::Light),
        ("ferroui_base::styling::ThemeVariant::dark", 0) => Some(PaletteLayer::Dark),
        (
            "ferroui_themes_winui::WinUITheme::high_contrast"
            | "ferroui_themes_material::MaterialTheme::high_contrast"
            | "ferroui_themes_macos::MacOSTheme::high_contrast",
            0,
        ) => Some(PaletteLayer::HighContrast),
        ("ferroui_base::styling::ThemeVariant::new", 2) => {
            let Expr::Call(inherits) = &call.args[1] else {
                return None;
            };
            let some = rust::path(&inherits.func)?;
            (literal_key(&call.args[0]).as_deref() == Some(HIGH_CONTRAST_KEY)
                && matches!(
                    some.as_str(),
                    "Some" | "core::option::Option::Some" | "std::option::Option::Some"
                )
                && inherits.args.len() == 1
                && read(&inherits.args[0]) == Some(PaletteLayer::Dark))
            .then_some(PaletteLayer::HighContrast)
        }
        _ => None,
    }
}

pub(super) fn constructor(layer: PaletteLayer) -> Option<String> {
    Some(match layer {
        PaletteLayer::Shared => return None,
        PaletteLayer::Default => format!("{THEME}::default()"),
        PaletteLayer::Light => format!("{THEME}::light()"),
        PaletteLayer::Dark => format!("{THEME}::dark()"),
        PaletteLayer::HighContrast => format!(
            "{THEME}::new({HIGH_CONTRAST_KEY:?}, ::core::option::Option::Some({THEME}::dark()))"
        ),
    })
}

pub(super) fn fallback_order(layer: PaletteLayer) -> &'static [PaletteLayer] {
    use PaletteLayer::{Dark, Default, HighContrast, Light, Shared};
    match layer {
        Shared => &[Shared],
        Default => &[Shared, Default],
        Light => &[Shared, Light, Default],
        Dark => &[Shared, Dark, Default],
        HighContrast => &[Shared, HighContrast, Dark, Default],
    }
}
