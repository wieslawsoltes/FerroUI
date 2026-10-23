use super::IDrawingContextImpl;
use crate::media::imaging::BitmapBlendingMode;
use crate::media::{ColorMatrix, ColorMatrixAlphaMode};
use crate::Rect;

/// A drawing context implementation that can compose a layer onto what is
/// under it with a blend mode or through a colour matrix: what the effects
/// of WinUI's composition (`BlendEffect`, `ColorMatrixEffect`,
/// `SaturationEffect`, the composite modes) are drawn with.
///
/// Every push has its pop; the pushes nest with the other pushes of the
/// context (clips, layers, opacities, effects).
pub trait IDrawingContextImplWithLayerEffects: IDrawingContextImpl {
    /// Pushes a layer: what is drawn until the matching
    /// [`pop_blend_layer`](Self::pop_blend_layer) is composed onto what is
    /// under it with `mode` (the Porter-Duff modes compose, the others mix
    /// and compose source-over). `bounds` is the area drawn into, in the
    /// coordinates of the context.
    fn push_blend_layer(&mut self, bounds: Rect, mode: BitmapBlendingMode);

    /// Pops the latest blend layer.
    fn pop_blend_layer(&mut self);

    /// Pushes a layer: what is drawn until the matching
    /// [`pop_color_matrix_layer`](Self::pop_color_matrix_layer) is
    /// transformed by `matrix` (applied to premultiplied or straight colours,
    /// as `alpha_mode` says) and composed source-over onto what is under it.
    /// `bounds` is the area drawn into, in the coordinates of the context;
    /// nothing outside it is kept.
    fn push_color_matrix_layer(&mut self, bounds: Rect, matrix: &ColorMatrix, alpha_mode: ColorMatrixAlphaMode);

    /// Pops the latest colour matrix layer.
    fn pop_color_matrix_layer(&mut self);
}
