//! The server-side counterparts of the WinUI nine-grid and mask brushes, and
//! the rectangles of a nine-grid.

use super::{paint_composition_brush, ServerCompositionSurfaceBrush};
use crate::media::imaging::BitmapBlendingMode;
use crate::media::{BoxShadows, RenderOptions};
use crate::platform::IDrawingContextImpl;
use crate::rendering::composition::generated::{
    ServerCompositionBrushHooks, ServerCompositionBrushProps, ServerCompositionMaskBrushHooks,
    ServerCompositionMaskBrushProps, ServerCompositionNineGridBrushHooks, ServerCompositionNineGridBrushProps,
};
use crate::rendering::composition::server::{
    impl_server_render_resource, IAnimatedServerObject, IServerObject, IServerRenderResource, ServerCompositor,
    ServerRenderResource,
};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::{Rect, RoundedRect};
use std::any::{Any, TypeId};
use std::cell::Cell;
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The four edges of a nine-grid, in the order left, top, right, bottom.
pub type NineGridEdges = [f64; 4];

/// The index of the centre cell among the nine cells of
/// [`nine_grid_rects`].
pub const NINE_GRID_CENTER: usize = 4;

/// The lines that cut `length` from `start` into three: `first` from the
/// start, `last` from the end. Negative insets count as zero; insets that
/// together exceed the length are scaled down to meet.
fn nine_grid_lines(start: f64, length: f64, first: f64, last: f64) -> [f64; 4] {
    let (mut first, mut last) = (first.max(0.0), last.max(0.0));
    let length = length.max(0.0);
    if first + last > length && first + last > 0.0 {
        let scale = length / (first + last);
        first *= scale;
        last *= scale;
    }
    [start, start + first, start + length - last, start + length]
}

/// The nine cells of a nine-grid, row by row from the top left corner (the
/// centre is [`NINE_GRID_CENTER`]): each pair is the cell of `source` cut by
/// `insets` and the cell of `dest` it is drawn into, cut by `insets` times
/// `scales`. Insets that do not fit are scaled down to meet, in the source
/// and in the destination alike.
pub fn nine_grid_rects(source: Rect, dest: Rect, insets: NineGridEdges, scales: NineGridEdges) -> [(Rect, Rect); 9] {
    let [left, top, right, bottom] = insets;
    let [left_scale, top_scale, right_scale, bottom_scale] = scales;
    let source_x = nine_grid_lines(source.x, source.width, left, right);
    let source_y = nine_grid_lines(source.y, source.height, top, bottom);
    let dest_x = nine_grid_lines(dest.x, dest.width, left * left_scale, right * right_scale);
    let dest_y = nine_grid_lines(dest.y, dest.height, top * top_scale, bottom * bottom_scale);
    let cell = |x: &[f64; 4], y: &[f64; 4], column: usize, row: usize| {
        Rect::new(x[column], y[row], x[column + 1] - x[column], y[row + 1] - y[row])
    };
    std::array::from_fn(|index| {
        let (row, column) = (index / 3, index % 3);
        (cell(&source_x, &source_y, column, row), cell(&dest_x, &dest_y, column, row))
    })
}

fn is_empty(rect: &Rect) -> bool {
    rect.width <= 0.0 || rect.height <= 0.0
}

/// The server-side counterpart of a
/// [`CompositionNineGridBrush`](super::CompositionNineGridBrush), painted
/// by the visuals that show it ([`paint_composition_brush`]).
pub struct ServerCompositionNineGridBrush {
    base: ServerRenderResource,
    props: ServerCompositionNineGridBrushProps,
    /// Set while the brush paints: a source that contains the brush is not
    /// painted inside itself.
    painting: Cell<bool>,
}

impl ServerCompositionNineGridBrush {
    /// Creates the brush, active: the UI-thread brush activates its server
    /// object when it is created.
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionNineGridBrush> {
        let brush = Rc::new_cyclic(|this: &Weak<ServerCompositionNineGridBrush>| ServerCompositionNineGridBrush {
            base: ServerRenderResource::new(
                compositor,
                this.clone() as Weak<dyn IAnimatedServerObject>,
                this.clone() as Weak<dyn IServerRenderResource>,
            ),
            props: ServerCompositionNineGridBrushProps::new(),
            painting: Cell::new(false),
        });
        brush.base.object().activate();
        brush
    }

    /// The properties of the brush.
    pub fn props(&self) -> &ServerCompositionNineGridBrushProps {
        &self.props
    }

    /// The properties of the generated `ServerCompositionBrush`.
    pub fn brush_props(&self) -> &ServerCompositionBrushProps {
        self.props.base()
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    /// The insets, left, top, right, bottom.
    pub fn insets(&self) -> NineGridEdges {
        [
            self.props.left_inset() as f64,
            self.props.top_inset() as f64,
            self.props.right_inset() as f64,
            self.props.bottom_inset() as f64,
        ]
    }

    /// The inset scales, left, top, right, bottom.
    pub fn inset_scales(&self) -> NineGridEdges {
        [
            self.props.left_inset_scale() as f64,
            self.props.top_inset_scale() as f64,
            self.props.right_inset_scale() as f64,
            self.props.bottom_inset_scale() as f64,
        ]
    }

    /// Paints the brush over `rect`, in the coordinates of `canvas`.
    ///
    /// A surface brush source has its surface cut at its natural size and
    /// each cell drawn into its cell of `rect`. A solid colour source fills
    /// the cells of `rect`; any other source is painted over `rect`, clipped
    /// to the cells (all at once when the centre is not hollow).
    pub fn paint(&self, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        let Some(source) = self.props.source() else { return };
        let opacity = self.brush_props().opacity();
        if opacity <= 0.0 || is_empty(&rect) || self.painting.replace(true) {
            return;
        }
        if opacity != 1.0 {
            canvas.push_opacity(opacity, Some(rect));
        }
        let hollow = self.props.is_center_hollow();
        let insets = self.insets();
        let scales = self.inset_scales();

        if let Some(surface_brush) = source.as_any().downcast_ref::<ServerCompositionSurfaceBrush>() {
            let surface = surface_brush.surface();
            let natural_size = surface.as_ref().and_then(|surface| surface.natural_size());
            let surface_opacity = surface_brush.brush_props().opacity();
            if let (Some(surface), Some(natural_size)) = (surface, natural_size) {
                let natural = Rect::new(0.0, 0.0, natural_size.width, natural_size.height);
                for (index, (from, to)) in nine_grid_rects(natural, rect, insets, scales).iter().enumerate() {
                    if (hollow && index == NINE_GRID_CENTER) || is_empty(from) || is_empty(to) {
                        continue;
                    }
                    // The whole surface, placed so that the source cell covers
                    // the destination cell, clipped to that cell.
                    let scale_x = to.width / from.width;
                    let scale_y = to.height / from.height;
                    let whole = Rect::new(
                        to.x - from.x * scale_x,
                        to.y - from.y * scale_y,
                        natural.width * scale_x,
                        natural.height * scale_y,
                    );
                    canvas.push_clip(*to);
                    surface.draw(canvas, whole, surface_opacity);
                    canvas.pop_clip();
                }
            }
        } else {
            let solid = source.clone().as_brush().filter(|brush| brush.as_solid_color_brush().is_some());
            let cells = nine_grid_rects(rect, rect, insets, scales);
            if let Some(solid) = solid {
                for (index, (_, to)) in cells.iter().enumerate() {
                    if (hollow && index == NINE_GRID_CENTER) || is_empty(to) {
                        continue;
                    }
                    canvas.draw_rectangle(Some(&*solid), None, RoundedRect::from_rect(*to), &BoxShadows::default());
                }
            } else if !hollow {
                paint_composition_brush(canvas, &source, rect);
            } else {
                for (index, (_, to)) in cells.iter().enumerate() {
                    if index == NINE_GRID_CENTER || is_empty(to) {
                        continue;
                    }
                    canvas.push_clip(*to);
                    paint_composition_brush(canvas, &source, rect);
                    canvas.pop_clip();
                }
            }
        }

        if opacity != 1.0 {
            canvas.pop_opacity();
        }
        self.painting.set(false);
    }
}

impl_server_render_resource!(
    ServerCompositionNineGridBrush,
    base,
    composition_property: ServerCompositionNineGridBrushProps::get_composition_property
);

impl ServerCompositionBrushHooks for ServerCompositionNineGridBrush {}

impl ServerCompositionNineGridBrushHooks for ServerCompositionNineGridBrush {}

impl IServerObject for ServerCompositionNineGridBrush {
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

/// The server-side counterpart of a
/// [`CompositionMaskBrush`](super::CompositionMaskBrush), painted by the
/// visuals that show it ([`paint_composition_brush`]).
pub struct ServerCompositionMaskBrush {
    base: ServerRenderResource,
    props: ServerCompositionMaskBrushProps,
    /// Set while the brush paints: a source or mask that contains the brush
    /// is not painted inside itself.
    painting: Cell<bool>,
}

impl ServerCompositionMaskBrush {
    /// Creates the brush, active: the UI-thread brush activates its server
    /// object when it is created.
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionMaskBrush> {
        let brush = Rc::new_cyclic(|this: &Weak<ServerCompositionMaskBrush>| ServerCompositionMaskBrush {
            base: ServerRenderResource::new(
                compositor,
                this.clone() as Weak<dyn IAnimatedServerObject>,
                this.clone() as Weak<dyn IServerRenderResource>,
            ),
            props: ServerCompositionMaskBrushProps::new(),
            painting: Cell::new(false),
        });
        brush.base.object().activate();
        brush
    }

    /// The properties of the brush.
    pub fn props(&self) -> &ServerCompositionMaskBrushProps {
        &self.props
    }

    /// The properties of the generated `ServerCompositionBrush`.
    pub fn brush_props(&self) -> &ServerCompositionBrushProps {
        self.props.base()
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    /// Paints the source over `rect` through the mask: a brush of the drawing
    /// context (a colour or gradient) is pushed as the opacity mask; any other
    /// mask is painted over the source in a layer with the destination-in
    /// blending of bitmaps. Without a mask the source is painted as it is.
    pub fn paint(&self, canvas: &mut dyn IDrawingContextImpl, rect: Rect) {
        let Some(source) = self.props.source() else { return };
        let opacity = self.brush_props().opacity();
        if opacity <= 0.0 || is_empty(&rect) || self.painting.replace(true) {
            return;
        }
        if opacity != 1.0 {
            canvas.push_opacity(opacity, Some(rect));
        }
        match self.props.mask() {
            None => paint_composition_brush(canvas, &source, rect),
            Some(mask) => match mask.clone().as_brush() {
                Some(mask_brush) => {
                    canvas.push_opacity_mask(&*mask_brush, rect);
                    paint_composition_brush(canvas, &source, rect);
                    canvas.pop_opacity_mask();
                }
                None => {
                    // Deviation (winui-composition.md): the drawing contexts
                    // blend bitmaps only, so only the bitmaps of the mask mask.
                    canvas.push_layer(rect);
                    paint_composition_brush(canvas, &source, rect);
                    canvas.push_render_options(RenderOptions {
                        bitmap_blending_mode: BitmapBlendingMode::DestinationIn,
                        ..RenderOptions::default()
                    });
                    paint_composition_brush(canvas, &mask, rect);
                    canvas.pop_render_options();
                    canvas.pop_layer();
                }
            },
        }
        if opacity != 1.0 {
            canvas.pop_opacity();
        }
        self.painting.set(false);
    }
}

impl_server_render_resource!(
    ServerCompositionMaskBrush,
    base,
    composition_property: ServerCompositionMaskBrushProps::get_composition_property
);

impl ServerCompositionBrushHooks for ServerCompositionMaskBrush {}

impl ServerCompositionMaskBrushHooks for ServerCompositionMaskBrush {}

impl IServerObject for ServerCompositionMaskBrush {
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
