use super::{CompositionSurface, ICompositionObject};

/// A surface a composition surface brush shows (WinUI
/// `ICompositionSurface`): a [`CompositionSurface`] (a drawing surface), a
/// [`CompositionVisualSurface`](super::CompositionVisualSurface) or a
/// [`LoadedImageSurface`](super::LoadedImageSurface).
///
/// The server-side counterpart of every surface is an
/// [`IServerCompositionSurface`](super::server::IServerCompositionSurface).
pub trait ICompositionSurface: ICompositionObject {}

impl ICompositionSurface for CompositionSurface {}
