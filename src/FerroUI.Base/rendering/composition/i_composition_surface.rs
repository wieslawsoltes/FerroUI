use super::{CompositionSurface, ICompositionObject};

/// A surface a composition surface brush shows (WinUI
/// `ICompositionSurface`): a [`CompositionSurface`] (a drawing surface), a
/// [`CompositionVisualSurface`](super::CompositionVisualSurface) or a
/// [`LoadedImageSurface`](super::LoadedImageSurface).
///
/// The server-side counterpart of every surface is an
/// [`IServerCompositionSurface`](super::server::IServerCompositionSurface).
pub trait ICompositionSurface: ICompositionObject {}

impl dyn ICompositionSurface {
    /// The value as [`Any`](std::any::Any), by native trait upcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl ICompositionSurface for CompositionSurface {}
