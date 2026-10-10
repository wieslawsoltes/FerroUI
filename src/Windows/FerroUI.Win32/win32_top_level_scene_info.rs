//! The platform-specific scene information of a top-level.

use ferroui_base::platform::PlatformThemeVariant;

/// An immutable bag with Win32-specific per-frame scene information,
/// published via the platform-specific scene info of a top-level and
/// consumed by render targets on the render thread.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Win32TopLevelSceneInfo {
    /// The theme variant of the frame of the window.
    pub theme_variant: PlatformThemeVariant,
}

impl Win32TopLevelSceneInfo {
    /// Creates the scene information for a theme variant.
    pub fn new(theme_variant: PlatformThemeVariant) -> Self {
        Self { theme_variant }
    }
}
