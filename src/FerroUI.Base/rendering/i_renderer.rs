use super::composition::{CompositionDrawListVisual, Compositor, LocalFuture, RenderInterfaceFeature};
use super::{RendererDiagnostics, SceneInvalidatedEventArgs};
use crate::reactive::IDisposable;
use crate::{Rect, Size, Visual};
use std::any::TypeId;
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

    /// Mark a visual whose composition properties changed (its render transform or opacity)
    /// but whose own drawing did not: a renderer that keeps the drawing of a visual apart from
    /// its properties synchronizes the properties without recording the drawing again. The
    /// default records it again (FerroUI deviation, docs/porting/desktop-performance.md,
    /// section 9.5).
    fn add_dirty_properties(&self, visual: &Visual) {
        self.add_dirty(visual);
    }

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

    /// Attempts to query for a feature from the platform render interface:
    /// a future awaited on the thread of the renderer, which completes with
    /// the feature or `None` (see `Compositor::try_get_render_interface_feature`:
    /// asynchronous because the render interface may live on the render
    /// thread). The feature is an object of the server side: what is handed
    /// out lends it inside the compositor lock.
    fn try_get_render_interface_feature(&self, feature_type: TypeId) -> LocalFuture<Option<RenderInterfaceFeature>>;

    /// A visual is leaving the visual tree of the renderer and asks whether the renderer keeps
    /// its composition visual until its next update pass: `true` when the renderer took it, and
    /// will have the visual release it in that pass unless the visual is in a tree again by then
    /// (`Visual::release_departed_composition_visual`). `false`, the default, has the visual
    /// release it at once, as upstream (FerroUI deviation, docs/porting/desktop-performance.md,
    /// section 9.32).
    fn keep_departed_composition_visual(&self, _visual: &Visual, _composition_visual: &CompositionDrawListVisual) -> bool {
        false
    }

    /// A visual whose composition visual was kept is attached to the visual tree of this
    /// renderer: whether the composition visual may be used there (section 9.32).
    fn rejoins_departed_composition_visual(&self, _composition_visual: &CompositionDrawListVisual) -> bool {
        false
    }

    /// The compositor of the renderer, if it renders through one
    /// (upstream: the `IRendererWithCompositor` interface).
    fn compositor(&self) -> Option<Rc<Compositor>> {
        None
    }

    /// Releases the renderer's resources.
    fn dispose(&self);
}
