use super::IDrawingContextImpl;
use crate::rendering::scene3d::Scene3DFrame;

/// A drawing context that draws 3D scenes (#608, `docs/porting/media3d.md`, section 3.7): the
/// optional capability through which the render-data operation `DrawScene3D` of a `Viewport3D`
/// reaches a backend.
///
/// The backend renders the scene of the frame into a target of its own (with a private depth
/// buffer, multisampled where it can) and draws the result over the bounds of the frame like an
/// image, under the transform, clip, opacity and layers of the context. It keeps what it made
/// for a scene (pipelines, mesh buffers by `(id, version)`, the output) and draws the cached
/// output again while the generations of the snapshot, the size of the target and its input
/// textures are unchanged.
///
/// A context reaches this through
/// [`IDrawingContextImpl::as_drawing_context_impl_with_scene_3d`]; one that does not implement it
/// draws nothing for a scene, which is counted
/// ([`scenes_not_drawn`](crate::rendering::scene3d::scenes_not_drawn)) and reported once.
pub trait IDrawingContextImplWithScene3D: IDrawingContextImpl {
    /// Draws the scene of `frame` over its bounds. Returns `false` without drawing anything
    /// when the context cannot draw it (no executor for the device, a failed pipeline): the
    /// caller then counts the scene as not drawn.
    fn draw_scene_3d(&mut self, frame: &Scene3DFrame) -> bool;
}
