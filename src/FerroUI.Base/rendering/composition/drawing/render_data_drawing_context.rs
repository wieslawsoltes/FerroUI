use super::server_resource_helper_extensions::{
    brush_get_server, brush_render_resource, geometry_get_server, geometry_render_resource, is_immutable_brush,
    is_immutable_pen, pen_get_client, pen_get_server, pen_render_resource,
};
use super::{CompositionRenderData, ICompositionRenderResource, ImmediateRenderDataSceneBrushContent, RenderDataStream};
use crate::media::{
    BoxShadows, EffectExtensions, Geometry, GlyphRun, IBrush, IDrawingContextCore, IEffect, IPen, ITileBrush,
    RenderOptions, TextOptions,
};
use crate::platform::{IBitmapImpl, IGeometryImpl};
use crate::rendering::composition::Compositor;
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::{Matrix, Point, Rect, Ref, RoundedRect};
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone, Copy, Default)]
struct PushEntry {
    emitted: bool,
    position_before: usize,
    position_after: usize,
    depth_before: i32,
}

/// A resource whose reference on the compositor is held by the render data
/// being recorded.
pub(crate) enum RecordedResource {
    Brush(Rc<dyn IBrush>),
    Pen(Rc<dyn IPen>),
    Geometry(Ref<Geometry>),
}

impl RecordedResource {
    pub(crate) fn with_resource<R>(&self, f: impl FnOnce(&dyn ICompositionRenderResource) -> R) -> Option<R> {
        match self {
            RecordedResource::Brush(brush) => brush_render_resource(&**brush).map(f),
            RecordedResource::Pen(pen) => pen_render_resource(&**pen).map(f),
            RecordedResource::Geometry(geometry) => geometry_render_resource(geometry).map(f),
        }
    }
}

/// The drawing context core that records what is drawn into render data
/// instead of drawing it.
///
/// With a compositor the result is a [`CompositionRenderData`] registered
/// for serialization to the server; without one the recorded stream refers
/// to the drawn objects directly and is replayed on the same thread.
pub struct RenderDataDrawingContext {
    compositor: Option<Rc<Compositor>>,
    stream: Option<RenderDataStream>,
    resources: Vec<RecordedResource>,
    resources_hash_set: HashSet<usize>,
    push_stack: Vec<PushEntry>,
}

impl RenderDataDrawingContext {
    pub fn new(compositor: Option<Rc<Compositor>>) -> Self {
        Self { compositor, stream: None, resources: Vec::new(), resources_hash_set: HashSet::new(), push_stack: Vec::new() }
    }

    fn stream(&mut self) -> &mut RenderDataStream {
        self.stream.get_or_insert_with(RenderDataStream::new)
    }

    fn add_brush_resource(&mut self, brush: Option<&Rc<dyn IBrush>>) {
        let (Some(compositor), Some(brush)) = (&self.compositor, brush) else { return };
        // Immutable brushes and composition brushes are referenced directly.
        if is_immutable_brush(&**brush) || brush.as_composition_brush().is_some() {
            return;
        }
        let Some(resource) = brush_render_resource(&**brush) else { return };
        if !self.resources_hash_set.insert(Rc::as_ptr(brush) as *const () as usize) {
            return;
        }
        resource.add_ref_on_compositor(compositor);
        self.resources.push(RecordedResource::Brush(brush.clone()));
    }

    fn add_pen_resource(&mut self, pen: Option<&Rc<dyn IPen>>) {
        let (Some(compositor), Some(pen)) = (&self.compositor, pen) else { return };
        if is_immutable_pen(&**pen) {
            return;
        }
        let Some(resource) = pen_render_resource(&**pen) else { return };
        if !self.resources_hash_set.insert(Rc::as_ptr(pen) as *const () as usize) {
            return;
        }
        resource.add_ref_on_compositor(compositor);
        self.resources.push(RecordedResource::Pen(pen.clone()));
    }

    fn add_geometry_resource(&mut self, geometry: &Ref<Geometry>) {
        let Some(compositor) = &self.compositor else { return };
        let Some(resource) = geometry_render_resource(geometry) else { return };
        if !self.resources_hash_set.insert(&**geometry as *const Geometry as usize) {
            return;
        }
        resource.add_ref_on_compositor(compositor);
        self.resources.push(RecordedResource::Geometry(geometry.clone()));
    }

    fn pushed_scope(&mut self, position_before: usize) {
        let stream = self.stream();
        let entry = PushEntry {
            emitted: true,
            position_before,
            position_after: stream.opcode_length(),
            depth_before: stream.depth() - 1,
        };
        self.push_stack.push(entry);
    }

    fn pushed_no_op_scope(&mut self) {
        self.push_stack.push(PushEntry::default());
    }

    fn pop_core(&mut self) {
        let entry = self.push_stack.pop().expect("a pop without a matching push");
        if !entry.emitted {
            return;
        }
        let stream = self.stream();
        // Nothing was drawn inside the scope: drop the push instead of
        // closing it.
        if stream.opcode_length() == entry.position_after {
            stream.rewind(entry.position_before, entry.depth_before);
        } else {
            stream.pop();
        }
    }

    fn flush_stack(&mut self) {
        while !self.push_stack.is_empty() {
            self.pop_core();
        }
    }

    /// Finishes the recording and returns the render data, registered for
    /// serialization on the compositor; `None` if nothing was drawn. The
    /// context is ready to record again.
    ///
    /// Panics if the context has no compositor.
    pub fn get_render_results(&mut self) -> Option<Rc<CompositionRenderData>> {
        let Some(compositor) = self.compositor.clone() else {
            panic!("the drawing context records without a compositor");
        };
        self.flush_stack();

        let resources = std::mem::take(&mut self.resources);
        self.resources_hash_set.clear();
        let stream = self.stream.take();
        match stream {
            Some(stream) if !resources.is_empty() || stream.opcode_length() > 0 => {
                let render_data = CompositionRenderData::new(&compositor, stream, resources);
                compositor.register_for_serialization(render_data.clone());
                Some(render_data)
            }
            Some(mut stream) => {
                stream.dispose();
                None
            }
            None => None,
        }
    }

    /// Finishes a compositor-less recording and returns what was recorded
    /// as the content of the scene brush `brush`; `None` if nothing was
    /// drawn. `rect` is the bounds of the content; without it the bounds of
    /// what was drawn are used.
    pub fn get_immediate_scene_brush_content(
        &mut self,
        brush: Rc<dyn ITileBrush>,
        rect: Option<Rect>,
        use_scalable_rasterization: bool,
    ) -> Option<Rc<ImmediateRenderDataSceneBrushContent>> {
        debug_assert!(self.compositor.is_none());
        self.flush_stack();

        match self.stream.take() {
            Some(stream) if stream.opcode_length() > 0 => Some(Rc::new(ImmediateRenderDataSceneBrushContent::new(
                brush,
                stream,
                rect,
                use_scalable_rasterization,
            ))),
            Some(mut stream) => {
                stream.dispose();
                None
            }
            None => None,
        }
    }

    /// Discards what has been recorded so far.
    pub fn reset(&mut self) {
        if let Some(mut stream) = self.stream.take() {
            stream.dispose_resources();
            stream.dispose();
        }
        if let Some(compositor) = &self.compositor {
            for resource in self.resources.drain(..) {
                resource.with_resource(|r| r.release_on_compositor(compositor));
            }
        }
        self.resources.clear();
        self.push_stack.clear();
        self.resources_hash_set.clear();
    }
}

impl IDrawingContextCore for RenderDataDrawingContext {
    fn draw_line_core(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        self.add_pen_resource(Some(pen));
        let compositor = self.compositor.clone();
        let server_pen = pen_get_server(Some(pen), compositor.as_deref());
        let client_pen = pen_get_client(Some(pen), compositor.as_deref());
        self.stream().draw_line(server_pen, client_pen, p1, p2);
    }

    fn draw_geometry_impl_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        geometry: &Arc<dyn IGeometryImpl>,
    ) {
        if brush.is_none() && pen.is_none() {
            return;
        }
        self.add_brush_resource(brush);
        self.add_pen_resource(pen);
        let compositor = self.compositor.clone();
        let server_brush = brush_get_server(brush, compositor.as_deref());
        let server_pen = pen_get_server(pen, compositor.as_deref());
        let client_pen = pen_get_client(pen, compositor.as_deref());
        self.stream().draw_geometry(
            server_brush,
            server_pen,
            client_pen,
            Some(super::RenderDataResource::GeometryImpl(geometry.clone())),
        );
    }

    fn draw_geometry_core(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, geometry: &Ref<Geometry>) {
        if brush.is_none() && pen.is_none() {
            return;
        }
        self.add_brush_resource(brush);
        self.add_pen_resource(pen);
        self.add_geometry_resource(geometry);
        let compositor = self.compositor.clone();
        let server_brush = brush_get_server(brush, compositor.as_deref());
        let server_pen = pen_get_server(pen, compositor.as_deref());
        let client_pen = pen_get_client(pen, compositor.as_deref());
        let geometry = geometry_get_server(geometry, compositor.as_deref());
        self.stream().draw_geometry(server_brush, server_pen, client_pen, geometry);
    }

    fn draw_rectangle_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        pen: Option<&Rc<dyn IPen>>,
        rrect: RoundedRect,
        box_shadows: &BoxShadows,
    ) {
        if rrect.is_empty() {
            return;
        }
        if brush.is_none() && pen.is_none() && *box_shadows == BoxShadows::default() {
            return;
        }
        self.add_brush_resource(brush);
        self.add_pen_resource(pen);
        let compositor = self.compositor.clone();
        let server_brush = brush_get_server(brush, compositor.as_deref());
        let server_pen = pen_get_server(pen, compositor.as_deref());
        let client_pen = pen_get_client(pen, compositor.as_deref());
        self.stream().draw_rectangle(server_brush, server_pen, client_pen, rrect, box_shadows);
    }

    fn draw_ellipse_core(&mut self, brush: Option<&Rc<dyn IBrush>>, pen: Option<&Rc<dyn IPen>>, rect: Rect) {
        if rect.is_empty() {
            return;
        }
        if brush.is_none() && pen.is_none() {
            return;
        }
        self.add_brush_resource(brush);
        self.add_pen_resource(pen);
        let compositor = self.compositor.clone();
        let server_brush = brush_get_server(brush, compositor.as_deref());
        let server_pen = pen_get_server(pen, compositor.as_deref());
        let client_pen = pen_get_client(pen, compositor.as_deref());
        self.stream().draw_ellipse(server_brush, server_pen, client_pen, rect);
    }

    fn draw_bitmap(&mut self, source: &std::sync::Arc<crate::platform::SharedBitmapImpl>, opacity: f64, source_rect: Rect, dest_rect: Rect) {
        if source_rect.is_empty() || dest_rect.is_empty() {
            return;
        }
        self.stream().draw_bitmap(Some(source.clone()), opacity, source_rect, dest_rect);
    }

    fn custom(&mut self, custom: &Rc<dyn ICustomDrawOperation>) {
        self.stream().draw_custom(Some(custom.clone()));
    }

    fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        if foreground.is_none() {
            return;
        }
        self.add_brush_resource(foreground);
        let compositor = self.compositor.clone();
        let server_brush = brush_get_server(foreground, compositor.as_deref());
        self.stream().draw_glyph_run(server_brush, Some(glyph_run.platform_impl()));
    }

    fn push_clip_core(&mut self, rect: Rect) {
        let before = self.stream().opcode_length();
        self.stream().push_clip(RoundedRect::from_rect(rect));
        self.pushed_scope(before);
    }

    fn push_rounded_clip_core(&mut self, rect: RoundedRect) {
        let before = self.stream().opcode_length();
        self.stream().push_clip(rect);
        self.pushed_scope(before);
    }

    fn push_geometry_clip_core(&mut self, clip: &Ref<Geometry>) {
        self.add_geometry_resource(clip);
        let before = self.stream().opcode_length();
        let compositor = self.compositor.clone();
        let geometry = geometry_get_server(clip, compositor.as_deref());
        self.stream().push_geometry_clip(geometry);
        self.pushed_scope(before);
    }

    fn push_opacity_core(&mut self, opacity: f64) {
        if opacity == 1.0 {
            self.pushed_no_op_scope();
            return;
        }
        let before = self.stream().opcode_length();
        self.stream().push_opacity(opacity);
        self.pushed_scope(before);
    }

    fn push_opacity_mask_core(&mut self, mask: &Rc<dyn IBrush>, bounds: Rect) {
        self.add_brush_resource(Some(mask));
        let before = self.stream().opcode_length();
        let compositor = self.compositor.clone();
        let server_brush = brush_get_server(Some(mask), compositor.as_deref());
        self.stream().push_opacity_mask(server_brush, bounds);
        self.pushed_scope(before);
    }

    fn push_transform_core(&mut self, matrix: Matrix) {
        if matrix.is_identity() {
            self.pushed_no_op_scope();
            return;
        }
        let before = self.stream().opcode_length();
        self.stream().push_transform(matrix);
        self.pushed_scope(before);
    }

    fn push_render_options_core(&mut self, render_options: RenderOptions) {
        let before = self.stream().opcode_length();
        self.stream().push_render_options(render_options);
        self.pushed_scope(before);
    }

    fn push_text_options_core(&mut self, text_options: TextOptions) {
        let before = self.stream().opcode_length();
        self.stream().push_text_options(text_options);
        self.pushed_scope(before);
    }

    fn push_effect_core(&mut self, effect: &Rc<dyn IEffect>, bounds: Rect) {
        let before = self.stream().opcode_length();
        let padding = EffectExtensions::get_effect_output_padding(Some(&**effect));
        let effect = EffectExtensions::to_immutable(effect) as Rc<dyn IEffect>;
        self.stream().push_effect(Some(effect), bounds.inflate_thickness(padding));
        self.pushed_scope(before);
    }

    fn pop_clip_core(&mut self) {
        self.pop_core();
    }

    fn pop_geometry_clip_core(&mut self) {
        self.pop_core();
    }

    fn pop_opacity_core(&mut self) {
        self.pop_core();
    }

    fn pop_opacity_mask_core(&mut self) {
        self.pop_core();
    }

    fn pop_transform_core(&mut self) {
        self.pop_core();
    }

    fn pop_render_options_core(&mut self) {
        self.pop_core();
    }

    fn pop_text_options_core(&mut self) {
        self.pop_core();
    }

    fn pop_effect_core(&mut self) {
        self.pop_core();
    }

    fn dispose_core(&mut self) {
        self.reset();
    }
}
