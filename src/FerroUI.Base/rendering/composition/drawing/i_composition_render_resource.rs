use crate::rendering::composition::server::ServerObjectId;
use crate::rendering::composition::Compositor;
use std::rc::Rc;

/// A mutable media object (brush, pen, geometry, transform) that has a
/// server-side counterpart on every compositor it is used with.
///
/// The counterpart is reference counted per compositor: each render data
/// that draws with the object holds a reference until it is disposed.
/// `get_for_compositor` is the generic `GetForCompositor` of upstream; the
/// id refers to a server object that exposes the resource's interface
/// (`IServerObject::as_brush`, `as_pen`, `as_render_data_geometry`).
pub trait ICompositionRenderResource {
    fn add_ref_on_compositor(&self, c: &Rc<Compositor>);
    fn release_on_compositor(&self, c: &Rc<Compositor>);

    /// The server-side counterpart on compositor `c`. Panics if the
    /// resource is not attached to that compositor.
    fn get_for_compositor(&self, c: &Compositor) -> ServerObjectId;
}
