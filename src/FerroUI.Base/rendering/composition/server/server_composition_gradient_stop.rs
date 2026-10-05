use super::{
    impl_server_render_resource, IAnimatedServerObject, IServerObject, IServerRenderResource, ServerCompositor,
    ServerRenderResource,
};
use crate::media::{Color, IGradientStop};
use crate::rendering::composition::generated::{ServerCompositionGradientStopHooks, ServerCompositionGradientStopProps};
use crate::rendering::composition::transport::BatchStreamReader;
use std::any::{Any, TypeId};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// The server-side counterpart of a
/// [`CompositionGradientStop`](crate::rendering::composition::CompositionGradientStop).
pub struct ServerCompositionGradientStop {
    base: ServerRenderResource,
    props: ServerCompositionGradientStopProps,
}

impl ServerCompositionGradientStop {
    /// Creates the stop, active: the UI-thread stop activates its server
    /// object when it is created (`InitializeDefaultsExtra`), which is here
    /// done when the server object is created for it.
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionGradientStop> {
        let stop = Rc::new_cyclic(|this: &Weak<ServerCompositionGradientStop>| ServerCompositionGradientStop {
            base: ServerRenderResource::new(
                compositor,
                this.clone() as Weak<dyn IAnimatedServerObject>,
                this.clone() as Weak<dyn IServerRenderResource>,
            ),
            props: ServerCompositionGradientStopProps::new(),
        });
        stop.base.object().activate();
        stop
    }

    /// The properties of the stop.
    pub fn props(&self) -> &ServerCompositionGradientStopProps {
        &self.props
    }

    pub fn is_disposed(&self) -> bool {
        self.base.is_disposed()
    }
}

impl_server_render_resource!(
    ServerCompositionGradientStop,
    base,
    composition_property: ServerCompositionGradientStopProps::get_composition_property
);

impl ServerCompositionGradientStopHooks for ServerCompositionGradientStop {}

impl IServerObject for ServerCompositionGradientStop {
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

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        Some(self)
    }

    fn as_gradient_stop(self: Rc<Self>) -> Option<Rc<dyn IGradientStop>> {
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

impl IGradientStop for ServerCompositionGradientStop {
    fn color(&self) -> Color {
        self.props.color()
    }

    fn offset(&self) -> f64 {
        self.props.offset()
    }
}
