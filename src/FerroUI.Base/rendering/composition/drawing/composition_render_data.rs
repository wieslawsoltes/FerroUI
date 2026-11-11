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

    /// Whether this render data holds `stream` with `resources`: the same operations on the same
    /// resources, by identity (an addition, desktop-performance.md section 9.14).
    pub(crate) fn has_recording(&self, stream: &RenderDataStream, resources: &[RecordedResource]) -> bool {
        let mine = self.resources.borrow();
        mine.len() == resources.len()
            && mine.iter().zip(resources).all(|(a, b)| a.identity() == b.identity())
            && self.stream.borrow().same_recording(stream)
    }

    /// Takes a new recording in place of the one the render data holds, and returns the stream
    /// it held, emptied for another recording. The server-side render data stays: it receives
    /// the recording with the next batch, as it received the first one, where a new render data
    /// would be created for the recording and this one disposed (FerroUI addition,
    /// desktop-performance.md, section 9.28). The caller registers the render data for
    /// serialization and has the visual drawn again.
    pub(crate) fn replace_recording(
        &self,
        stream: RenderDataStream,
        resources: Vec<RecordedResource>,
    ) -> RenderDataStream {
        let mut old_stream = self.stream.replace(stream);
        // As in `dispose`: a stream that was sent is released by the server-side render data,
        // which resets itself when it reads the new one.
        if !self.items_sent.get() {
            old_stream.dispose_resources();
        }
        self.items_sent.set(false);
        // The new recording took its references while it was recorded, so a resource of both
        // recordings stays referenced while those of the old one are released.
        let old_resources = self.resources.replace(resources);
        if let Some(compositor) = self.compositor.upgrade() {
            for resource in old_resources {
                resource.with_resource(|r| r.release_on_compositor(&compositor));
            }
        }
        old_stream.recycle();
        old_stream
    }

    /// Whether the render data was made for `compositor`.
    pub(crate) fn is_of(&self, compositor: &Rc<Compositor>) -> bool {
        std::ptr::eq(self.compositor.as_ptr(), Rc::as_ptr(compositor))
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
