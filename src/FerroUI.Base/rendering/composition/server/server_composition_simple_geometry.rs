use super::{impl_simple_server_render_resource, IServerObject, IServerRenderResource, ServerCompositor, SimpleServerRenderResource};
use crate::platform::IGeometryImpl;
use crate::rendering::composition::drawing::IRenderDataGeometry;
use crate::rendering::composition::generated::{ServerCompositionSimpleGeometryHooks, ServerCompositionSimpleGeometryProps};
use crate::rendering::composition::transport::BatchStreamReader;
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::time::Duration;

/// The server-side counterpart of a geometry object: the platform geometry
/// the object currently resolves to.
pub struct ServerCompositionSimpleGeometry {
    base: SimpleServerRenderResource,
    props: ServerCompositionSimpleGeometryProps,
}

impl ServerCompositionSimpleGeometry {
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionSimpleGeometry> {
        Rc::new_cyclic(|this| {
            let this: std::rc::Weak<ServerCompositionSimpleGeometry> = this.clone();
            ServerCompositionSimpleGeometry {
                base: SimpleServerRenderResource::new(compositor, this),
                props: ServerCompositionSimpleGeometryProps::new(),
            }
        })
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    pub fn geometry_impl(&self) -> Option<Rc<dyn IGeometryImpl>> {
        self.props.geometry_impl()
    }

    pub fn set_geometry_impl(&self, value: Option<Rc<dyn IGeometryImpl>>) {
        self.props.set_geometry_impl(self, value)
    }
}

impl_simple_server_render_resource!(ServerCompositionSimpleGeometry, base);

impl ServerCompositionSimpleGeometryHooks for ServerCompositionSimpleGeometry {}

impl IServerObject for ServerCompositionSimpleGeometry {
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

    fn as_render_data_geometry(self: Rc<Self>) -> Option<Rc<dyn IRenderDataGeometry>> {
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

impl IRenderDataGeometry for ServerCompositionSimpleGeometry {
    fn geometry_impl(&self) -> Option<Rc<dyn IGeometryImpl>> {
        self.props.geometry_impl()
    }
}
