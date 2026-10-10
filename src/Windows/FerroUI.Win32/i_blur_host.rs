//! The blur effects of a composition surface.

/// The effect a composition surface puts behind the content of a window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[allow(dead_code)] // The effects are chosen by the composition surfaces of stage 2c.
pub(crate) enum BlurEffect {
    #[default]
    None,
    GaussianBlur,
    Acrylic,
    MicaLight,
    MicaDark,
}

/// A surface of a window that can put a blur effect behind its content:
/// the surfaces of the composition modes.
pub(crate) trait ICompositionEffectsSurface: Send + Sync {
    /// Whether the system this runs on has the effect.
    fn is_blur_supported(&self, effect: BlurEffect) -> bool;
}
