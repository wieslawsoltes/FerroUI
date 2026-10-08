//! The recorded form of what a visual draws: opcodes followed by their
//! payloads in a byte buffer ([`RenderDataWriter`]), with the resources
//! they refer to in a table.

use super::{
    DrawBitmapPayload, DrawCustomPayload, DrawEllipsePayload, DrawGeometryPayload, DrawGlyphRunPayload,
    DrawLinePayload, DrawRectanglePayload, IRenderDataVisitor, PushClipPayload,
    PushEffectPayload, PushGeometryClipPayload, PushOpacityMaskPayload, PushOpacityPayload,
    PushRenderOptionsPayload, PushTextOptionsPayload, PushTransformPayload, RenderDataOpcode, RenderDataReader,
    RenderDataResource, RenderDataResources, RenderDataWriter,
};
use crate::media::{BoxShadow, BoxShadows, RenderOptions, TextOptions};
use crate::platform::IGlyphRunImpl;
use crate::rendering::composition::server::{IServerObject, ServerObjectId};
use crate::rendering::composition::transport::{BatchObject, BatchStreamReader, BatchStreamWriter};
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::{Matrix, Point, Rect, RoundedRect};
use std::rc::Rc;
use std::sync::Arc;

const TAG_NOT_SENT: u8 = 0;
const TAG_BRUSH: u8 = 1;
const TAG_PEN: u8 = 2;
const TAG_GEOMETRY_IMPL: u8 = 3;
// Tag 4 was a geometry wrapper sent by value, which does not happen.
const TAG_GLYPH_RUN: u8 = 5;
const TAG_BITMAP: u8 = 6;
const TAG_CUSTOM: u8 = 7;
const TAG_EFFECT: u8 = 8;
const TAG_SERVER_BRUSH: u8 = 9;
const TAG_SERVER_PEN: u8 = 10;
const TAG_SERVER_GEOMETRY: u8 = 11;

/// A recorded sequence of drawing operations with its resource table.
#[derive(Default)]
pub struct RenderDataStream {
    writer: RenderDataWriter,
    pub(super) resources: RenderDataResources,
    depth: i32,
    max_depth: i32,
}

impl RenderDataStream {
    pub fn new() -> Self {
        Self::default()
    }

    /// The recorded operations.
    pub fn opcodes(&self) -> &[u8] {
        self.writer.written()
    }

    /// The number of bytes recorded: the position a later
    /// [`rewind`](Self::rewind) can return to.
    pub fn opcode_length(&self) -> usize {
        self.writer.length()
    }

    /// The current scope depth.
    pub fn depth(&self) -> i32 {
        self.depth
    }

    /// The deepest scope nesting recorded.
    pub fn max_depth(&self) -> i32 {
        self.max_depth
    }

    pub fn resource_count(&self) -> usize {
        self.resources.count()
    }

    pub fn get_resource(&self, handle: i32) -> Option<&RenderDataResource> {
        self.resources.get(handle)
    }

    /// Drops everything recorded after position `length` and restores the
    /// scope depth.
    pub fn rewind(&mut self, length: usize, depth: i32) {
        self.writer.rewind(length);
        self.depth = depth;
    }

    fn enter_scope(&mut self) {
        self.depth += 1;
        if self.depth > self.max_depth {
            self.max_depth = self.depth;
        }
    }

    /// Releases the resources the stream holds a reference to on behalf of
    /// their owners.
    pub fn dispose_resources(&self) {
        for resource in self.resources.iter() {
            if let RenderDataResource::CustomDrawOperation(operation) = resource {
                operation.dispose();
            }
        }
    }

    pub fn draw_line(
        &mut self,
        server_pen: Option<RenderDataResource>,
        client_pen: Option<RenderDataResource>,
        p1: Point,
        p2: Point,
    ) {
        let payload = DrawLinePayload {
            server_pen: self.resources.intern(server_pen),
            client_pen: self.resources.intern(client_pen),
            p1,
            p2,
        };
        self.writer.write_payload(payload);
    }

    pub fn draw_rectangle(
        &mut self,
        server_brush: Option<RenderDataResource>,
        server_pen: Option<RenderDataResource>,
        client_pen: Option<RenderDataResource>,
        rect: RoundedRect,
        box_shadows: &BoxShadows,
    ) {
        let payload = DrawRectanglePayload {
            server_brush: self.resources.intern(server_brush),
            server_pen: self.resources.intern(server_pen),
            client_pen: self.resources.intern(client_pen),
            rect,
            box_shadow_count: box_shadows.count() as i32,
        };
        self.writer.write_payload(payload);
        for shadow in box_shadows.iter() {
            self.writer.write(shadow);
        }
    }

    pub fn draw_ellipse(
        &mut self,
        server_brush: Option<RenderDataResource>,
        server_pen: Option<RenderDataResource>,
        client_pen: Option<RenderDataResource>,
        rect: Rect,
    ) {
        let payload = DrawEllipsePayload {
            server_brush: self.resources.intern(server_brush),
            server_pen: self.resources.intern(server_pen),
            client_pen: self.resources.intern(client_pen),
            rect,
        };
        self.writer.write_payload(payload);
    }

    pub fn draw_geometry(
        &mut self,
        server_brush: Option<RenderDataResource>,
        server_pen: Option<RenderDataResource>,
        client_pen: Option<RenderDataResource>,
        geometry: Option<RenderDataResource>,
    ) {
        let payload = DrawGeometryPayload {
            server_brush: self.resources.intern(server_brush),
            server_pen: self.resources.intern(server_pen),
            client_pen: self.resources.intern(client_pen),
            geometry: self.resources.intern(geometry),
        };
        self.writer.write_payload(payload);
    }

    /// Records a glyph run. The table entry is a new counted reference to
    /// it (upstream's caller passes `PlatformImpl.Clone()`).
    pub fn draw_glyph_run(
        &mut self,
        server_brush: Option<RenderDataResource>,
        glyph_run: Option<std::sync::Arc<dyn IGlyphRunImpl>>,
    ) {
        let glyph_run = glyph_run.map(|glyph_run| RenderDataResource::GlyphRun(Arc::new(glyph_run)));
        let payload = DrawGlyphRunPayload {
            server_brush: self.resources.intern(server_brush),
            glyph_run: self.resources.intern(glyph_run),
        };
        self.writer.write_payload(payload);
    }

    /// Records a bitmap. The table entry is a new counted reference to it
    /// (upstream's caller passes `source.Clone()`).
    pub fn draw_bitmap(
        &mut self,
        bitmap: Option<std::sync::Arc<crate::platform::SharedBitmapImpl>>,
        opacity: f64,
        source_rect: Rect,
        dest_rect: Rect,
    ) {
        let bitmap = bitmap.map(|bitmap| RenderDataResource::Bitmap(Arc::new(bitmap)));
        let payload = DrawBitmapPayload {
            bitmap: self.resources.intern(bitmap),
            opacity,
            source_rect,
            dest_rect,
        };
        self.writer.write_payload(payload);
    }

    pub fn draw_custom(&mut self, operation: Option<std::sync::Arc<dyn ICustomDrawOperation>>) {
        let payload = DrawCustomPayload {
            operation: self.resources.intern(operation.map(RenderDataResource::CustomDrawOperation)),
        };
        self.writer.write_payload(payload);
    }

    pub fn push_clip(&mut self, clip: RoundedRect) {
        self.writer.write_payload(PushClipPayload { clip });
        self.enter_scope();
    }

    pub fn push_geometry_clip(&mut self, geometry: Option<RenderDataResource>) {
        let payload = PushGeometryClipPayload { geometry: self.resources.intern(geometry) };
        self.writer.write_payload(payload);
        self.enter_scope();
    }

    pub fn push_opacity(&mut self, opacity: f64) {
        self.writer.write_payload(PushOpacityPayload { opacity });
        self.enter_scope();
    }

    pub fn push_opacity_mask(&mut self, server_brush: Option<RenderDataResource>, bounds: Rect) {
        let payload = PushOpacityMaskPayload { brush: self.resources.intern(server_brush), bounds };
        self.writer.write_payload(payload);
        self.enter_scope();
    }

    pub fn push_transform(&mut self, matrix: Matrix) {
        self.writer.write_payload(PushTransformPayload { matrix });
        self.enter_scope();
    }

    pub fn push_render_options(&mut self, options: RenderOptions) {
        self.writer.write_payload(PushRenderOptionsPayload { options });
        self.enter_scope();
    }

    pub fn push_text_options(&mut self, text_options: TextOptions) {
        self.writer.write_payload(PushTextOptionsPayload { options: text_options });
        self.enter_scope();
    }

    pub fn push_effect(&mut self, effect: Option<Arc<dyn crate::media::IImmutableEffect>>, bounds: Rect) {
        let payload =
            PushEffectPayload { effect: self.resources.intern(effect.map(RenderDataResource::Effect)), bounds };
        self.writer.write_payload(payload);
        self.enter_scope();
    }

    pub fn pop(&mut self) {
        self.writer.write_opcode(RenderDataOpcode::Pop);
        self.depth -= 1;
    }

    /// Writes the stream to a batch. Resources with a server-side
    /// counterpart are sent as the counterpart's id; client-only resources
    /// keep their slot but are not sent.
    pub fn serialize_to(&self, writer: &mut BatchStreamWriter<'_>) {
        writer.write(self.max_depth);
        writer.write(self.resources.count() as i32);
        for resource in self.resources.iter() {
            match resource {
                RenderDataResource::ClientPen(_) | RenderDataResource::NotSent => writer.write(TAG_NOT_SENT),
                // A brush or a pen sent by value is an immutable one: what
                // crosses to the render thread is its shared form.
                RenderDataResource::Brush(v) => {
                    writer.write(TAG_BRUSH);
                    let shared = crate::media::shared_brush_of(&**v)
                        .unwrap_or_else(|| panic!("The brush is not compatible with composition"));
                    writer.write_object(BatchObject::value(shared));
                }
                RenderDataResource::Pen(v) => {
                    writer.write(TAG_PEN);
                    let shared = crate::media::shared_pen_of(&**v)
                        .unwrap_or_else(|| panic!("The pen is not compatible with composition"));
                    writer.write_object(BatchObject::value(shared));
                }
                RenderDataResource::GeometryImpl(v) => {
                    writer.write(TAG_GEOMETRY_IMPL);
                    writer.write_object(BatchObject::value(v.clone()));
                }
                // With a compositor a geometry object is sent as the id of
                // its server object; the wrapper is what the server resolves
                // the id to, and what render data keeps on the UI thread.
                RenderDataResource::Geometry(_) => panic!("a geometry of render data is not sent by value"),
                RenderDataResource::GlyphRun(v) => {
                    writer.write(TAG_GLYPH_RUN);
                    writer.write_object(BatchObject::value(v.clone()));
                }
                RenderDataResource::Bitmap(v) => {
                    writer.write(TAG_BITMAP);
                    writer.write_object(BatchObject::value(v.clone()));
                }
                RenderDataResource::CustomDrawOperation(v) => {
                    writer.write(TAG_CUSTOM);
                    writer.write_object(BatchObject::value(v.clone()));
                }
                RenderDataResource::Effect(v) => {
                    writer.write(TAG_EFFECT);
                    writer.write_object(BatchObject::value(v.clone()));
                }
                RenderDataResource::ServerBrush { server, .. } => {
                    writer.write(TAG_SERVER_BRUSH);
                    writer.write_server_object(Some(*server));
                }
                RenderDataResource::ServerPen { server, .. } => {
                    writer.write(TAG_SERVER_PEN);
                    writer.write_server_object(Some(*server));
                }
                RenderDataResource::ServerGeometry { server, .. } => {
                    writer.write(TAG_SERVER_GEOMETRY);
                    writer.write_server_object(Some(*server));
                }
            }
        }
        let opcodes = self.writer.written();
        writer.write(opcodes.len() as i32);
        writer.write_bytes(opcodes);
    }

    /// Reads a stream from a batch on the server. `resolve` maps the id of
    /// a server object to the object.
    pub fn deserialize_from(
        &mut self,
        reader: &mut BatchStreamReader<'_>,
        resolve: &dyn Fn(ServerObjectId) -> Option<Rc<dyn IServerObject>>,
    ) {
        fn value<T: 'static>(reader: &mut BatchStreamReader<'_>) -> T {
            match reader.read_value::<T>() {
                Some(value) => value,
                None => panic!("a render data resource is missing from the batch"),
            }
        }
        fn server_object(
            reader: &mut BatchStreamReader<'_>,
            resolve: &dyn Fn(ServerObjectId) -> Option<Rc<dyn IServerObject>>,
        ) -> Rc<dyn IServerObject> {
            match reader.read_server_object().and_then(resolve) {
                Some(object) => object,
                None => panic!("render data refers to a server object that does not exist"),
            }
        }

        self.max_depth = reader.read::<i32>();
        let resource_count = reader.read::<i32>();
        for _ in 0..resource_count {
            let resource = match reader.read::<u8>() {
                TAG_NOT_SENT => RenderDataResource::NotSent,
                TAG_BRUSH => RenderDataResource::Brush(Rc::new(value::<crate::media::SharedBrush>(reader))),
                TAG_PEN => RenderDataResource::Pen(Rc::new(value::<crate::media::SharedPen>(reader))),
                TAG_GEOMETRY_IMPL => RenderDataResource::GeometryImpl(value(reader)),
                TAG_GLYPH_RUN => RenderDataResource::GlyphRun(value(reader)),
                TAG_BITMAP => RenderDataResource::Bitmap(value(reader)),
                TAG_CUSTOM => RenderDataResource::CustomDrawOperation(value(reader)),
                TAG_EFFECT => RenderDataResource::Effect(value(reader)),
                TAG_SERVER_BRUSH => match server_object(reader, resolve).as_brush() {
                    Some(brush) => RenderDataResource::Brush(brush),
                    None => panic!("the server object referenced by render data is not a brush"),
                },
                TAG_SERVER_PEN => match server_object(reader, resolve).as_pen() {
                    Some(pen) => RenderDataResource::Pen(pen),
                    None => panic!("the server object referenced by render data is not a pen"),
                },
                TAG_SERVER_GEOMETRY => match server_object(reader, resolve).as_render_data_geometry() {
                    Some(geometry) => RenderDataResource::Geometry(geometry),
                    None => panic!("the server object referenced by render data is not a geometry"),
                },
                _ => panic!("unknown render data resource tag"),
            };
            self.resources.append_deserialized(resource);
        }
        let byte_count = reader.read::<i32>();
        if byte_count > 0 {
            reader.read_bytes(self.writer.reserve(byte_count as usize));
        }
    }

    /// Walks the recorded operations.
    pub fn visit<V: IRenderDataVisitor>(&self, visitor: &mut V) {
        let mut scopes: Vec<V::Scope> = Vec::with_capacity(self.max_depth.max(0) as usize);
        self.visit_with_scopes(visitor, &mut scopes);
    }

    /// Walks the recorded operations using a caller-provided scope stack,
    /// which lets a caller that visits often reuse the allocation (upstream
    /// keeps the scopes on the stack).
    pub fn visit_with_scopes<V: IRenderDataVisitor>(&self, visitor: &mut V, scopes: &mut Vec<V::Scope>) {
        let resources = &self.resources;
        let mut reader = RenderDataReader::new(self.writer.written());
        while !visitor.stop_visiting() && !reader.is_at_end() {
            match reader.peek::<RenderDataOpcode>() {
                RenderDataOpcode::DrawLine => {
                    let p = reader.read_payload::<DrawLinePayload>();
                    visitor.on_draw_line(resources.pen(p.server_pen), resources.pen(p.client_pen), p.p1, p.p2);
                }
                RenderDataOpcode::DrawRectangle => {
                    let p = reader.read_payload::<DrawRectanglePayload>();
                    let shadows = Self::read_box_shadows(&mut reader, p.box_shadow_count);
                    visitor.on_draw_rectangle(
                        resources.brush(p.server_brush),
                        resources.pen(p.server_pen),
                        resources.pen(p.client_pen),
                        p.rect,
                        &shadows,
                    );
                }
                RenderDataOpcode::DrawEllipse => {
                    let p = reader.read_payload::<DrawEllipsePayload>();
                    visitor.on_draw_ellipse(
                        resources.brush(p.server_brush),
                        resources.pen(p.server_pen),
                        resources.pen(p.client_pen),
                        p.rect,
                    );
                }
                RenderDataOpcode::DrawGeometry => {
                    let p = reader.read_payload::<DrawGeometryPayload>();
                    visitor.on_draw_geometry(
                        resources.brush(p.server_brush),
                        resources.pen(p.server_pen),
                        resources.pen(p.client_pen),
                        resources.geometry_impl(p.geometry).as_ref(),
                    );
                }
                RenderDataOpcode::DrawGlyphRun => {
                    let p = reader.read_payload::<DrawGlyphRunPayload>();
                    visitor.on_draw_glyph_run(resources.brush(p.server_brush), resources.glyph_run(p.glyph_run));
                }
                RenderDataOpcode::DrawBitmap => {
                    let p = reader.read_payload::<DrawBitmapPayload>();
                    visitor.on_draw_bitmap(resources.bitmap(p.bitmap), p.opacity, p.source_rect, p.dest_rect);
                }
                RenderDataOpcode::DrawCustom => {
                    let p = reader.read_payload::<DrawCustomPayload>();
                    visitor.on_draw_custom(resources.custom_draw_operation(p.operation));
                }
                RenderDataOpcode::PushClip => {
                    let p = reader.read_payload::<PushClipPayload>();
                    scopes.push(visitor.on_push_clip(p.clip));
                }
                RenderDataOpcode::PushGeometryClip => {
                    let p = reader.read_payload::<PushGeometryClipPayload>();
                    scopes.push(visitor.on_push_geometry_clip(resources.geometry_impl(p.geometry).as_ref()));
                }
                RenderDataOpcode::PushOpacity => {
                    let p = reader.read_payload::<PushOpacityPayload>();
                    scopes.push(visitor.on_push_opacity(p.opacity));
                }
                RenderDataOpcode::PushOpacityMask => {
                    let p = reader.read_payload::<PushOpacityMaskPayload>();
                    scopes.push(visitor.on_push_opacity_mask(resources.brush(p.brush), p.bounds));
                }
                RenderDataOpcode::PushTransform => {
                    let p = reader.read_payload::<PushTransformPayload>();
                    scopes.push(visitor.on_push_transform(p.matrix));
                }
                RenderDataOpcode::PushRenderOptions => {
                    let p = reader.read_payload::<PushRenderOptionsPayload>();
                    scopes.push(visitor.on_push_render_options(p.options));
                }
                RenderDataOpcode::PushTextOptions => {
                    let p = reader.read_payload::<PushTextOptionsPayload>();
                    scopes.push(visitor.on_push_text_options(p.options));
                }
                RenderDataOpcode::PushEffect => {
                    let p = reader.read_payload::<PushEffectPayload>();
                    scopes.push(visitor.on_push_effect(resources.effect(p.effect), p.bounds));
                }
                RenderDataOpcode::Pop => {
                    reader.read::<RenderDataOpcode>();
                    let scope = scopes.pop().expect("a pop without a matching push in render data");
                    visitor.on_pop(scope);
                }
                RenderDataOpcode::Invalid => panic!("render data holds an invalid opcode"),
            }
        }
        scopes.clear();
    }

    fn read_box_shadows(reader: &mut RenderDataReader<'_>, count: i32) -> BoxShadows {
        if count == 0 {
            return BoxShadows::default();
        }

        let first = reader.read::<BoxShadow>();
        if count == 1 {
            return BoxShadows::new(first);
        }

        let rest: Vec<BoxShadow> = (1..count).map(|_| reader.read::<BoxShadow>()).collect();
        BoxShadows::with_rest(first, &rest)
    }

    /// Releases the stream.
    pub fn dispose(&mut self) {
        self.writer.dispose();
        self.resources.dispose();
    }
}
