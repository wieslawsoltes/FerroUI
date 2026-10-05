use super::{
    CompositionProperty, IServerAnimatedPropertyHost, IServerRenderResource, ServerCompositor, ServerObjectAnimations,
    ServerValueChange,
};
use crate::rendering::composition::animations::IAnimationInstance;
use crate::rendering::composition::expressions::{ExpressionObjectKey, ExpressionVariant, IExpressionObject};
use crate::media::{IBrush, IGradientStop, IPen, ITransform};
use crate::rendering::composition::drawing::IRenderDataGeometry;
use crate::rendering::composition::transport::BatchStreamReader;
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// Identifies a server-side object within its server compositor.
///
/// UI-thread objects never hold a server object; they hold its id, and the
/// batch that creates, changes or disposes the object refers to it by id.
/// The id is a plain value that can cross threads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ServerObjectId(pub(crate) u32);

impl ServerObjectId {
    /// The slot index of the object.
    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// The base contract of every server-side object (upstream
/// `SimpleServerObject`).
pub trait IServerObject: 'static {
    /// Reads the changes of the object from a batch.
    fn deserialize_changes_core(&self, _reader: &mut BatchStreamReader<'_>, _committed_at: Duration) {}

    /// Called after a group of values of the object changed.
    fn values_invalidated(&self) {}

    /// Applies the changes of the object from a batch.
    fn deserialize_changes(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.deserialize_changes_core(reader, committed_at);
        self.values_invalidated();
    }

    /// Releases the object: it has been disposed on the UI thread.
    fn dispose(&self) {}

    /// The object as a render resource, if it is one.
    fn as_render_resource(self: Rc<Self>) -> Option<Rc<dyn IServerRenderResource>> {
        None
    }

    /// The object as a brush, if it is a server-side brush.
    fn as_brush(self: Rc<Self>) -> Option<Rc<dyn IBrush>> {
        None
    }

    /// The object as a pen, if it is a server-side pen.
    fn as_pen(self: Rc<Self>) -> Option<Rc<dyn IPen>> {
        None
    }

    /// The object as a geometry usable by render data, if it is a
    /// server-side geometry.
    fn as_render_data_geometry(self: Rc<Self>) -> Option<Rc<dyn IRenderDataGeometry>> {
        None
    }

    /// The object as a transform, if it is a server-side transform.
    fn as_transform(self: Rc<Self>) -> Option<Rc<dyn ITransform>> {
        None
    }

    /// The object as a gradient stop, if it is a server-side gradient stop.
    fn as_gradient_stop(self: Rc<Self>) -> Option<Rc<dyn IGradientStop>> {
        None
    }

    /// The object as a composition surface, if it is one.
    fn as_surface(self: Rc<Self>) -> Option<Rc<dyn super::IServerCompositionSurface>> {
        None
    }

    /// The object as an animatable object, if it is one.
    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        None
    }

    /// The generated property block of the given type embedded in the
    /// object, if it has one. A class that embeds generated properties
    /// forwards to their `find_props`; composition properties locate their
    /// fields through it.
    fn get_props(&self, _type_id: TypeId) -> Option<&dyn Any> {
        None
    }

    fn as_any(&self) -> &dyn Any;

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any>;
}

/// A server object that supports animated properties: what upstream's
/// abstract `ServerObject` is to its subclasses.
///
/// A class "derives" from `ServerObject` by embedding a [`ServerObject`]
/// and implementing this trait; the embedded part reaches the class through
/// it.
pub trait IAnimatedServerObject: IServerObject + IServerAnimatedPropertyHost {
    /// The embedded `ServerObject` part.
    fn server_object(&self) -> &ServerObject;

    /// The composition property with the given name, if the class has one.
    fn get_composition_property(&self, _field_name: &str) -> Option<&'static CompositionProperty> {
        None
    }

    /// The object as the base contract (an explicit upcast).
    fn as_server_object_dyn(&self) -> &dyn IServerObject;
}

/// An animatable server object as an expression reads it: the target of an
/// animation, or an object an animation refers to through a reference
/// parameter (upstream passes the `ServerObject` itself, which implements
/// `IExpressionObject`).
#[derive(Clone)]
pub struct ServerExpressionObject(pub Rc<dyn IAnimatedServerObject>);

impl IExpressionObject for ServerExpressionObject {
    fn get_property(&self, name: &str) -> ExpressionVariant {
        self.0.server_object().get_property(name)
    }

    fn key(&self) -> ExpressionObjectKey {
        ExpressionObjectKey(Rc::as_ptr(&self.0) as *const () as usize)
    }
}

/// Server-side `CompositionObject` counterpart. Is responsible for
/// animation activation and invalidation.
pub struct ServerObject {
    compositor: Weak<ServerCompositor>,
    owner: Weak<dyn IAnimatedServerObject>,
    activation_count: Cell<u32>,
    animations: RefCell<Option<Rc<ServerObjectAnimations>>>,
}

impl ServerObject {
    /// `owner` is the object that embeds this part.
    pub fn new(compositor: &Rc<ServerCompositor>, owner: Weak<dyn IAnimatedServerObject>) -> Self {
        Self {
            compositor: Rc::downgrade(compositor),
            owner,
            activation_count: Cell::new(0),
            animations: RefCell::new(None),
        }
    }

    /// The compositor of the object, while it is alive.
    pub fn compositor(&self) -> Option<Rc<ServerCompositor>> {
        self.compositor.upgrade()
    }

    pub fn animations(&self) -> Option<Rc<ServerObjectAnimations>> {
        self.animations.borrow().clone()
    }

    pub fn get_or_create_animations(&self) -> Rc<ServerObjectAnimations> {
        self.animations
            .borrow_mut()
            .get_or_insert_with(|| Rc::new(ServerObjectAnimations::new(self.owner.clone(), self.compositor.clone())))
            .clone()
    }

    pub fn is_active(&self) -> bool {
        self.activation_count.get() != 0
    }

    pub fn activate(&self) {
        self.activation_count.set(self.activation_count.get() + 1);
        if self.activation_count.get() == 1 {
            self.activated();
        }
    }

    pub fn deactivate(&self) {
        if cfg!(debug_assertions) && self.activation_count.get() == 0 {
            panic!("the server object is not active");
        }
        self.activation_count.set(self.activation_count.get().saturating_sub(1));
        if self.activation_count.get() == 0 {
            self.deactivated();
        }
    }

    fn activated(&self) {
        if let Some(animations) = self.animations() {
            animations.activated();
        }
    }

    fn deactivated(&self) {
        if let Some(animations) = self.animations() {
            animations.deactivated();
        }
    }

    /// `SetValue(prop, ref field, value)` of an animatable object: assigns
    /// and invalidates the subscriptions of the property.
    pub fn set_value(&self, property: &'static CompositionProperty, change: ServerValueChange<'_>) {
        (change.assign)();
        if let Some(animations) = self.animations() {
            animations.on_set_direct_value(property);
        }
    }

    /// `SetAnimatedValue(prop, ref field, committedAt, animation)`.
    pub fn set_animated_value(
        &self,
        property: &'static CompositionProperty,
        current_value: ExpressionVariant,
        committed_at: Duration,
        animation: Rc<dyn IAnimationInstance>,
    ) {
        self.get_or_create_animations().on_set_animated_value(property, current_value, committed_at, animation);
    }

    /// The tail of `SetAnimatedValue(prop, out field, value)`.
    pub fn remove_animation_for_property(&self, property: &'static CompositionProperty) {
        if let Some(animations) = self.animations() {
            animations.remove_animation_for_property(property);
        }
    }

    /// `IExpressionObject.GetProperty` of the owner.
    pub fn get_property(&self, name: &str) -> ExpressionVariant {
        match self.animations() {
            None => {
                let Some(owner) = self.owner.upgrade() else { return ExpressionVariant::default() };
                owner
                    .get_composition_property(name)
                    .and_then(|property| property.get_variant())
                    .map(|get| get(owner.as_server_object_dyn()))
                    .unwrap_or_default()
            }
            Some(animations) => animations.get_property_for_animation(name),
        }
    }
}

/// Implements the property-host and animation contracts of a class that
/// embeds a [`ServerObject`] in the field `$object` and a
/// `Weak<ServerCompositor>` in the field `compositor`: what deriving from
/// upstream's `ServerObject` provides.
#[doc(hidden)]
#[macro_export]
macro_rules! __impl_animated_server_object {
    ($ty:ty, $object:ident $(, composition_property: $get:expr)?) => {
        impl $crate::rendering::composition::server::IServerPropertyHost for $ty {
            fn server_compositor(
                &self,
            ) -> Option<::std::rc::Rc<$crate::rendering::composition::server::ServerCompositor>> {
                self.$object.compositor()
            }

            fn set_value(
                &self,
                property: &'static $crate::rendering::composition::server::CompositionProperty,
                change: $crate::rendering::composition::server::ServerValueChange<'_>,
            ) {
                self.$object.set_value(property, change);
            }
        }

        impl $crate::rendering::composition::server::IServerAnimatedPropertyHost for $ty {
            fn set_animated_value(
                &self,
                property: &'static $crate::rendering::composition::server::CompositionProperty,
                current_value: $crate::rendering::composition::expressions::ExpressionVariant,
                committed_at: ::std::time::Duration,
                animation: ::std::rc::Rc<dyn $crate::rendering::composition::animations::IAnimationInstance>,
            ) {
                self.$object.set_animated_value(property, current_value, committed_at, animation);
            }

            fn remove_animation_for_property(
                &self,
                property: &'static $crate::rendering::composition::server::CompositionProperty,
            ) {
                self.$object.remove_animation_for_property(property);
            }

            fn notify_animated_value_changed(
                &self,
                _property: &'static $crate::rendering::composition::server::CompositionProperty,
            ) {
                $crate::rendering::composition::server::IServerObject::values_invalidated(self);
            }
        }

        impl $crate::rendering::composition::server::IAnimatedServerObject for $ty {
            fn server_object(&self) -> &$crate::rendering::composition::server::ServerObject {
                &self.$object
            }

            $(fn get_composition_property(
                &self,
                field_name: &str,
            ) -> Option<&'static $crate::rendering::composition::server::CompositionProperty> {
                ($get)(field_name)
            })?

            fn as_server_object_dyn(&self) -> &dyn $crate::rendering::composition::server::IServerObject {
                self
            }
        }
    };
}

pub use crate::__impl_animated_server_object as impl_animated_server_object;
