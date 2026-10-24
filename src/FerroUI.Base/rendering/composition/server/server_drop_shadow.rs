use super::{
    impl_server_render_resource, paint_composition_brush, IAnimatedServerObject, IServerObject, IServerRenderResource,
    ServerCompositionVisual, ServerCompositor, ServerRenderResource,
};
use crate::media::effects::ImmutableBlurEffect;
use crate::media::imaging::BitmapBlendingMode;
use crate::media::{BoxShadow, BoxShadows, Color, RenderOptions};
use crate::platform::{AlphaFormat, IDrawingContextImpl, PixelFormats, SharedBitmapImpl};
use crate::rendering::composition::generated::{ServerDropShadowHooks, ServerDropShadowProps};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::rendering::composition::CompositionDropShadowSourcePolicy;
use crate::{Matrix, PixelSize, Rect, RoundedRect, Vector};
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::time::Duration;

/// The side of the bitmap of the colour of a shadow, in pixels. Only its
/// middle is sampled, so that the filtering of its edges never shows.
const COLOR_BITMAP_SIDE: i32 = 4;

/// The server-side counterpart of a
/// [`DropShadow`](crate::rendering::composition::DropShadow).
///
/// The shadow of the rectangle of a visual is the box shadow of the drawing
/// context. The shadow of an alpha (a `Mask` brush, or the content of the
/// visual with the `InheritFromVisualContent` source policy) is drawn by
/// [`render_shadow_of`](Self::render_shadow_of) the same way on every backend
/// that has effects: the source, coloured by the shadow, offset and blurred.
pub struct ServerDropShadow {
    base: ServerRenderResource,
    props: ServerDropShadowProps,
    /// The bitmap of the colour of the shadow (premultiplied, with the
    /// opacity), and the colour it was made for.
    color_bitmap: RefCell<Option<(Color, Arc<SharedBitmapImpl>)>>,
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
            color_bitmap: RefCell::new(None),
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

    /// The colour of the shadow with its opacity in the alpha.
    pub fn effective_color(&self) -> Color {
        let color = self.props.color();
        let opacity = self.props.opacity().clamp(0.0, 1.0);
        let alpha = (color.a as f32 * opacity).round().clamp(0.0, 255.0) as u8;
        Color::from_argb(alpha, color.r, color.g, color.b)
    }

    /// The offset of the shadow in the plane of `visual`: the X and Y of
    /// `Offset`, and its Z as the renderers project that depth for the
    /// visual ([`ServerCompositionVisual::depth_offset`]); without a visual
    /// the X and Y alone.
    pub fn plane_offset(&self, visual: Option<&ServerCompositionVisual>) -> Vector {
        let offset = self.props.offset();
        let depth = match visual {
            Some(visual) => visual.depth_offset(offset.z as f32),
            None => Vector::default(),
        };
        Vector::new(offset.x + depth.x, offset.y + depth.y)
    }

    /// The shadow of the rectangle of `visual` as a box shadow: offset, blur
    /// radius, and the colour with the opacity of the shadow in its alpha.
    pub fn box_shadow_in(&self, visual: Option<&ServerCompositionVisual>) -> BoxShadow {
        let offset = self.plane_offset(visual);
        BoxShadow {
            offset_x: offset.x,
            offset_y: offset.y,
            blur: self.props.blur_radius().max(0.0) as f64,
            spread: 0.0,
            color: self.effective_color(),
            is_inset: false,
        }
    }

    /// [`box_shadow_in`](Self::box_shadow_in) without a visual.
    pub fn box_shadow(&self) -> BoxShadow {
        self.box_shadow_in(None)
    }

    /// The box shadows the shadow draws under a rectangle of `visual`: empty
    /// when the shadow is invisible.
    pub fn box_shadows_in(&self, visual: Option<&ServerCompositionVisual>) -> BoxShadows {
        let shadow = self.box_shadow_in(visual);
        if shadow.color.a == 0 {
            return BoxShadows::default();
        }
        BoxShadows::new(shadow)
    }

    /// [`box_shadows_in`](Self::box_shadows_in) without a visual.
    pub fn box_shadows(&self) -> BoxShadows {
        self.box_shadows_in(None)
    }

    /// The area the shadow of `rect` of `visual` covers, together with
    /// `rect`: the rectangle moved by the offset and grown by the blur
    /// radius. The shadow of an alpha inside `rect` stays inside it too.
    pub fn bounds_in(&self, visual: Option<&ServerCompositionVisual>, rect: Rect) -> Rect {
        let shadow = self.box_shadow_in(visual);
        rect.union(shadow.transform_bounds(rect))
    }

    /// [`bounds_in`](Self::bounds_in) without a visual.
    pub fn bounds(&self, rect: Rect) -> Rect {
        self.bounds_in(None, rect)
    }

    /// Whether the shadow is drawn at all: its colour and opacity leave some
    /// alpha.
    pub fn is_visible(&self) -> bool {
        self.effective_color().a != 0
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

    /// Draws the shadow of a sprite or layer visual whose area is `rect`,
    /// without the content of the visual: the shadow of the alpha of the
    /// mask when there is one ([`render_shadow_of`](Self::render_shadow_of)),
    /// otherwise the box shadow of the rectangle. Nothing for an invisible
    /// shadow or an empty rectangle.
    ///
    /// Deviation (winui-composition.md): without effects in the drawing
    /// context, the shadow of a mask is the box shadow of the rectangle.
    pub fn render_under_rect(
        &self,
        canvas: &mut dyn IDrawingContextImpl,
        visual: Option<&ServerCompositionVisual>,
        rect: Rect,
    ) {
        if rect.width <= 0.0 || rect.height <= 0.0 || !self.is_visible() {
            return;
        }
        if let Some(mask) = self.effective_mask() {
            if self.render_shadow_of(canvas, visual, rect, |canvas| paint_composition_brush(canvas, &mask, rect)) {
                return;
            }
        }
        let shadows = self.box_shadows_in(visual);
        if shadows.count() > 0 {
            canvas.draw_rectangle(None, None, RoundedRect::from_rect(rect), &shadows);
        }
    }

    /// Draws the shadow of the alpha of what `paint` draws inside `rect` (a
    /// mask brush, the brush of a sprite visual, the image of a mask): that
    /// alpha in the colour of the shadow, moved by its offset and blurred by
    /// its blur radius, and nothing of the source itself. Returns `false`,
    /// drawing nothing, when the drawing context has no effects or there is
    /// no render interface for the bitmap of the colour; the caller then
    /// draws the box shadow of the rectangle.
    ///
    /// Inside a blur effect, the source is painted into a layer at the
    /// offset of the shadow and a bitmap of the colour of the shadow is drawn
    /// over it with source-in blending, which keeps the colour where the
    /// source has alpha, in the proportion of that alpha: the operations
    /// every drawing context with effects has, so Skia and Vello draw the
    /// same shadow.
    pub fn render_shadow_of(
        &self,
        canvas: &mut dyn IDrawingContextImpl,
        visual: Option<&ServerCompositionVisual>,
        rect: Rect,
        paint: impl FnOnce(&mut dyn IDrawingContextImpl),
    ) -> bool {
        if rect.width <= 0.0 || rect.height <= 0.0 {
            return true;
        }
        let color = self.effective_color();
        if color.a == 0 {
            return true;
        }
        if canvas.as_drawing_context_impl_with_effects().is_none() {
            return false;
        }
        let Some(bitmap) = self.color_bitmap(color) else { return false };

        let offset = self.plane_offset(visual);
        let blur = self.props.blur_radius().max(0.0) as f64;
        let bounds = self.bounds_in(visual, rect);
        // The source is painted inside `rect`; the colour covers a little
        // more, so that its own edges are never part of the shadow.
        let colored = rect.inflate(2.0);

        let blurred = blur > 0.0;
        if blurred {
            if let Some(effects) = canvas.as_drawing_context_impl_with_effects() {
                effects.push_effect(Some(bounds), &ImmutableBlurEffect::new(blur));
            }
        }
        let transform = canvas.transform();
        let moved = offset != Vector::default();
        if moved {
            canvas.set_transform(Matrix::create_translation(offset.x, offset.y) * transform);
        }
        canvas.push_layer(colored);
        paint(&mut *canvas);
        canvas.push_render_options(RenderOptions {
            bitmap_blending_mode: BitmapBlendingMode::SourceIn,
            ..RenderOptions::default()
        });
        let side = COLOR_BITMAP_SIDE as f64;
        canvas.draw_bitmap(&*bitmap, 1.0, Rect::new(1.0, 1.0, side - 2.0, side - 2.0), colored);
        canvas.pop_render_options();
        canvas.pop_layer();
        if moved {
            canvas.set_transform(transform);
        }
        if blurred {
            if let Some(effects) = canvas.as_drawing_context_impl_with_effects() {
                effects.pop_effect();
            }
        }
        true
    }

    /// The bitmap of `color` (a few pixels of it, premultiplied), made with
    /// the render interface of the compositor and kept until the colour
    /// changes; `None` without a render interface.
    fn color_bitmap(&self, color: Color) -> Option<Arc<SharedBitmapImpl>> {
        if let Some((cached, bitmap)) = self.color_bitmap.borrow().as_ref() {
            if *cached == color {
                return Some(bitmap.clone());
            }
        }
        let compositor = self.base.compositor()?;
        let premultiply = |channel: u8| ((channel as u32 * color.a as u32 + 127) / 255) as u8;
        let pixel = [premultiply(color.r), premultiply(color.g), premultiply(color.b), color.a];
        let pixels: Vec<u8> = pixel.iter().copied().cycle().take((COLOR_BITMAP_SIDE * COLOR_BITMAP_SIDE * 4) as usize).collect();
        let size = PixelSize::new(COLOR_BITMAP_SIDE, COLOR_BITMAP_SIDE);
        let bitmap = compositor.render_interface().with_platform_render_interface(|render_interface| {
            render_interface.load_bitmap_from_pixels(
                PixelFormats::RGBA8888,
                AlphaFormat::Premul,
                &pixels,
                size,
                Vector::new(96.0, 96.0),
                COLOR_BITMAP_SIDE * 4,
            )
        })?;
        *self.color_bitmap.borrow_mut() = Some((color, bitmap.clone()));
        Some(bitmap)
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
        self.color_bitmap.borrow_mut().take();
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
