use super::animations::{ICompositionAnimation, ICompositionAnimationBase, ImplicitAnimationCollection};
use super::expressions::ExpressionVariant;
use super::generated::{
    CompositionGradientStopHooks, CompositionGradientStopProps, ServerCompositionGradientStopProps,
};
use super::server::{CompositionProperty, ServerCompositionGradientStop, ServerObjectId};
use super::transport::{BatchStreamWriter, IRegisterForSerialization};
use super::{
    AsCompositionObject, CompositionObject, Compositor, ICompositionObject, ICompositionObjectAnimations,
    ICompositionObjectHost, ICompositorSerializable, PendingAnimations,
};
use crate::media::{Color, IGradientStop};
use crate::utilities::MathUtilities;
use std::any::Any;
use std::rc::{Rc, Weak};

/// A gradient stop of the composition brushes, with animatable color and
/// offset.
pub struct CompositionGradientStop {
    this: Weak<CompositionGradientStop>,
    object: CompositionObject,
    props: CompositionGradientStopProps,
}

impl CompositionGradientStop {
    fn create(compositor: &Rc<Compositor>) -> Rc<CompositionGradientStop> {
        let server = compositor.create_server_object(|compositor, _| ServerCompositionGradientStop::new(compositor));
        Rc::new_cyclic(|this: &Weak<CompositionGradientStop>| CompositionGradientStop {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            props: CompositionGradientStopProps::new(),
        })
    }

    pub(crate) fn new(compositor: &Rc<Compositor>) -> Rc<CompositionGradientStop> {
        let stop = Self::create(compositor);
        stop.props.initialize_defaults(&*stop);
        stop
    }

    pub(crate) fn with_offset_and_color(compositor: &Rc<Compositor>, offset: f64, color: Color) -> Rc<CompositionGradientStop> {
        let stop = Self::create(compositor);
        let mut offset = offset;
        if MathUtilities::is_zero(offset) {
            offset = 0.0;
        }
        stop.set_offset(if offset < 0.0 {
            0.0
        } else if offset > 1.0 {
            1.0
        } else {
            offset
        });
        stop.set_color(color);
        stop.props.initialize_defaults(&*stop);
        stop
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The id of the server-side stop.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    pub fn color(&self) -> Color {
        self.props.color()
    }

    pub fn set_color(&self, value: Color) {
        self.props.set_color(self, value)
    }

    pub fn offset(&self) -> f64 {
        self.props.offset()
    }

    pub fn set_offset(&self, value: f64) {
        self.props.set_offset(self, value)
    }

    /// The collection of implicit animations attached to this object.
    pub fn implicit_animations(&self) -> Option<Rc<ImplicitAnimationCollection>> {
        self.object.implicit_animations()
    }

    pub fn set_implicit_animations(&self, value: Option<Rc<ImplicitAnimationCollection>>) {
        self.object.set_implicit_animations(value)
    }
}

/// `InitializeDefaultsExtra` activates the server object: done when the
/// server object is created (see [`ServerCompositionGradientStop::new`]).
impl CompositionGradientStopHooks for CompositionGradientStop {}

impl IGradientStop for CompositionGradientStop {
    fn color(&self) -> Color {
        self.props.color()
    }

    fn offset(&self) -> f64 {
        self.props.offset()
    }

    fn as_composition_gradient_stop(&self) -> Option<&CompositionGradientStop> {
        Some(self)
    }
}

impl IRegisterForSerialization for CompositionGradientStop {
    fn register_for_serialization(&self) {
        self.object
            .register_for_serialization(|| self.this.upgrade().map(|this| this as Rc<dyn ICompositorSerializable>));
    }
}

impl ICompositionObjectHost for CompositionGradientStop {
    fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    fn pending_animations(&self) -> &PendingAnimations {
        self.object.pending_animations()
    }

    fn implicit_animation(&self, property_name: &str) -> Option<Rc<dyn ICompositionAnimationBase>> {
        self.object.implicit_animation(property_name)
    }

    fn start_animation_group(
        &self,
        grp: &Rc<dyn ICompositionAnimationBase>,
        target: &str,
        final_value: ExpressionVariant,
    ) -> bool {
        self.start_animation_group_for(&**grp, target, final_value)
    }
}

impl ICompositionObjectAnimations for CompositionGradientStop {
    fn try_start_animation(
        &self,
        property_name: &str,
        animation: &dyn ICompositionAnimation,
        final_value: Option<ExpressionVariant>,
    ) -> bool {
        self.props.start_animation(self, property_name, animation, final_value)
    }

    fn get_composition_property(&self, property_name: &str) -> Option<&'static CompositionProperty> {
        ServerCompositionGradientStopProps::get_composition_property(property_name)
    }

    fn composition_object(&self) -> &CompositionObject {
        &self.object
    }
}

impl AsCompositionObject for CompositionGradientStop {
    fn as_composition_object(&self) -> &CompositionObject {
        &self.object
    }

    fn composition_type_name(&self) -> &'static str {
        "CompositionGradientStop"
    }
}

impl ICompositionObject for CompositionGradientStop {
    fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}

impl ICompositorSerializable for CompositionGradientStop {
    fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId> {
        self.object.try_get_server(c)
    }

    fn serialize_changes(&self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        self.object.begin_serialize_changes(c);
        self.props.serialize_changes_core(self, writer);
    }
}
