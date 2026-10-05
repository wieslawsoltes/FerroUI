use super::composition::Compositor;
use super::{RendererDiagnostics, SceneInvalidatedEventArgs};
use crate::reactive::IDisposable;
use crate::{Rect, Size, Visual};
use std::any::{Any, TypeId};
use std::rc::Rc;

/// Defines the interface for a renderer.
pub trait IRenderer {
    /// A value controlling the renderer's diagnostics.
    fn diagnostics(&self) -> Rc<RendererDiagnostics>;

    /// Subscribes to the notification raised when a portion of the scene
    /// has been invalidated.
    ///
    /// Indicates that the underlying low-level scene information has been
    /// updated. Used to signal that an update to the current pointer-over
    /// state may be required.
    fn scene_invalidated(&self, handler: Rc<dyn Fn(&SceneInvalidatedEventArgs)>) -> Rc<dyn IDisposable>;

    /// Mark a visual as dirty and needing re-rendering.
    fn add_dirty(&self, visual: &Visual);

    /// Informs the renderer that the z-ordering of a visual's children has
    /// changed.
    fn recalculate_children(&self, visual: &Visual);

    /// Called when a resize notification is received by the control being
    /// rendered.
    fn resized(&self, size: Size);

    /// Called when a paint notification is received by the control being
    /// rendered.
    fn paint(&self, rect: Rect);

    /// Starts the renderer.
    fn start(&self);

    /// Stops the renderer.
    fn stop(&self);

    /// Attempts to query for a feature from the platform render interface.
    ///
    /// Upstream this is asynchronous because the render interface lives on
    /// the render thread; the server compositor currently runs on the
    /// renderer's thread, so the answer is available immediately.
    fn try_get_render_interface_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>>;

    /// The compositor of the renderer, if it renders through one
    /// (upstream: the `IRendererWithCompositor` interface).
    fn compositor(&self) -> Option<Rc<Compositor>> {
        None
    }

    /// Releases the renderer's resources.
    fn dispose(&self);
}
