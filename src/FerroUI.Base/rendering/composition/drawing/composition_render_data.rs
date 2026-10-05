use super::render_data_drawing_context::RecordedResource;
use super::{RenderDataStream, ServerCompositionRenderData};
use crate::media::{Geometry, IntersectionResult};
use crate::rendering::composition::server::ServerObjectId;
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::{Compositor, ICompositorSerializable};
use crate::{Point, Ref};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// What a visual drew, on the UI thread: the recorded stream (kept for hit
/// testing) and the id of its server-side counterpart.
pub struct CompositionRenderData {
    compositor: Weak<Compositor>,
    stream: RefCell<RenderDataStream>,
    resources: RefCell<Vec<RecordedResource>>,
    items_sent: Cell<bool>,
    server: ServerObjectId,
}

impl CompositionRenderData {
    pub(crate) fn new(
        compositor: &Rc<Compositor>,
        stream: RenderDataStream,
        resources: Vec<RecordedResource>,
    ) -> Rc<CompositionRenderData> {
        let server = compositor.create_server_object(|server_compositor, _| ServerCompositionRenderData::new(server_compositor));
        Rc::new(CompositionRenderData {
            compositor: Rc::downgrade(compositor),
            stream: RefCell::new(stream),
            resources: RefCell::new(resources),
            items_sent: Cell::new(false),
            server,
        })
    }

    /// The id of the server-side render data.
    pub fn server(&self) -> ServerObjectId {
        self.server
    }

    /// Releases the render data: the resource references it holds and its
    /// server-side counterpart.
    pub fn dispose(&self) {
        let mut stream = self.stream.borrow_mut();
        if !self.items_sent.get() {
            stream.dispose_resources();
        }
        let compositor = self.compositor.upgrade();
        for resource in self.resources.borrow_mut().drain(..) {
            if let Some(compositor) = &compositor {
                resource.with_resource(|r| r.release_on_compositor(compositor));
            }
        }
        stream.dispose();
        self.items_sent.set(false);
        if let Some(compositor) = compositor {
            compositor.dispose_on_next_batch(self.server);
        }
    }

    /// Whether anything drawn is hit at a point.
    pub fn hit_test(&self, pt: Point) -> bool {
        self.stream.borrow().hit_test(pt)
    }

    /// How the drawn content intersects a geometry.
    pub fn hit_test_geometry(&self, geometry: &Ref<Geometry>) -> IntersectionResult {
        self.stream.borrow().hit_test_geometry(geometry)
    }
}

impl ICompositorSerializable for CompositionRenderData {
    fn try_get_server(&self, _c: &Compositor) -> Option<ServerObjectId> {
        Some(self.server)
    }

    fn serialize_changes(&self, _c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        self.stream.borrow().serialize_to(writer);
        self.items_sent.set(true);
    }
}
