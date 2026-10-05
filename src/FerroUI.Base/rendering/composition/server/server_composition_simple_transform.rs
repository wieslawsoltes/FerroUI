use super::{impl_simple_server_render_resource, IServerObject, IServerRenderResource, ServerCompositor, SimpleServerRenderResource};
use crate::media::ITransform;
use crate::rendering::composition::generated::{ServerCompositionSimpleTransformHooks, ServerCompositionSimpleTransformProps};
use crate::rendering::composition::transport::BatchStreamReader;
use crate::Matrix;
use std::any::{Any, TypeId};
use std::rc::Rc;
use std::time::Duration;

/// The server-side counterpart of a mutable transform.
pub struct ServerCompositionSimpleTransform {
    base: SimpleServerRenderResource,
    props: ServerCompositionSimpleTransformProps,
}

impl ServerCompositionSimpleTransform {
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionSimpleTransform> {
        Rc::new_cyclic(|this| {
            let this: std::rc::Weak<ServerCompositionSimpleTransform> = this.clone();
            ServerCompositionSimpleTransform {
                base: SimpleServerRenderResource::new(compositor, this),
                props: ServerCompositionSimpleTransformProps::new(),
            }
        })
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }

    pub fn value(&self) -> Matrix {
        self.props.value()
    }

    pub fn set_value(&self, value: Matrix) {
        self.props.set_value(self, value)
    }
}

impl_simple_server_render_resource!(ServerCompositionSimpleTransform, base);

impl ServerCompositionSimpleTransformHooks for ServerCompositionSimpleTransform {}

impl IServerObject for ServerCompositionSimpleTransform {
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

    fn as_transform(self: Rc<Self>) -> Option<Rc<dyn ITransform>> {
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

impl ITransform for ServerCompositionSimpleTransform {
    fn value(&self) -> Matrix {
        self.props.value()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
