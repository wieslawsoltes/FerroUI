use super::server::ServerObjectId;
use super::transport::BatchStreamWriter;
use super::Compositor;

/// An object whose changes are serialized into the next batch of a
/// compositor.
pub trait ICompositorSerializable {
    /// The server-side counterpart of the object on compositor `c`, if it
    /// has one.
    fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId>;

    /// The identity of the object, by which a compositor avoids queueing
    /// it twice. The default is the address of the implementing value; an
    /// adapter around an object returns the identity of that object.
    fn serialization_key(&self) -> *const () {
        self as *const Self as *const ()
    }

    /// Called before the changes of the object are written for compositor
    /// `c`.
    ///
    /// A server object is referred to by its id and created by the batch
    /// itself, so an object whose changes refer to server objects that only
    /// come to exist while it is serialized (the recorded content of a
    /// scene brush) creates them here: their creation is then written
    /// before the changes that refer to them.
    fn prepare_serialization(&self, _c: &Compositor) {}

    /// Writes the pending changes of the object for compositor `c`.
    fn serialize_changes(&self, c: &Compositor, writer: &mut BatchStreamWriter<'_>);
}
