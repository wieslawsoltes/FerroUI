//! The media of the WinUI object model that need controls and windows
//! (winui-materials.md): the Reveal brushes and lights, and the system
//! backdrops (`Microsoft.UI.Xaml.Media`).

mod reveal_brush;
mod reveal_focus;
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
pub use reveal_focus::{
    direction_from, glow_inset, is_traveling_focus_enabled, spot_light_duration, spot_light_path, FocusVisualKind,
    RevealFocus, RevealFocusDirection, REVEAL_FOCUS_GLOW_PNG,
};
pub(crate) use reveal_focus::{on_control_got_focus, on_control_lost_focus, track_focus};
pub use reveal_lights::{
    RevealBorderLight, RevealHoverLight, RevealPressLight, XamlAmbientLight, BORDER_LIGHT_SWITCH_DURATION,
    REVEAL_BORDER_LIGHT_DARK_THEME_ID, REVEAL_BORDER_LIGHT_LIGHT_THEME_ID, REVEAL_HOVER_LIGHT_ID, XAML_AMBIENT_LIGHT_ID,
};

#[cfg(test)]
mod media_tests;
