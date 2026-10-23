//! The media of the WinUI object model that need controls and windows
//! (winui-materials.md): the Reveal brushes and lights, and the system
//! backdrops (`Microsoft.UI.Xaml.Media`).

mod reveal_brush;
mod reveal_lights;
mod system_backdrop;

pub use reveal_brush::{
    connected_reveal_brushes, reveal_border_effect, reveal_hover_effect, ApplicationTheme, RevealBackgroundBrush,
    RevealBorderBrush, RevealBrush, RevealBrushState, COLOR_TO_ALPHA_MATRIX, LUMINANCE_TO_ALPHA_MATRIX,
    REVEAL_BORDER_COLOR_MATRIX, REVEAL_INVERTED_BORDER_COLOR_MATRIX,
};
pub(crate) use reveal_brush::on_top_level_created;
pub use system_backdrop::{
    DesktopAcrylicBackdrop, MicaBackdrop, SystemBackdrop, SystemBackdropImpl, SystemBackdropImplExt, SystemBackdropVTable,
    ElementBackdropTarget, TopLevelBackdropTarget,
};
pub(crate) use system_backdrop::ElementSystemBackdrop;
pub use reveal_lights::{
    RevealBorderLight, RevealHoverLight, RevealPressLight, XamlAmbientLight, BORDER_LIGHT_SWITCH_DURATION,
    REVEAL_BORDER_LIGHT_DARK_THEME_ID, REVEAL_BORDER_LIGHT_LIGHT_THEME_ID, REVEAL_HOVER_LIGHT_ID, XAML_AMBIENT_LIGHT_ID,
};

/// The number of enums of this module that declare their markup metadata
/// next to the type (registered with the value types of the crate).
pub(crate) const MARKUP_ENUM_COUNT: usize = 2;

#[cfg(test)]
mod media_tests;
