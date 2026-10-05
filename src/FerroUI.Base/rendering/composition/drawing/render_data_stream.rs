//! The recorded form of what a visual draws.
//!
//! Upstream encodes the operations as a byte stream of opcodes followed by
//! blitted payload structs (`RenderDataOpcode`, `RenderDataPayloads`,
//! `RenderDataWriter`, `RenderDataReader`). Blitting structs needs `unsafe`
//! in Rust, so the stream is a vector of the typed [`RenderDataOp`] instead:
//! one element per opcode, the payload as the variant's fields, the box
//! shadows of a rectangle inline. Positions ("opcode length") are element
//! indices. Everything observable (scope elision by rewinding, depth
//! tracking, resource interning, visiting order) is the same.

use super::{IRenderDataGeometry, IRenderDataVisitor, RenderDataResource, RenderDataResources};
use crate::media::{BoxShadows, IBrush, IEffect, IPen, RenderOptions, TextOptions};
use crate::platform::{IBitmapImpl, IGlyphRunImpl};
use crate::rendering::composition::server::{IServerObject, ServerObjectId};
use crate::rendering::composition::transport::{BatchObject, BatchStreamReader, BatchStreamWriter};
use crate::rendering::scene_graph::ICustomDrawOperation;
use crate::{Matrix, Point, Rect, RoundedRect};
use std::rc::Rc;

/// One recorded drawing operation. Resources are referred to by handle
/// into the stream's resource table.
#[derive(Clone, Debug)]
pub enum RenderDataOp {
    DrawLine { server_pen: i32, client_pen: i32, p1: Point, p2: Point },
    DrawRectangle { server_brush: i32, server_pen: i32, client_pen: i32, rect: RoundedRect, box_shadows: BoxShadows },
    DrawEllipse { server_brush: i32, server_pen: i32, client_pen: i32, rect: Rect },
    DrawGeometry { server_brush: i32, server_pen: i32, client_pen: i32, geometry: i32 },
    DrawGlyphRun { server_brush: i32, glyph_run: i32 },
    DrawBitmap { bitmap: i32, opacity: f64, source_rect: Rect, dest_rect: Rect },
    DrawCustom { operation: i32 },
    PushClip { clip: RoundedRect },
    PushGeometryClip { geometry: i32 },
    PushOpacity { opacity: f64 },
    PushOpacityMask { brush: i32, bounds: Rect },
    PushTransform { matrix: Matrix },
    PushRenderOptions { options: RenderOptions },
    PushTextOptions { options: TextOptions },
    PushEffect { effect: i32, bounds: Rect },
    Pop,
}

const TAG_NOT_SENT: u8 = 0;
const TAG_BRUSH: u8 = 1;
const TAG_PEN: u8 = 2;
const TAG_GEOMETRY_IMPL: u8 = 3;
const TAG_GEOMETRY: u8 = 4;
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
    pub(super) ops: Vec<RenderDataOp>,
    pub(super) resources: RenderDataResources,
    depth: i32,
    max_depth: i32,
}

impl RenderDataStream {
    pub fn new() -> Self {
        Self::default()
    }

    /// The recorded operations.
    pub fn opcodes(&self) -> &[RenderDataOp] {
        &self.ops
    }

    /// The number of recorded operations: the position a later
    /// [`rewind`](Self::rewind) can return to.
    pub fn opcode_length(&self) -> usize {
        self.ops.len()
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
        self.ops.truncate(length);
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
        let op = RenderDataOp::DrawLine {
            server_pen: self.resources.intern(server_pen),
            client_pen: self.resources.intern(client_pen),
            p1,
            p2,
        };
        self.ops.push(op);
    }

    pub fn draw_rectangle(
        &mut self,
        server_brush: Option<RenderDataResource>,
        server_pen: Option<RenderDataResource>,
        client_pen: Option<RenderDataResource>,
        rect: RoundedRect,
        box_shadows: &BoxShadows,
    ) {
        let op = RenderDataOp::DrawRectangle {
            server_brush: self.resources.intern(server_brush),
            server_pen: self.resources.intern(server_pen),
            client_pen: self.resources.intern(client_pen),
            rect,
            box_shadows: box_shadows.clone(),
        };
        self.ops.push(op);
    }

    pub fn draw_ellipse(
        &mut self,
        server_brush: Option<RenderDataResource>,
        server_pen: Option<RenderDataResource>,
        client_pen: Option<RenderDataResource>,
        rect: Rect,
    ) {
        let op = RenderDataOp::DrawEllipse {
            server_brush: self.resources.intern(server_brush),
            server_pen: self.resources.intern(server_pen),
            client_pen: self.resources.intern(client_pen),
            rect,
        };
        self.ops.push(op);
    }

    pub fn draw_geometry(
        &mut self,
        server_brush: Option<RenderDataResource>,
        server_pen: Option<RenderDataResource>,
        client_pen: Option<RenderDataResource>,
        geometry: Option<RenderDataResource>,
    ) {
        let op = RenderDataOp::DrawGeometry {
            server_brush: self.resources.intern(server_brush),
            server_pen: self.resources.intern(server_pen),
            client_pen: self.resources.intern(client_pen),
            geometry: self.resources.intern(geometry),
        };
        self.ops.push(op);
    }

    pub fn draw_glyph_run(&mut self, server_brush: Option<RenderDataResource>, glyph_run: Option<Rc<dyn IGlyphRunImpl>>) {
        let op = RenderDataOp::DrawGlyphRun {
            server_brush: self.resources.intern(server_brush),
            glyph_run: self.resources.intern(glyph_run.map(RenderDataResource::GlyphRun)),
        };
        self.ops.push(op);
    }

    pub fn draw_bitmap(&mut self, bitmap: Option<Rc<dyn IBitmapImpl>>, opacity: f64, source_rect: Rect, dest_rect: Rect) {
        let op = RenderDataOp::DrawBitmap {
            bitmap: self.resources.intern(bitmap.map(RenderDataResource::Bitmap)),
            opacity,
            source_rect,
            dest_rect,
        };
        self.ops.push(op);
    }

    pub fn draw_custom(&mut self, operation: Option<Rc<dyn ICustomDrawOperation>>) {
        let op = RenderDataOp::DrawCustom {
            operation: self.resources.intern(operation.map(RenderDataResource::CustomDrawOperation)),
        };
        self.ops.push(op);
    }

    pub fn push_clip(&mut self, clip: RoundedRect) {
        self.ops.push(RenderDataOp::PushClip { clip });
        self.enter_scope();
    }

    pub fn push_geometry_clip(&mut self, geometry: Option<RenderDataResource>) {
        let op = RenderDataOp::PushGeometryClip { geometry: self.resources.intern(geometry) };
        self.ops.push(op);
        self.enter_scope();
    }

    pub fn push_opacity(&mut self, opacity: f64) {
        self.ops.push(RenderDataOp::PushOpacity { opacity });
        self.enter_scope();
    }

    pub fn push_opacity_mask(&mut self, server_brush: Option<RenderDataResource>, bounds: Rect) {
        let op = RenderDataOp::PushOpacityMask { brush: self.resources.intern(server_brush), bounds };
        self.ops.push(op);
        self.enter_scope();
    }

    pub fn push_transform(&mut self, matrix: Matrix) {
        self.ops.push(RenderDataOp::PushTransform { matrix });
        self.enter_scope();
    }

    pub fn push_render_options(&mut self, options: RenderOptions) {
        self.ops.push(RenderDataOp::PushRenderOptions { options });
        self.enter_scope();
    }

    pub fn push_text_options(&mut self, text_options: TextOptions) {
        self.ops.push(RenderDataOp::PushTextOptions { options: text_options });
        self.enter_scope();
    }

    pub fn push_effect(&mut self, effect: Option<Rc<dyn IEffect>>, bounds: Rect) {
        let op = RenderDataOp::PushEffect { effect: self.resources.intern(effect.map(RenderDataResource::Effect)), bounds };
        self.ops.push(op);
        self.enter_scope();
    }

    pub fn pop(&mut self) {
        self.ops.push(RenderDataOp::Pop);
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
                RenderDataResource::Brush(v) => {
                    writer.write(TAG_BRUSH);
                    writer.write_object(BatchObject::value(v.clone()));
                }
                RenderDataResource::Pen(v) => {
                    writer.write(TAG_PEN);
                    writer.write_object(BatchObject::value(v.clone()));
                }
                RenderDataResource::GeometryImpl(v) => {
                    writer.write(TAG_GEOMETRY_IMPL);
                    writer.write_object(BatchObject::value(v.clone()));
                }
                RenderDataResource::Geometry(v) => {
                    writer.write(TAG_GEOMETRY);
                    writer.write_object(BatchObject::value(v.clone()));
                }
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
        writer.write(self.ops.len() as i32);
        if !self.ops.is_empty() {
            writer.write_object(BatchObject::value(self.ops.clone()));
        }
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
                TAG_BRUSH => RenderDataResource::Brush(value::<Rc<dyn IBrush>>(reader)),
                TAG_PEN => RenderDataResource::Pen(value::<Rc<dyn IPen>>(reader)),
                TAG_GEOMETRY_IMPL => RenderDataResource::GeometryImpl(value(reader)),
                TAG_GEOMETRY => RenderDataResource::Geometry(value::<Rc<dyn IRenderDataGeometry>>(reader)),
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
        let op_count = reader.read::<i32>();
        if op_count > 0 {
            self.ops = value::<Vec<RenderDataOp>>(reader);
        }
    }

    /// Walks the recorded operations.
    pub fn visit<V: IRenderDataVisitor>(&self, visitor: &mut V) {
        let mut scopes: Vec<V::Scope> = Vec::with_capacity(self.max_depth.max(0) as usize);
        self.visit_with_scopes(visitor, &mut scopes);
    }

    /// Walks the recorded operations using a caller-provided scope stack,
    /// which lets a caller that visits often reuse the allocation.
    pub fn visit_with_scopes<V: IRenderDataVisitor>(&self, visitor: &mut V, scopes: &mut Vec<V::Scope>) {
        let resources = &self.resources;
        for op in &self.ops {
            if visitor.stop_visiting() {
                break;
            }
            match op {
                RenderDataOp::DrawLine { server_pen, client_pen, p1, p2 } => {
                    visitor.on_draw_line(resources.pen(*server_pen), resources.pen(*client_pen), *p1, *p2)
                }
                RenderDataOp::DrawRectangle { server_brush, server_pen, client_pen, rect, box_shadows } => visitor
                    .on_draw_rectangle(
                        resources.brush(*server_brush),
                        resources.pen(*server_pen),
                        resources.pen(*client_pen),
                        *rect,
                        box_shadows,
                    ),
                RenderDataOp::DrawEllipse { server_brush, server_pen, client_pen, rect } => visitor.on_draw_ellipse(
                    resources.brush(*server_brush),
                    resources.pen(*server_pen),
                    resources.pen(*client_pen),
                    *rect,
                ),
                RenderDataOp::DrawGeometry { server_brush, server_pen, client_pen, geometry } => visitor
                    .on_draw_geometry(
                        resources.brush(*server_brush),
                        resources.pen(*server_pen),
                        resources.pen(*client_pen),
                        resources.geometry_impl(*geometry).as_ref(),
                    ),
                RenderDataOp::DrawGlyphRun { server_brush, glyph_run } => {
                    visitor.on_draw_glyph_run(resources.brush(*server_brush), resources.glyph_run(*glyph_run))
                }
                RenderDataOp::DrawBitmap { bitmap, opacity, source_rect, dest_rect } => {
                    visitor.on_draw_bitmap(resources.bitmap(*bitmap), *opacity, *source_rect, *dest_rect)
                }
                RenderDataOp::DrawCustom { operation } => {
                    visitor.on_draw_custom(resources.custom_draw_operation(*operation))
                }
                RenderDataOp::PushClip { clip } => scopes.push(visitor.on_push_clip(*clip)),
                RenderDataOp::PushGeometryClip { geometry } => {
                    scopes.push(visitor.on_push_geometry_clip(resources.geometry_impl(*geometry).as_ref()))
                }
                RenderDataOp::PushOpacity { opacity } => scopes.push(visitor.on_push_opacity(*opacity)),
                RenderDataOp::PushOpacityMask { brush, bounds } => {
                    scopes.push(visitor.on_push_opacity_mask(resources.brush(*brush), *bounds))
                }
                RenderDataOp::PushTransform { matrix } => scopes.push(visitor.on_push_transform(*matrix)),
                RenderDataOp::PushRenderOptions { options } => scopes.push(visitor.on_push_render_options(*options)),
                RenderDataOp::PushTextOptions { options } => scopes.push(visitor.on_push_text_options(*options)),
                RenderDataOp::PushEffect { effect, bounds } => {
                    scopes.push(visitor.on_push_effect(resources.effect(*effect), *bounds))
                }
                RenderDataOp::Pop => {
                    let scope = scopes.pop().expect("a pop without a matching push in render data");
                    visitor.on_pop(scope);
                }
            }
        }
        scopes.clear();
    }

    /// Releases the stream.
    pub fn dispose(&mut self) {
        self.ops = Vec::new();
        self.resources.dispose();
    }
}
