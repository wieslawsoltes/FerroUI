//! The server-side counterpart of the composition surface brush, and the
//! painting of composition brushes by the visuals that show them.

use crate::media::imaging::BitmapInterpolationMode;
use crate::media::{BoxShadows, RenderOptions};
use crate::platform::IDrawingContextImpl;
use crate::rendering::composition::generated::{
    ServerCompositionBrushHooks, ServerCompositionBrushProps, ServerCompositionSurfaceBrushHooks,
    ServerCompositionSurfaceBrushProps,
};
use crate::rendering::composition::server::{
    composition_transform, impl_server_render_resource, IAnimatedServerObject, IServerCompositionSurface,
    IServerObject, IServerRenderResource, ServerCompositor, ServerRenderResource,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::rendering::composition::CompositionStretch;
use crate::{Matrix, Rect, RoundedRect, Size, Vector};
use std::any::{Any, TypeId};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The server-side counterpart of a
/// [`CompositionSurfaceBrush`](super::CompositionSurfaceBrush).
///
/// Unlike the colour and gradient brushes it is not a brush of the drawing
/// context (`as_brush` is `None`): the visuals that show it paint it with
/// [`paint_composition_brush`].
pub struct ServerCompositionSurfaceBrush {
    this: Weak<ServerCompositionSurfaceBrush>,
    base: ServerRenderResource,
    props: ServerCompositionSurfaceBrushProps,
}

impl ServerCompositionSurfaceBrush {
    /// Creates the brush, active: the UI-thread brush activates its server
    /// object when it is created.
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionSurfaceBrush> {
        let brush = Rc::new_cyclic(|this: &Weak<ServerCompositionSurfaceBrush>| ServerCompositionSurfaceBrush {
            this: this.clone(),
            base: ServerRenderResource::new(
                compositor,
                this.clone() as Weak<dyn IAnimatedServerObject>,
                this.clone() as Weak<dyn IServerRenderResource>,
            ),
            props: ServerCompositionSurfaceBrushProps::new(),
        });
        brush.base.object().activate();
        brush
    }

    /// The properties of the brush.
    pub fn props(&self) -> &ServerCompositionSurfaceBrushProps {
        &self.props
    }

    /// The properties of the generated `ServerCompositionBrush`.
    pub fn brush_props(&self) -> &ServerCompositionBrushProps {
        self.props.base()
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    /// The surface the brush shows.
    pub fn surface(&self) -> Option<Rc<dyn IServerCompositionSurface>> {
        self.props.surface().and_then(|surface| surface.as_surface())
    }

    fn subscription_key(&self) -> usize {
        self.this.as_ptr() as *const () as usize
    }

    /// Follows the `Changed` notification of the surface: a change of its
    /// content invalidates the brush and so the visuals that show it. (A
    /// surface that is a render resource is observed by the brush as well.)
    fn subscribe(&self) {
        if let Some(surface) = self.surface() {
            let weak = self.this.clone();
            surface.changed().add(
                self.subscription_key(),
                Rc::new(move || {
                    if let Some(brush) = weak.upgrade() {
                        brush.base.core().invalidated(&*brush);
                    }
                }),
            );
        }
    }

    fn unsubscribe(&self) {
        if let Some(surface) = self.surface() {
            surface.changed().remove(self.subscription_key());
        }
    }

    /// The transform of the content of the brush in an area of `size`
    /// (`AnchorPoint`, `CenterPoint`, `Offset`, `RotationAngle`, `Scale`,
    /// `TransformMatrix`); `None` for the identity.
    pub fn content_transform(&self, size: Vector) -> Option<Matrix> {
        composition_transform(
            size,
            self.props.anchor_point(),
            self.props.center_point(),
            self.props.transform_matrix(),
            self.props.scale(),
            self.props.rotation_angle(),
            self.props.offset(),
        )
    }

    /// Paints the brush over `rect`, in the coordinates of `canvas`: the
    /// surface at its natural size placed by the stretch and the alignment
    /// ratios, transformed by the brush transform, clipped to `rect`.
    ///
    /// Deviation (winui-composition.md): the `Transform` and
    /// `RelativeTransform` the composition brushes of the upstream framework
    /// have are not applied to a surface brush; its own WinUI transform is.
    pub fn paint(&self, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        let Some(surface) = self.surface() else { return };
        let Some(natural_size) = surface.natural_size() else { return };
        let Some(mut content) = surface_brush_content_rect(
            natural_size,
            rect,
            self.props.stretch(),
            self.props.horizontal_alignment_ratio() as f64,
            self.props.vertical_alignment_ratio() as f64,
        ) else {
            return;
        };
        let opacity = self.brush_props().opacity();
        if opacity <= 0.0 {
            return;
        }

        canvas.push_clip(rect);

        // Deviation (winui-composition.md): the drawing context has one
        // interpolation quality; the WinUI default (`Linear`) leaves the
        // quality of the context as it is.
        let interpolation = self.props.bitmap_interpolation_mode().to_bitmap_interpolation_mode();
        let push_options = interpolation != BitmapInterpolationMode::LowQuality;
        if push_options {
            canvas.push_render_options(RenderOptions {
                bitmap_interpolation_mode: interpolation,
                ..RenderOptions::default()
            });
        }

        let saved = canvas.transform();
        let mut world = saved;
        if let Some(transform) = self.content_transform(Vector::new(rect.width, rect.height)) {
            // The brush transform is in the space of the area: its origin is
            // the top left corner of `rect`.
            let local = Matrix::create_translation(-rect.x, -rect.y)
                * transform
                * Matrix::create_translation(rect.x, rect.y);
            world = local * saved;
        }
        if self.props.snap_to_pixels() {
            // Snaps the top left corner of the content to a device pixel.
            let corner = content.top_left().transform(world);
            let (dx, dy) = (corner.x.round() - corner.x, corner.y.round() - corner.y);
            if dx != 0.0 || dy != 0.0 {
                world = world * Matrix::create_translation(dx, dy);
            }
            content = Rect::new(content.x, content.y, content.width.round(), content.height.round());
        }
        let transformed = world != saved;
        if transformed {
            canvas.set_transform(world);
        }
        surface.draw(canvas, content, opacity);
        if transformed {
            canvas.set_transform(saved);
        }

        if push_options {
            canvas.pop_render_options();
        }
        canvas.pop_clip();
    }
}

/// Where a surface brush puts the whole content of a surface of
/// `natural_size` in the area `area`: the content is sized by `stretch`
/// (`None` keeps the natural size, `Fill` takes the size of the area,
/// `Uniform` fits it inside, `UniformToFill` covers the area), then placed so
/// that `horizontal_ratio` / `vertical_ratio` of the free space (negative
/// when the content is larger) is left of / above it: 0 aligns the left or
/// top edges, 0.5 centres, 1 aligns the right or bottom edges. `None` for an
/// empty surface or area.
pub fn surface_brush_content_rect(
    natural_size: Size,
    area: Rect,
    stretch: CompositionStretch,
    horizontal_ratio: f64,
    vertical_ratio: f64,
) -> Option<Rect> {
    if natural_size.width <= 0.0 || natural_size.height <= 0.0 || area.width <= 0.0 || area.height <= 0.0 {
        return None;
    }
    let (width, height) = match stretch {
        CompositionStretch::None => (natural_size.width, natural_size.height),
        CompositionStretch::Fill => (area.width, area.height),
        CompositionStretch::Uniform => {
            let scale = (area.width / natural_size.width).min(area.height / natural_size.height);
            (natural_size.width * scale, natural_size.height * scale)
        }
        CompositionStretch::UniformToFill => {
            let scale = (area.width / natural_size.width).max(area.height / natural_size.height);
            (natural_size.width * scale, natural_size.height * scale)
        }
    };
    Some(Rect::new(
        area.x + (area.width - width) * horizontal_ratio,
        area.y + (area.height - height) * vertical_ratio,
        width,
        height,
    ))
}

/// Paints a composition brush over `rect`, in the coordinates of `canvas`:
/// what a sprite visual (and later a shape) does with its brush. A surface
/// brush is painted by [`ServerCompositionSurfaceBrush::paint`], a nine-grid,
/// mask or effect brush by its own `paint` (a backdrop brush paints nothing); a brush of
/// the drawing context (the colour and gradient brushes) fills the
/// rectangle.
pub fn paint_composition_brush(canvas: &mut dyn IDrawingContextImpl, brush: &Rc<dyn IServerObject>, rect: Rect) {
    if let Some(surface_brush) = brush.as_any().downcast_ref::<ServerCompositionSurfaceBrush>() {
        surface_brush.paint(canvas, rect);
    } else if let Some(nine_grid) = brush.as_any().downcast_ref::<super::ServerCompositionNineGridBrush>() {
        nine_grid.paint(canvas, rect);
    } else if let Some(mask) = brush.as_any().downcast_ref::<super::ServerCompositionMaskBrush>() {
        mask.paint(canvas, rect);
    } else if let Some(effect) = brush.as_any().downcast_ref::<super::ServerCompositionEffectBrush>() {
        effect.paint(canvas, rect);
    } else if let Some(backdrop) = brush.as_any().downcast_ref::<super::ServerCompositionBackdropBrush>() {
        backdrop.paint(canvas, rect);
    } else if let Some(platform_brush) = brush.clone().as_brush() {
        canvas.draw_rectangle(Some(&*platform_brush), None, RoundedRect::from_rect(rect), &BoxShadows::default());
    }
}

impl_server_render_resource!(
    ServerCompositionSurfaceBrush,
    base,
    composition_property: ServerCompositionSurfaceBrushProps::get_composition_property
);

impl ServerCompositionBrushHooks for ServerCompositionSurfaceBrush {}

impl ServerCompositionSurfaceBrushHooks for ServerCompositionSurfaceBrush {
    fn on_surface_changing(&self) {
        self.unsubscribe();
    }

    fn on_surface_changed(&self) {
        self.subscribe();
    }
}

impl IServerObject for ServerCompositionSurfaceBrush {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
    }

    fn values_invalidated(&self) {
        self.base.core().invalidated(self);
    }

    fn dispose(&self) {
        self.unsubscribe();
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
