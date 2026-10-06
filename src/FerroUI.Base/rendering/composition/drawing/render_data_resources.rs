use super::IRenderDataGeometry;
use crate::media::{IBrush, IEffect, IPen};
use crate::platform::{IBitmapImpl, IGeometryImpl, IGlyphRunImpl};
use crate::rendering::composition::server::ServerObjectId;
use crate::rendering::scene_graph::ICustomDrawOperation;
use std::collections::HashMap;
use std::rc::Rc;

/// The handle of "no resource".
pub const NULL_HANDLE: i32 = -1;

/// A resource referenced by render data.
///
/// On the UI thread a resource that has a server-side counterpart is kept
/// as the pair of the counterpart's id and the client object (which is what
/// hit testing uses). Only the id is sent; the server resolves it to the
/// server object, so a deserialized table holds no `Server*` entries.
#[derive(Clone)]
pub enum RenderDataResource {
    Brush(Rc<dyn IBrush>),
    Pen(Rc<dyn IPen>),
    GeometryImpl(Rc<dyn IGeometryImpl>),
    Geometry(Rc<dyn IRenderDataGeometry>),
    /// A recorded glyph run: one counted reference (`IRef<IGlyphRunImpl>`
    /// upstream), shared by the client render data and the server render
    /// data it is sent to, so that the glyph run is held once until both
    /// are released.
    GlyphRun(Rc<Rc<dyn IGlyphRunImpl>>),
    /// A recorded bitmap: one counted reference (`IRef<IBitmapImpl>`
    /// upstream), shared as a glyph run is.
    Bitmap(Rc<Rc<dyn IBitmapImpl>>),
    CustomDrawOperation(Rc<dyn ICustomDrawOperation>),
    Effect(Rc<dyn IEffect>),
    ServerBrush { server: ServerObjectId, client: Rc<dyn IBrush> },
    ServerPen { server: ServerObjectId, client: Rc<dyn IPen> },
    ServerGeometry { server: ServerObjectId, client: Rc<dyn IRenderDataGeometry> },
    /// A mutable pen kept on the UI thread for hit testing only; it is not
    /// sent to the server.
    ClientPen(Rc<dyn IPen>),
    /// The slot of a resource that was not sent to the server.
    NotSent,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Kind {
    Brush,
    Pen,
    GeometryImpl,
    Geometry,
    GlyphRun,
    Bitmap,
    Custom,
    Effect,
    Server,
}

impl RenderDataResource {
    /// The identity of the resource: resources are deduplicated by
    /// reference, like upstream.
    fn key(&self) -> (Kind, usize) {
        fn address<T: ?Sized>(rc: &Rc<T>) -> usize {
            Rc::as_ptr(rc) as *const () as usize
        }
        match self {
            RenderDataResource::Brush(v) => (Kind::Brush, address(v)),
            RenderDataResource::Pen(v) | RenderDataResource::ClientPen(v) => (Kind::Pen, address(v)),
            RenderDataResource::GeometryImpl(v) => (Kind::GeometryImpl, address(v)),
            RenderDataResource::Geometry(v) => (Kind::Geometry, address(v)),
            RenderDataResource::GlyphRun(v) => (Kind::GlyphRun, address(v)),
            RenderDataResource::Bitmap(v) => (Kind::Bitmap, address(v)),
            RenderDataResource::CustomDrawOperation(v) => (Kind::Custom, address(v)),
            RenderDataResource::Effect(v) => (Kind::Effect, address(v)),
            RenderDataResource::ServerBrush { server, .. }
            | RenderDataResource::ServerPen { server, .. }
            | RenderDataResource::ServerGeometry { server, .. } => (Kind::Server, server.index()),
            RenderDataResource::NotSent => (Kind::Server, usize::MAX),
        }
    }
}

/// The resource table of a render data stream: operations refer to
/// resources by handle.
#[derive(Default)]
pub struct RenderDataResources {
    resources: Vec<RenderDataResource>,
    intern_map: Option<HashMap<(Kind, usize), i32>>,
}

impl RenderDataResources {
    pub fn count(&self) -> usize {
        self.resources.len()
    }

    /// Recording path: dedupes by reference equality so a resource reused
    /// across many draws gets one slot.
    pub fn intern(&mut self, resource: Option<RenderDataResource>) -> i32 {
        let Some(resource) = resource else { return NULL_HANDLE };
        let key = resource.key();
        let intern_map = self.intern_map.get_or_insert_with(HashMap::new);
        if let Some(handle) = intern_map.get(&key) {
            return *handle;
        }
        let handle = self.resources.len() as i32;
        self.resources.push(resource);
        intern_map.insert(key, handle);
        handle
    }

    /// Deserialize path: appends without deduping since the wire format is
    /// already deduped.
    pub fn append_deserialized(&mut self, resource: RenderDataResource) -> i32 {
        let handle = self.resources.len() as i32;
        self.resources.push(resource);
        handle
    }

    /// The resource of a handle; `None` for [`NULL_HANDLE`].
    pub fn get(&self, handle: i32) -> Option<&RenderDataResource> {
        if handle == NULL_HANDLE {
            None
        } else {
            Some(&self.resources[handle as usize])
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &RenderDataResource> {
        self.resources.iter()
    }

    pub fn brush(&self, handle: i32) -> Option<&dyn IBrush> {
        match self.get(handle)? {
            RenderDataResource::Brush(brush) | RenderDataResource::ServerBrush { client: brush, .. } => Some(&**brush),
            _ => panic!("the render data resource is not a brush"),
        }
    }

    pub fn pen(&self, handle: i32) -> Option<&dyn IPen> {
        match self.get(handle)? {
            RenderDataResource::Pen(pen)
            | RenderDataResource::ClientPen(pen)
            | RenderDataResource::ServerPen { client: pen, .. } => Some(&**pen),
            RenderDataResource::NotSent => None,
            _ => panic!("the render data resource is not a pen"),
        }
    }

    pub fn geometry_impl(&self, handle: i32) -> Option<Rc<dyn IGeometryImpl>> {
        match self.get(handle)? {
            RenderDataResource::GeometryImpl(geometry) => Some(geometry.clone()),
            RenderDataResource::Geometry(geometry) | RenderDataResource::ServerGeometry { client: geometry, .. } => {
                geometry.geometry_impl()
            }
            _ => panic!("the render data resource is not a geometry"),
        }
    }

    pub fn glyph_run(&self, handle: i32) -> Option<&Rc<dyn IGlyphRunImpl>> {
        match self.get(handle)? {
            RenderDataResource::GlyphRun(glyph_run) => Some(&**glyph_run),
            _ => panic!("the render data resource is not a glyph run"),
        }
    }

    pub fn bitmap(&self, handle: i32) -> Option<&Rc<dyn IBitmapImpl>> {
        match self.get(handle)? {
            RenderDataResource::Bitmap(bitmap) => Some(&**bitmap),
            _ => panic!("the render data resource is not a bitmap"),
        }
    }

    pub fn custom_draw_operation(&self, handle: i32) -> Option<&Rc<dyn ICustomDrawOperation>> {
        match self.get(handle)? {
            RenderDataResource::CustomDrawOperation(operation) => Some(operation),
            _ => panic!("the render data resource is not a custom draw operation"),
        }
    }

    pub fn effect(&self, handle: i32) -> Option<&Rc<dyn IEffect>> {
        match self.get(handle)? {
            RenderDataResource::Effect(effect) => Some(effect),
            _ => panic!("the render data resource is not an effect"),
        }
    }

    /// Releases the table.
    pub fn dispose(&mut self) {
        self.resources.clear();
        self.intern_map = None;
    }
}
