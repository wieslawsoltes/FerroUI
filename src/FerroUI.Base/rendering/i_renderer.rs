use super::composition::{
    CompositionDrawListVisual, Compositor, KeptCompositionVisuals, LocalFuture, RenderInterfaceFeature,
};
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

    /// A visual that has no composition visual is leaving the visual tree of the renderer and
    /// asks whether the renderer keeps what it drew until its next update pass, as it keeps a
    /// composition visual: `true` when the renderer took it. `false`, the default, has the
    /// visual let go of it at once (FerroUI deviation, docs/porting/desktop-performance.md,
    /// section 9.42).
    fn keep_departed_composition_item(&self, _visual: &Visual) -> bool {
        false
    }

    /// A visual whose composition visual was kept is attached to the visual tree of this
    /// renderer: whether the composition visual may be used there (section 9.32).
    fn rejoins_departed_composition_visual(&self, _composition_visual: &CompositionDrawListVisual) -> bool {
        false
    }

    /// A visual in the visual tree of the renderer that had no composition visual was given
    /// one outside the synchronisation of the renderer (something asked for it, or the visual
    /// set something of the composition API): the renderer places it in the composition tree,
    /// under the composition visual of the nearest visual above that has one and over those of
    /// the visuals below (FerroUI addition, docs/porting/desktop-performance.md, section 9.41).
    /// The default does nothing: a renderer without a compositor has no composition tree.
    fn composition_visual_materialized(&self, _visual: &Visual) {}

    /// Runs `departure`, in which visuals leave the visual tree of the renderer, and returns the
    /// composition visuals the renderer kept for them: they live until what is returned is
    /// released or dropped, and a visual that is in a tree of the renderer again before that has
    /// the composition visual it had (section 9.32). `departure` is called at most once. The
    /// default keeps nothing.
    fn keep_composition_visuals_departing_in(&self, departure: &mut dyn FnMut()) -> Option<KeptCompositionVisuals> {
        departure();
        None
    }

    /// The compositor of the renderer, if it renders through one
    /// (upstream: the `IRendererWithCompositor` interface).
    fn compositor(&self) -> Option<Rc<Compositor>> {
        None
    }

    /// Releases the renderer's resources.
    fn dispose(&self);
}
