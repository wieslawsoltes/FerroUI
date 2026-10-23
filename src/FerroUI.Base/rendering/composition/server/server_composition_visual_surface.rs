use super::{
    impl_server_render_resource, IAnimatedServerObject, IServerCompositionSurface, IServerObject,
    IServerRenderResource, ServerCompositionSurfaceChanged, ServerCompositionVisual, ServerCompositor,
    ServerRenderResource,
};
use crate::media::Colors;
use crate::platform::{IDrawingContextImpl, IDrawingContextLayerImpl, LtrbRect};
use crate::rendering::composition::generated::{
    ServerCompositionVisualSurfaceHooks, ServerCompositionVisualSurfaceProps,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Matrix, PixelSize, Rect, Size, Vector};
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The server-side counterpart of a
/// [`CompositionVisualSurface`](crate::rendering::composition::CompositionVisualSurface):
/// a live image of the subtree of a visual.
///
/// The subtree is rendered where the surface is drawn, as the bitmap cache
/// renders it: the visibility, opacity, transform and clip of the source
/// visual itself are ignored, so a hidden source still shows. A surface
/// that shows an ancestor of the visual that draws it would draw itself
/// again inside that drawing; the second drawing is skipped.
///
/// The compositor keeps a list of the visual surfaces and, once per frame
/// before the targets are updated, invalidates the surfaces whose source
/// subtree changed ([`update_live_surfaces`](Self::update_live_surfaces)),
/// which redraws the visuals that show them.
///
/// A frozen surface shows the image of the source taken when it was frozen:
/// the UI-thread surface asks for the freeze with its next batch, and the
/// compositor renders the image in the same pass ([`freeze`](Self::freeze)).
pub struct ServerCompositionVisualSurface {
    base: ServerRenderResource,
    props: ServerCompositionVisualSurfaceProps,
    changed: ServerCompositionSurfaceChanged,
    /// Set while the source is rendered: the guard against a surface that
    /// shows an ancestor of the visual that draws it.
    rendering: Cell<bool>,
    freeze_requested: Cell<bool>,
    frozen: Cell<bool>,
    frozen_content: RefCell<Option<FrozenContent>>,
    /// Whether a change of the extent of the source subtree counts as a
    /// change of the surface (the surface of a redirect visual, whose bounds
    /// cover the source).
    follows_source_bounds: Cell<bool>,
    followed_source_bounds: Cell<Option<LtrbRect>>,
}

/// The image of a frozen visual surface: an offscreen layer, and the size of
/// the source area it holds.
struct FrozenContent {
    layer: Rc<dyn IDrawingContextLayerImpl>,
    size: Size,
}

impl ServerCompositionVisualSurface {
    /// Creates the surface, active, and registers it with the compositor
    /// for the live updates.
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionVisualSurface> {
        let surface = Rc::new_cyclic(|this: &Weak<ServerCompositionVisualSurface>| ServerCompositionVisualSurface {
            base: ServerRenderResource::new(
                compositor,
                this.clone() as Weak<dyn IAnimatedServerObject>,
                this.clone() as Weak<dyn IServerRenderResource>,
            ),
            props: ServerCompositionVisualSurfaceProps::new(),
            changed: ServerCompositionSurfaceChanged::new(),
            rendering: Cell::new(false),
            freeze_requested: Cell::new(false),
            frozen: Cell::new(false),
            frozen_content: RefCell::new(None),
            follows_source_bounds: Cell::new(false),
            followed_source_bounds: Cell::new(None),
        });
        surface.base.object().activate();
        compositor.register_visual_surface(Rc::downgrade(&surface));
        surface
    }

    /// The properties of the surface.
    pub fn props(&self) -> &ServerCompositionVisualSurfaceProps {
        &self.props
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    /// Whether the surface is frozen.
    pub fn is_frozen(&self) -> bool {
        self.frozen.get()
    }

    /// The visual whose subtree the surface shows.
    pub fn source_visual(&self) -> Option<Rc<ServerCompositionVisual>> {
        self.props.source_visual().and_then(|visual| visual.into_any_rc().downcast::<ServerCompositionVisual>().ok())
    }

    /// The area of the source visual the surface shows, in the coordinates
    /// of the source: `SourceOffset` and `SourceSize`, where a zero
    /// `SourceSize` is the size of the source visual. `None` when the area
    /// is empty or there is no source.
    pub fn source_rect(&self) -> Option<Rect> {
        let source = self.source_visual()?;
        let offset = self.props.source_offset();
        let mut size = self.props.source_size();
        if size == Vector::default() {
            size = source.size();
        }
        (size.x > 0.0 && size.y > 0.0).then(|| Rect::new(offset.x, offset.y, size.x, size.y))
    }

    /// Asks for the surface to be frozen by the next pass of the compositor
    /// (what the batch of a UI-thread [`freeze`](crate::rendering::composition::CompositionVisualSurface::freeze)
    /// carries).
    pub fn request_freeze(&self) {
        if !self.frozen.get() {
            self.freeze_requested.set(true);
        }
    }

    /// The surfaces that were asked to freeze are frozen, and the live
    /// surfaces whose source changed since the last update of its target are
    /// invalidated, which redraws the visuals that show them. Run once per
    /// frame by the compositor, after the own properties of the visuals were
    /// recomputed and before the targets are updated. Returns whether a
    /// surface changed. Dropped surfaces are removed from `surfaces`.
    pub fn update_live_surfaces(
        compositor: &ServerCompositor,
        surfaces: &RefCell<Vec<Weak<ServerCompositionVisualSurface>>>,
    ) -> bool {
        let live: Vec<Rc<ServerCompositionVisualSurface>> = {
            let mut surfaces = surfaces.borrow_mut();
            surfaces.retain(|surface| surface.upgrade().is_some_and(|surface| !surface.is_disposed()));
            surfaces.iter().filter_map(Weak::upgrade).collect()
        };
        let mut changed = false;
        for surface in live {
            if surface.freeze_requested.get() {
                // The image needs the graphics: a freeze waits for them.
                if compositor.is_rendering_stopped() || !compositor.render_interface().is_ready() {
                    continue;
                }
                surface.freeze(compositor);
                changed = true;
            } else if surface.source_changed() {
                surface.content_changed();
                changed = true;
            }
        }
        changed
    }

    /// Whether the source subtree changed since the last update of the
    /// target it is attached to. A source that is not attached to a target
    /// is not updated, so its changes are not followed.
    fn source_changed(&self) -> bool {
        if self.frozen.get() {
            return false;
        }
        let Some(source) = self.source_visual() else { return false };
        if source.root().is_none() {
            return false;
        }
        let mut changed = source.has_pending_render_changes();
        if self.follows_source_bounds.get() {
            // The bounds of the last update of the target, which the
            // pending changes of the last frame may have moved.
            let bounds = source.unclipped_sub_tree_bounds();
            if self.followed_source_bounds.replace(bounds) != bounds {
                changed = true;
            }
        }
        changed
    }

    /// Makes a change of the extent of the source subtree a change of the
    /// surface (what a redirect visual, whose bounds cover its source, needs;
    /// not from upstream).
    pub fn set_follows_source_bounds(&self, value: bool) {
        self.follows_source_bounds.set(value);
    }

    /// Renders the whole subtree of the source onto `canvas`, whose transform
    /// maps the coordinates of the source, as the bitmap cache renders it
    /// (the visibility, opacity, transform and clip of the source itself are
    /// ignored), unless the surface is rendering its source already: what a
    /// redirect visual draws (not from upstream). `clip` is in the
    /// coordinates of the device.
    pub fn render_source_subtree(&self, canvas: &mut dyn IDrawingContextImpl, clip: LtrbRect) {
        let Some(source) = self.source_visual() else { return };
        self.render_source(&source, canvas, clip);
    }

    fn content_changed(&self) {
        self.base.core().invalidated(self);
        self.changed.invoke();
    }

    /// Renders the source into an offscreen layer once, and shows that layer
    /// from now on (not a public member in WinUI; the connected animation
    /// freezes the image of its source). Runs on the render thread, with the
    /// bounds of the source subtree as of the last frame: best effort, a
    /// source that was never rendered in a target has no bounds for its
    /// children yet.
    pub fn freeze(&self, compositor: &ServerCompositor) {
        self.freeze_requested.set(false);
        if self.frozen.replace(true) {
            return;
        }
        let (Some(source), Some(rect)) = (self.source_visual(), self.source_rect()) else {
            self.content_changed();
            return;
        };
        let scaling = source.root().map(|root| root.scaling()).filter(|scaling| *scaling > 0.0).unwrap_or(1.0);
        let size = Size::new(rect.width, rect.height);
        let pixel_size = PixelSize::from_size_ceiling(size, scaling);
        if pixel_size.width < 1 || pixel_size.height < 1 {
            self.content_changed();
            return;
        }

        let current = compositor.render_interface().ensure_current();
        let layer = compositor.render_interface().value().create_offscreen_render_target(
            pixel_size,
            Vector::new(scaling, scaling),
            false,
        );
        {
            let mut canvas = layer.create_drawing_context();
            canvas.clear(Colors::TRANSPARENT);
            canvas.set_transform(
                Matrix::create_translation(-rect.x, -rect.y) * Matrix::create_scale(scaling, scaling),
            );
            self.render_source(&source, &mut *canvas, LtrbRect::INFINITE);
            canvas.dispose();
        }
        current.dispose();
        *self.frozen_content.borrow_mut() = Some(FrozenContent { layer, size });
        self.content_changed();
    }

    /// Renders the subtree of `source` onto `canvas` (whose transform maps
    /// the coordinates of the source), unless it is being rendered already.
    fn render_source(&self, source: &Rc<ServerCompositionVisual>, canvas: &mut dyn IDrawingContextImpl, clip: LtrbRect) {
        if self.rendering.replace(true) {
            return;
        }
        // The root of a bitmap-cache walk skips its own transform already; asking to skip it as
        // well would leave the request to the first child.
        source.render(canvas, clip, None, true, false, true);
        self.rendering.set(false);
    }

    /// Whether the source is being rendered by this surface: the drawing of
    /// the surface inside it is skipped.
    pub fn is_rendering(&self) -> bool {
        self.rendering.get()
    }

    fn free_frozen_content(&self) {
        if let Some(content) = self.frozen_content.borrow_mut().take() {
            content.layer.dispose();
        }
    }
}

impl_server_render_resource!(
    ServerCompositionVisualSurface,
    base,
    composition_property: ServerCompositionVisualSurfaceProps::get_composition_property
);

impl ServerCompositionVisualSurfaceHooks for ServerCompositionVisualSurface {
    /// The request to freeze, written by the UI-thread surface before its
    /// properties.
    fn deserialize_changes_extra(&self, reader: &mut BatchStreamReader<'_>) {
        if reader.read::<bool>() {
            self.request_freeze();
        }
    }
}

impl IServerObject for ServerCompositionVisualSurface {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
    }

    fn values_invalidated(&self) {
        self.content_changed();
    }

    fn dispose(&self) {
        self.free_frozen_content();
        self.base.core().dispose();
    }

    fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
        Some(self)
    }

    fn as_surface(self: Rc<Self>) -> Option<Rc<dyn IServerCompositionSurface>> {
        Some(self)
    }

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        Some(self)
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl IServerCompositionSurface for ServerCompositionVisualSurface {
    /// A visual surface has no bitmap: its content is drawn by
    /// [`draw`](IServerCompositionSurface::draw).
    fn bitmap(&self) -> Option<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
        None
    }

    fn changed(&self) -> &ServerCompositionSurfaceChanged {
        &self.changed
    }

    fn natural_size(&self) -> Option<Size> {
        if let Some(content) = self.frozen_content.borrow().as_ref() {
            return Some(content.size);
        }
        if self.frozen.get() {
            return None;
        }
        self.source_rect().map(|rect| Size::new(rect.width, rect.height))
    }

    /// Draws the source area of the source visual into `dest`: the frozen
    /// image, or the subtree rendered with the transform that maps the
    /// source area onto `dest`, clipped to `dest`.
    fn draw(&self, canvas: &mut dyn IDrawingContextImpl, dest: Rect, opacity: f64) {
        if dest.width <= 0.0 || dest.height <= 0.0 {
            return;
        }
        if let Some(content) = self.frozen_content.borrow().as_ref() {
            let pixel_size = content.layer.pixel_size();
            canvas.draw_bitmap(
                &*content.layer,
                opacity,
                Rect::new(0.0, 0.0, pixel_size.width as f64, pixel_size.height as f64),
                dest,
            );
            return;
        }
        if self.frozen.get() || self.rendering.get() {
            return;
        }
        let (Some(source), Some(rect)) = (self.source_visual(), self.source_rect()) else { return };

        let saved = canvas.transform();
        canvas.push_clip(dest);
        if opacity != 1.0 {
            canvas.push_opacity(opacity, Some(dest));
        }
        let map = Matrix::create_translation(-rect.x, -rect.y)
            * Matrix::create_scale(dest.width / rect.width, dest.height / rect.height)
            * Matrix::create_translation(dest.x, dest.y);
        canvas.set_transform(map * saved);
        let clip = LtrbRect::from_rect(dest).transform_to_aabb(saved);
        self.render_source(&source, canvas, clip);
        canvas.set_transform(saved);
        if opacity != 1.0 {
            canvas.pop_opacity();
        }
        canvas.pop_clip();
    }
}
