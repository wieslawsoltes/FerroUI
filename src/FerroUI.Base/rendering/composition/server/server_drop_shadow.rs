use super::{
    drop_shadow_effect, impl_server_render_resource, paint_composition_brush, IAnimatedServerObject, IServerObject,
    IServerRenderResource, ServerCompositor, ServerRenderResource,
};
use crate::media::imaging::BitmapBlendingMode;
use crate::media::{BoxShadow, BoxShadows, Color, RenderOptions};
use crate::platform::IDrawingContextImpl;
use crate::rendering::composition::generated::{ServerDropShadowHooks, ServerDropShadowProps};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::rendering::composition::CompositionDropShadowSourcePolicy;
use crate::{Rect, RoundedRect};
use std::any::{Any, TypeId};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The server-side counterpart of a
/// [`DropShadow`](crate::rendering::composition::DropShadow).
pub struct ServerDropShadow {
    base: ServerRenderResource,
    props: ServerDropShadowProps,
}

impl ServerDropShadow {
    /// Creates the shadow, active: the UI-thread shadow activates its server
    /// object when it is created.
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerDropShadow> {
        let shadow = Rc::new_cyclic(|this: &Weak<ServerDropShadow>| ServerDropShadow {
            base: ServerRenderResource::new(
                compositor,
                this.clone() as Weak<dyn IAnimatedServerObject>,
                this.clone() as Weak<dyn IServerRenderResource>,
            ),
            props: ServerDropShadowProps::new(),
        });
        shadow.base.object().activate();
        shadow
    }

    /// The properties of the shadow.
    pub fn props(&self) -> &ServerDropShadowProps {
        &self.props
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    /// The shadow as the box shadow of a rectangle: offset, blur radius, and
    /// the colour with the opacity of the shadow in its alpha.
    ///
    /// Deviation (winui-composition.md): the Z offset is ignored.
    pub fn box_shadow(&self) -> BoxShadow {
        let color = self.props.color();
        let opacity = self.props.opacity().clamp(0.0, 1.0);
        let alpha = (color.a as f32 * opacity).round().clamp(0.0, 255.0) as u8;
        let offset = self.props.offset();
        BoxShadow {
            offset_x: offset.x,
            offset_y: offset.y,
            blur: self.props.blur_radius().max(0.0) as f64,
            spread: 0.0,
            color: Color::from_argb(alpha, color.r, color.g, color.b),
            is_inset: false,
        }
    }

    /// The box shadows the shadow draws under a rectangle: empty when the
    /// shadow is invisible.
    pub fn box_shadows(&self) -> BoxShadows {
        let shadow = self.box_shadow();
        if shadow.color.a == 0 {
            return BoxShadows::default();
        }
        BoxShadows::new(shadow)
    }

    /// The area the shadow of `rect` covers, together with `rect`.
    pub fn bounds(&self, rect: Rect) -> Rect {
        self.box_shadows().transform_bounds(rect)
    }

    /// Whether the shadow is drawn at all: its colour and opacity leave some
    /// alpha.
    pub fn is_visible(&self) -> bool {
        self.box_shadow().color.a != 0
    }

    /// Whether the shadow takes the alpha of the content of its visual
    /// (`SourcePolicy` `InheritFromVisualContent`), which wins over the mask.
    pub fn inherits_from_visual_content(&self) -> bool {
        self.props.source_policy() == CompositionDropShadowSourcePolicy::InheritFromVisualContent
    }

    /// The mask that shapes the shadow: the `Mask` brush, unless the shadow
    /// takes the alpha of the content of its visual.
    pub fn effective_mask(&self) -> Option<Rc<dyn IServerObject>> {
        if self.inherits_from_visual_content() {
            return None;
        }
        self.props.mask()
    }

    /// Draws the shadow of a visual whose area is `rect`, without the
    /// content of the visual: the shadow of the mask when there is one,
    /// otherwise the box shadow of the rectangle. Nothing for an invisible
    /// shadow or an empty rectangle.
    ///
    /// A colour or gradient mask (a brush of the drawing context) is the
    /// opacity mask of the box shadow over the area of the shadow. Any other
    /// mask (a surface, nine-grid or mask brush) is painted through a drop
    /// shadow effect when the drawing context has effects, in a layer from
    /// which the mask itself is then removed with the destination-out
    /// blending of bitmaps, so only its shadow stays; without effects the
    /// box shadow of the rectangle is drawn.
    ///
    /// Deviation (winui-composition.md): a colour or gradient mask scales
    /// the box shadow of the rectangle (its gradient is spread over the area
    /// of the shadow, not blurred with it); the shadow of any other mask is
    /// removed where the bitmaps of the mask are (the content drawn over it
    /// covers that area in the usual case of a mask that is the content),
    /// and only bitmaps are removed.
    pub fn render_under_rect(&self, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return;
        }
        let shadows = self.box_shadows();
        if shadows.count() == 0 {
            return;
        }
        let bounds = self.bounds(rect);
        let Some(mask) = self.effective_mask() else {
            canvas.draw_rectangle(None, None, RoundedRect::from_rect(rect), &shadows);
            return;
        };
        if let Some(mask_brush) = mask.clone().as_brush() {
            canvas.push_opacity_mask(&*mask_brush, bounds);
            canvas.draw_rectangle(None, None, RoundedRect::from_rect(rect), &shadows);
            canvas.pop_opacity_mask();
            return;
        }
        if canvas.as_drawing_context_impl_with_effects().is_none() {
            canvas.draw_rectangle(None, None, RoundedRect::from_rect(rect), &shadows);
            return;
        }
        let effect = drop_shadow_effect(self);
        canvas.push_layer(bounds);
        if let Some(effects) = canvas.as_drawing_context_impl_with_effects() {
            effects.push_effect(Some(bounds), &effect);
        }
        paint_composition_brush(canvas, &mask, rect);
        if let Some(effects) = canvas.as_drawing_context_impl_with_effects() {
            effects.pop_effect();
        }
        // The mask itself goes: only its shadow stays.
        canvas.push_render_options(RenderOptions {
            bitmap_blending_mode: BitmapBlendingMode::DestinationOut,
            ..RenderOptions::default()
        });
        paint_composition_brush(canvas, &mask, rect);
        canvas.pop_render_options();
        canvas.pop_layer();
    }

    /// Draws the content of a visual whose area is `rect` (`paint`) with the
    /// shadow of the alpha of that content under it, through a drop shadow
    /// effect of the drawing context: the `InheritFromVisualContent` source
    /// policy. Returns `false`, drawing nothing, when the drawing context has
    /// no effects; the caller then draws the shadow with
    /// [`render_under_rect`](Self::render_under_rect) and the content itself.
    pub fn render_with_content(
        &self,
        canvas: &mut dyn IDrawingContextImpl,
        rect: Rect,
        paint: impl FnOnce(&mut dyn IDrawingContextImpl),
    ) -> bool {
        if canvas.as_drawing_context_impl_with_effects().is_none() {
            return false;
        }
        let effect = drop_shadow_effect(self);
        let bounds = self.bounds(rect);
        if let Some(effects) = canvas.as_drawing_context_impl_with_effects() {
            effects.push_effect(Some(bounds), &effect);
        }
        paint(&mut *canvas);
        if let Some(effects) = canvas.as_drawing_context_impl_with_effects() {
            effects.pop_effect();
        }
        true
    }
}

impl_server_render_resource!(
    ServerDropShadow,
    base,
    composition_property: ServerDropShadowProps::get_composition_property
);

impl ServerDropShadowHooks for ServerDropShadow {}

impl IServerObject for ServerDropShadow {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
    }

    fn values_invalidated(&self) {
        self.base.core().invalidated(self);
    }

    fn dispose(&self) {
        self.base.core().dispose();
    }

    fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
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
