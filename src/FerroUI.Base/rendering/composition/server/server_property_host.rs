//! The seam between the generated property blocks of server objects and the
//! hand-written classes that embed them.

use super::{CompositionProperty, IServerObject, IServerRenderResource, ServerCompositor};
use crate::rendering::composition::animations::IAnimationInstance;
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::transport::{BatchResource, BatchStreamReader};
use std::rc::Rc;
use std::time::Duration;

/// One assignment of a property field, in the form `SetValue(prop, ref
/// field, value)` takes it upstream: the class decides what else happens
/// around the assignment.
pub struct ServerValueChange<'a> {
    /// Whether the new value equals the current one.
    pub equal: bool,
    /// The render resource behind the current value, if it is one.
    pub old_resource: Option<Rc<dyn IServerRenderResource>>,
    /// The render resource behind the new value, if it is one.
    pub new_resource: Option<Rc<dyn IServerRenderResource>>,
    /// Performs `field = value`.
    pub assign: &'a mut dyn FnMut(),
}

/// What a generated server property block needs from the object that embeds
/// it (upstream `SimpleServerObject`).
pub trait IServerPropertyHost {
    /// The compositor of the object, while it is alive. Used to resolve the
    /// server objects a batch refers to.
    fn server_compositor(&self) -> Option<Rc<ServerCompositor>>;

    /// `SetValue(prop, ref field, value)` as the class of the object
    /// defines it. A plain server object assigns; an animated object also
    /// invalidates the subscriptions of the property; a render resource
    /// goes through [`super::ServerRenderResourceCore::set_value`].
    fn set_value(&self, property: &'static CompositionProperty, change: ServerValueChange<'_>) {
        let _ = property;
        (change.assign)();
    }
}

/// What the generated property block of an animatable server object needs
/// from the object that embeds it (the animation members of upstream
/// `ServerObject`).
pub trait IServerAnimatedPropertyHost: IServerPropertyHost {
    /// `SetAnimatedValue(prop, ref field, committedAt, animation)`: starts
    /// `animation` on the property. `current_value` is the value of the
    /// field as a variant; the animation writes the field through
    /// [`CompositionProperty::set_variant`].
    fn set_animated_value(
        &self,
        property: &'static CompositionProperty,
        current_value: ExpressionVariant,
        committed_at: Duration,
        animation: Rc<dyn IAnimationInstance>,
    );

    /// The tail of `SetAnimatedValue(prop, out field, value)`: the field
    /// has been assigned directly, so the animation of the property, if
    /// any, is removed.
    fn remove_animation_for_property(&self, property: &'static CompositionProperty);

    /// An animation wrote a new value into the field of `property`.
    fn notify_animated_value_changed(&self, property: &'static CompositionProperty);
}

/// A reference-typed property value of a server object that may be a
/// server-side render resource (a brush or a transform): the value as its
/// media interface, and the render resource behind it when it is one.
pub struct ServerResourceRef<T: ?Sized> {
    pub value: Rc<T>,
    pub resource: Option<Rc<dyn IServerRenderResource>>,
}

impl<T: ?Sized> Clone for ServerResourceRef<T> {
    fn clone(&self) -> Self {
        Self { value: self.value.clone(), resource: self.resource.clone() }
    }
}

impl<T: ?Sized> ServerResourceRef<T> {
    /// A value that is not a server object.
    pub fn from_value(value: Rc<T>) -> Self {
        Self { value, resource: None }
    }

    pub fn ptr_eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.value), Rc::as_ptr(&other.value))
    }
}

/// Resolves a resource reference read from a batch: a value is taken as it
/// is, the id of a server object is looked up and viewed through `cast`.
pub fn resolve_resource<T: ?Sized + 'static>(
    resource: Option<BatchResource<T>>,
    compositor: Option<&Rc<ServerCompositor>>,
    cast: fn(Rc<dyn IServerObject>) -> Option<Rc<T>>,
) -> Option<ServerResourceRef<T>> {
    match resource? {
        BatchResource::Value(value) => Some(ServerResourceRef::from_value(value)),
        BatchResource::Server(id) => {
            let Some(object) = compositor.and_then(|c| c.get_object(id)) else {
                panic!("a batch refers to server object {id:?}, which does not exist");
            };
            let resource = object.clone().as_render_resource();
            match cast(object) {
                Some(value) => Some(ServerResourceRef { value, resource }),
                None => panic!("server object {id:?} is not of the type the property expects"),
            }
        }
    }
}

/// Reads a resource reference from a batch and resolves it.
pub fn read_resource<T: ?Sized + 'static>(
    reader: &mut BatchStreamReader<'_>,
    compositor: Option<&Rc<ServerCompositor>>,
    cast: fn(Rc<dyn IServerObject>) -> Option<Rc<T>>,
) -> Option<ServerResourceRef<T>> {
    resolve_resource(reader.read_resource::<T>(), compositor, cast)
}

/// Reads a reference to a server object from a batch and resolves it.
pub fn read_server_object(
    reader: &mut BatchStreamReader<'_>,
    compositor: Option<&Rc<ServerCompositor>>,
) -> Option<Rc<dyn IServerObject>> {
    let id = reader.read_server_object()?;
    match compositor.and_then(|c| c.get_object(id)) {
        Some(object) => Some(object),
        None => panic!("a batch refers to server object {id:?}, which does not exist"),
    }
}

/// Equality of property values as the generated setters need it.
///
/// `differs` is the `!=` of the setter; `default_equals` is the
/// `EqualityComparer<T>.Default.Equals` of `ServerRenderResourceCore`. They
/// disagree for NaN.
pub trait ServerPropertyValue {
    fn differs(&self, other: &Self) -> bool;

    fn default_equals(&self, other: &Self) -> bool {
        !self.differs(other)
    }
}

macro_rules! impl_server_property_value {
    ($($ty:ty),* $(,)?) => {$(
        impl ServerPropertyValue for $ty {
            #[inline]
            fn differs(&self, other: &Self) -> bool {
                self != other
            }
        }
    )*};
}

impl_server_property_value!(
    bool,
    crate::Vector,
    crate::Vector3D,
    crate::Size,
    crate::Matrix,
    crate::CornerRadius,
    crate::RelativePoint,
    crate::RelativeScalar,
    crate::RelativeRect,
    crate::numerics::Quaternion,
    crate::media::Color,
    crate::media::RenderOptions,
    crate::media::TextOptions,
    crate::media::ImmutableExperimentalAcrylicMaterial,
    crate::media::PenLineCap,
    crate::media::PenLineJoin,
    crate::media::AlignmentX,
    crate::media::AlignmentY,
    crate::media::Stretch,
    crate::media::TileMode,
    crate::rendering::RendererDebugOverlays,
    crate::rendering::LayoutPassTiming,
    crate::rendering::composition::CompositionTransparencyLevel,
);

impl ServerPropertyValue for f32 {
    fn differs(&self, other: &Self) -> bool {
        self != other
    }
    fn default_equals(&self, other: &Self) -> bool {
        self == other || (self.is_nan() && other.is_nan())
    }
}

impl ServerPropertyValue for f64 {
    fn differs(&self, other: &Self) -> bool {
        self != other
    }
    fn default_equals(&self, other: &Self) -> bool {
        self == other || (self.is_nan() && other.is_nan())
    }
}

/// References compare by identity.
impl<T: ?Sized> ServerPropertyValue for Option<Rc<T>> {
    fn differs(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => !std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b)),
            (None, None) => false,
            _ => true,
        }
    }
}

impl<T: ?Sized> ServerPropertyValue for Option<ServerResourceRef<T>> {
    fn differs(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(a), Some(b)) => !a.ptr_eq(b),
            (None, None) => false,
            _ => true,
        }
    }
}

impl<T: ?Sized> ServerPropertyValue for Option<BatchResource<T>> {
    fn differs(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(BatchResource::Value(a)), Some(BatchResource::Value(b))) => {
                !std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
            }
            (Some(BatchResource::Server(a)), Some(BatchResource::Server(b))) => a != b,
            (None, None) => false,
            _ => true,
        }
    }
}

/// The render resource behind a property value, when it is one.
pub trait AsServerRenderResource {
    fn as_server_render_resource(&self) -> Option<Rc<dyn IServerRenderResource>>;
}

impl AsServerRenderResource for Option<Rc<dyn IServerObject>> {
    fn as_server_render_resource(&self) -> Option<Rc<dyn IServerRenderResource>> {
        self.clone().and_then(|object| object.as_render_resource())
    }
}

impl<T: ?Sized> AsServerRenderResource for Option<ServerResourceRef<T>> {
    fn as_server_render_resource(&self) -> Option<Rc<dyn IServerRenderResource>> {
        self.as_ref().and_then(|value| value.resource.clone())
    }
}
