//! The composition brushes: brushes that live on a compositor and whose
//! properties can be animated.
//!
//! Upstream these are the classes `CompositionBrush`,
//! `CompositionSolidColorBrush`, `CompositionGradientBrush` and the
//! linear, radial and conic gradient brushes, generated from the schema and
//! completed by hand. Here every composition brush is one
//! [`CompositionBrush`] behind an `Rc`, which holds what the concrete class
//! adds in a [`CompositionBrushKind`]; the concrete classes are typed
//! handles around the same `Rc` ([`CompositionSolidColorBrush`],
//! [`CompositionLinearGradientBrush`], ...), as for the composition visuals.

use super::{
    ServerCompositionConicGradientBrush, ServerCompositionLinearGradientBrush, ServerCompositionRadialGradientBrush,
    ServerCompositionSolidColorBrush,
};
use crate::collections::FerroList;
use crate::media::immutable::ImmutableGradientStop;
use crate::media::{
    Color, GradientSpreadMethod, IBrush, IConicGradientBrush, IGradientBrush, IGradientStop, ILinearGradientBrush,
    IRadialGradientBrush, ISolidColorBrush, ITransform,
};
use crate::rendering::composition::animations::{
    ICompositionAnimation, ICompositionAnimationBase, ImplicitAnimationCollection,
};
use crate::rendering::composition::drawing::transform_get_server;
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::generated::{
    CompositionBrushHooks, CompositionBrushProps, CompositionConicGradientBrushHooks,
    CompositionConicGradientBrushProps, CompositionLinearGradientBrushHooks, CompositionLinearGradientBrushProps,
    CompositionRadialGradientBrushHooks, CompositionRadialGradientBrushProps, CompositionSolidColorBrushHooks,
    CompositionSolidColorBrushProps, ServerCompositionBrushProps, ServerCompositionConicGradientBrushProps,
    ServerCompositionLinearGradientBrushProps, ServerCompositionRadialGradientBrushProps,
    ServerCompositionSolidColorBrushProps,
};
use crate::rendering::composition::server::{CompositionProperty, IServerObject, ServerCompositor, ServerObjectId};
use crate::rendering::composition::transport::{BatchResource, BatchStreamWriter, IRegisterForSerialization};
use crate::rendering::composition::{
    AsCompositionObject, CompositionObject, Compositor, ICompositionObject, ICompositionObjectAnimations,
    ICompositionObjectHost, ICompositorSerializable, PendingAnimations,
};
use crate::{RelativePoint, RelativeScalar};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// What the hand-written `CompositionGradientBrush` adds to the generated
/// `CompositionBrush`: the stops and the spread method.
pub(crate) struct CompositionGradientBrushData {
    brush: CompositionBrushProps,
    gradient_stops: RefCell<FerroList<Rc<dyn IGradientStop>>>,
    spread_method: Cell<GradientSpreadMethod>,
}

impl CompositionGradientBrushData {
    fn new() -> Self {
        Self {
            brush: CompositionBrushProps::new(),
            gradient_stops: RefCell::new(FerroList::new()),
            spread_method: Cell::new(GradientSpreadMethod::default()),
        }
    }

    /// `SerializeChangesCore` of `CompositionGradientBrush`: the generated
    /// base class, then the spread method and the stops.
    fn serialize_changes_core(&self, host: &dyn CompositionBrushHooks, writer: &mut BatchStreamWriter<'_>) {
        self.brush.serialize_changes_core(host, writer);
        writer.write(self.spread_method.get());
        let stops = self.gradient_stops.borrow().clone();
        writer.write(stops.count() as i32);
        for stop in stops.to_vec() {
            let item: BatchResource<dyn IGradientStop> = match stop.as_composition_gradient_stop() {
                Some(comp) => BatchResource::Server(comp.server()),
                // A mutable UI-thread stop must not cross to the render thread
                // by reference; ship its current values instead.
                None if stop.is_immutable_gradient_stop() => BatchResource::Value(stop.clone()),
                None => BatchResource::Value(Rc::new(ImmutableGradientStop::new(stop.offset(), stop.color()))),
            };
            writer.write_resource(Some(item));
        }
    }
}

/// What the concrete class of a composition brush adds to
/// [`CompositionBrush`].
pub(crate) enum CompositionBrushKind {
    SolidColor(CompositionSolidColorBrushProps),
    LinearGradient(CompositionGradientBrushData, CompositionLinearGradientBrushProps),
    RadialGradient(CompositionGradientBrushData, CompositionRadialGradientBrushProps),
    ConicGradient(CompositionGradientBrushData, CompositionConicGradientBrushProps),
}

/// A brush of the composition API.
pub struct CompositionBrush {
    this: Weak<CompositionBrush>,
    object: CompositionObject,
    kind: CompositionBrushKind,
    transform: RefCell<Option<Rc<dyn ITransform>>>,
    relative_transform: RefCell<Option<Rc<dyn ITransform>>>,
}

impl CompositionBrush {
    fn create(
        compositor: &Rc<Compositor>,
        kind: CompositionBrushKind,
        server: impl FnOnce(&Rc<ServerCompositor>) -> Rc<dyn IServerObject> + 'static,
    ) -> Rc<CompositionBrush> {
        let server = compositor.create_server_object(move |compositor, _| server(compositor));
        Rc::new_cyclic(|this: &Weak<CompositionBrush>| CompositionBrush {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            kind,
            transform: RefCell::new(None),
            relative_transform: RefCell::new(None),
        })
    }

    /// The constructor chain of the gradient brushes: `CompositionBrush`,
    /// then the class.
    fn initialize_defaults(&self) {
        match &self.kind {
            CompositionBrushKind::SolidColor(props) => props.initialize_defaults(self),
            CompositionBrushKind::LinearGradient(data, props) => {
                data.brush.initialize_defaults(self);
                props.initialize_defaults(self);
            }
            CompositionBrushKind::RadialGradient(data, props) => {
                data.brush.initialize_defaults(self);
                props.initialize_defaults(self);
            }
            CompositionBrushKind::ConicGradient(data, props) => {
                data.brush.initialize_defaults(self);
                props.initialize_defaults(self);
            }
        }
    }

    /// The generated properties of `CompositionBrush`.
    pub fn brush_props(&self) -> &CompositionBrushProps {
        match &self.kind {
            CompositionBrushKind::SolidColor(props) => props.base(),
            CompositionBrushKind::LinearGradient(data, _)
            | CompositionBrushKind::RadialGradient(data, _)
            | CompositionBrushKind::ConicGradient(data, _) => &data.brush,
        }
    }

    fn gradient_data(&self) -> Option<&CompositionGradientBrushData> {
        match &self.kind {
            CompositionBrushKind::SolidColor(_) => None,
            CompositionBrushKind::LinearGradient(data, _)
            | CompositionBrushKind::RadialGradient(data, _)
            | CompositionBrushKind::ConicGradient(data, _) => Some(data),
        }
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The id of the server-side brush.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    pub fn is_disposed(&self) -> bool {
        self.object.is_disposed()
    }

    /// The name of the concrete class, for messages.
    pub fn type_name(&self) -> &'static str {
        match &self.kind {
            CompositionBrushKind::SolidColor(_) => "CompositionSolidColorBrush",
            CompositionBrushKind::LinearGradient(..) => "CompositionLinearGradientBrush",
            CompositionBrushKind::RadialGradient(..) => "CompositionRadialGradientBrush",
            CompositionBrushKind::ConicGradient(..) => "CompositionConicGradientBrush",
        }
    }

    /// The brush as an `IBrush` handle.
    pub fn as_brush(&self) -> Rc<dyn IBrush> {
        self.this.upgrade().expect("the brush is alive while it is used") as Rc<dyn IBrush>
    }

    /// The collection of implicit animations attached to this object.
    pub fn implicit_animations(&self) -> Option<Rc<ImplicitAnimationCollection>> {
        self.object.implicit_animations()
    }

    pub fn set_implicit_animations(&self, value: Option<Rc<ImplicitAnimationCollection>>) {
        self.object.set_implicit_animations(value)
    }

    pub fn opacity(&self) -> f64 {
        self.brush_props().opacity()
    }

    pub fn set_opacity(&self, value: f64) {
        self.brush_props().set_opacity(self, value)
    }

    pub fn transform_origin(&self) -> RelativePoint {
        self.brush_props().transform_origin()
    }

    pub fn set_transform_origin(&self, value: RelativePoint) {
        self.brush_props().set_transform_origin(self, value)
    }

    pub fn transform(&self) -> Option<Rc<dyn ITransform>> {
        self.transform.borrow().clone()
    }

    /// Sets the transform. What the server receives is the form a
    /// transform takes on a compositor (`GetServer`): an immutable
    /// transform as it is, a mutable one as a copy of its current value.
    pub fn set_transform(&self, value: Option<Rc<dyn ITransform>>) {
        let transport = transform_get_server(value.as_ref(), Some(self.compositor()));
        *self.transform.borrow_mut() = value;
        self.brush_props().set_transform(self, transport)
    }

    pub fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        self.relative_transform.borrow().clone()
    }

    /// Sets the relative transform; see [`set_transform`](Self::set_transform).
    pub fn set_relative_transform(&self, value: Option<Rc<dyn ITransform>>) {
        let transport = transform_get_server(value.as_ref(), Some(self.compositor()));
        *self.relative_transform.borrow_mut() = value;
        self.brush_props().set_relative_transform(self, transport)
    }

    fn serialize_changes_core(&self, writer: &mut BatchStreamWriter<'_>) {
        match &self.kind {
            CompositionBrushKind::SolidColor(props) => props.serialize_changes_core(self, writer),
            CompositionBrushKind::LinearGradient(_, props) => props.serialize_changes_core(self, writer),
            CompositionBrushKind::RadialGradient(_, props) => props.serialize_changes_core(self, writer),
            CompositionBrushKind::ConicGradient(_, props) => props.serialize_changes_core(self, writer),
        }
    }

    /// The `base.SerializeChangesCore` of the gradient brushes.
    fn serialize_gradient_base(&self, writer: &mut BatchStreamWriter<'_>) {
        if let Some(data) = self.gradient_data() {
            data.serialize_changes_core(self, writer);
        }
    }
}

/// `InitializeDefaultsExtra` activates the server object: done when the
/// server object is created (see the constructors of the server brushes).
impl CompositionBrushHooks for CompositionBrush {}

impl CompositionSolidColorBrushHooks for CompositionBrush {}

impl CompositionLinearGradientBrushHooks for CompositionBrush {
    fn base_serialize_changes_core(&self, writer: &mut BatchStreamWriter<'_>) {
        self.serialize_gradient_base(writer)
    }
}

impl CompositionRadialGradientBrushHooks for CompositionBrush {
    fn base_serialize_changes_core(&self, writer: &mut BatchStreamWriter<'_>) {
        self.serialize_gradient_base(writer)
    }
}

impl CompositionConicGradientBrushHooks for CompositionBrush {
    fn base_serialize_changes_core(&self, writer: &mut BatchStreamWriter<'_>) {
        self.serialize_gradient_base(writer)
    }
}

impl IRegisterForSerialization for CompositionBrush {
    fn register_for_serialization(&self) {
        self.object
            .register_for_serialization(|| self.this.upgrade().map(|this| this as Rc<dyn ICompositorSerializable>));
    }
}

impl ICompositionObjectHost for CompositionBrush {
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

impl ICompositionObjectAnimations for CompositionBrush {
    fn try_start_animation(
        &self,
        property_name: &str,
        animation: &dyn ICompositionAnimation,
        final_value: Option<ExpressionVariant>,
    ) -> bool {
        // The gradient brushes have a hand-written base class, which the
        // generated code does not chain to: the properties of
        // `CompositionBrush` are tried after those of the class.
        match &self.kind {
            CompositionBrushKind::SolidColor(props) => props.start_animation(self, property_name, animation, final_value),
            CompositionBrushKind::LinearGradient(data, props) => {
                props.start_animation(self, property_name, animation, final_value)
                    || data.brush.start_animation(self, property_name, animation, final_value)
            }
            CompositionBrushKind::RadialGradient(data, props) => {
                props.start_animation(self, property_name, animation, final_value)
                    || data.brush.start_animation(self, property_name, animation, final_value)
            }
            CompositionBrushKind::ConicGradient(data, props) => {
                props.start_animation(self, property_name, animation, final_value)
                    || data.brush.start_animation(self, property_name, animation, final_value)
            }
        }
    }

    fn get_composition_property(&self, property_name: &str) -> Option<&'static CompositionProperty> {
        match &self.kind {
            CompositionBrushKind::SolidColor(_) => {
                ServerCompositionSolidColorBrushProps::get_composition_property(property_name)
            }
            CompositionBrushKind::LinearGradient(..) => {
                ServerCompositionLinearGradientBrushProps::get_composition_property(property_name)
            }
            CompositionBrushKind::RadialGradient(..) => {
                ServerCompositionRadialGradientBrushProps::get_composition_property(property_name)
            }
            CompositionBrushKind::ConicGradient(..) => {
                ServerCompositionConicGradientBrushProps::get_composition_property(property_name)
            }
        }
        .or_else(|| ServerCompositionBrushProps::get_composition_property(property_name))
    }

    fn composition_object(&self) -> &CompositionObject {
        &self.object
    }
}

impl AsCompositionObject for CompositionBrush {
    fn as_composition_object(&self) -> &CompositionObject {
        &self.object
    }

    fn composition_type_name(&self) -> &'static str {
        self.type_name()
    }
}

impl ICompositionObject for CompositionBrush {
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

impl ICompositorSerializable for CompositionBrush {
    fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId> {
        self.object.try_get_server(c)
    }

    fn serialize_changes(&self, c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        self.object.begin_serialize_changes(c);
        self.serialize_changes_core(writer);
    }
}

// --- IBrush --------------------------------------------------------------------
//
// The brush is drawn as itself where there is no compositor, and through its
// server object on its own compositor. The members of the interfaces of the
// concrete classes are answered only by the matching kind (the `as_*`
// members of `IBrush` select it).

impl IBrush for CompositionBrush {
    fn opacity(&self) -> f64 {
        CompositionBrush::opacity(self)
    }

    fn transform(&self) -> Option<Rc<dyn ITransform>> {
        CompositionBrush::transform(self)
    }

    fn transform_origin(&self) -> RelativePoint {
        CompositionBrush::transform_origin(self)
    }

    fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        CompositionBrush::relative_transform(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_solid_color_brush(&self) -> Option<&dyn ISolidColorBrush> {
        matches!(self.kind, CompositionBrushKind::SolidColor(_)).then_some(self as &dyn ISolidColorBrush)
    }

    fn as_gradient_brush(&self) -> Option<&dyn IGradientBrush> {
        self.gradient_data().map(|_| self as &dyn IGradientBrush)
    }

    fn as_linear_gradient_brush(&self) -> Option<&dyn ILinearGradientBrush> {
        matches!(self.kind, CompositionBrushKind::LinearGradient(..)).then_some(self as &dyn ILinearGradientBrush)
    }

    fn as_radial_gradient_brush(&self) -> Option<&dyn IRadialGradientBrush> {
        matches!(self.kind, CompositionBrushKind::RadialGradient(..)).then_some(self as &dyn IRadialGradientBrush)
    }

    fn as_conic_gradient_brush(&self) -> Option<&dyn IConicGradientBrush> {
        matches!(self.kind, CompositionBrushKind::ConicGradient(..)).then_some(self as &dyn IConicGradientBrush)
    }

    fn as_composition_brush(&self) -> Option<&CompositionBrush> {
        Some(self)
    }
}

fn not_of_kind(member: &str) -> ! {
    panic!("{member} is not a member of this composition brush");
}

impl ISolidColorBrush for CompositionBrush {
    fn color(&self) -> Color {
        match &self.kind {
            CompositionBrushKind::SolidColor(props) => props.color(),
            _ => not_of_kind("Color"),
        }
    }
}

impl IGradientBrush for CompositionBrush {
    fn gradient_stops(&self) -> Vec<Rc<dyn IGradientStop>> {
        match self.gradient_data() {
            Some(data) => data.gradient_stops.borrow().to_vec(),
            None => not_of_kind("GradientStops"),
        }
    }

    fn spread_method(&self) -> GradientSpreadMethod {
        match self.gradient_data() {
            Some(data) => data.spread_method.get(),
            None => not_of_kind("SpreadMethod"),
        }
    }
}

impl ILinearGradientBrush for CompositionBrush {
    fn start_point(&self) -> RelativePoint {
        match &self.kind {
            CompositionBrushKind::LinearGradient(_, props) => props.start_point(),
            _ => not_of_kind("StartPoint"),
        }
    }

    fn end_point(&self) -> RelativePoint {
        match &self.kind {
            CompositionBrushKind::LinearGradient(_, props) => props.end_point(),
            _ => not_of_kind("EndPoint"),
        }
    }
}

impl IRadialGradientBrush for CompositionBrush {
    fn center(&self) -> RelativePoint {
        match &self.kind {
            CompositionBrushKind::RadialGradient(_, props) => props.center(),
            _ => not_of_kind("Center"),
        }
    }

    fn gradient_origin(&self) -> RelativePoint {
        match &self.kind {
            CompositionBrushKind::RadialGradient(_, props) => props.gradient_origin(),
            _ => not_of_kind("GradientOrigin"),
        }
    }

    fn radius_x(&self) -> RelativeScalar {
        match &self.kind {
            CompositionBrushKind::RadialGradient(_, props) => props.radius_x(),
            _ => not_of_kind("RadiusX"),
        }
    }

    fn radius_y(&self) -> RelativeScalar {
        match &self.kind {
            CompositionBrushKind::RadialGradient(_, props) => props.radius_y(),
            _ => not_of_kind("RadiusY"),
        }
    }
}

impl IConicGradientBrush for CompositionBrush {
    fn center(&self) -> RelativePoint {
        match &self.kind {
            CompositionBrushKind::ConicGradient(_, props) => props.center(),
            _ => not_of_kind("Center"),
        }
    }

    fn angle(&self) -> f64 {
        match &self.kind {
            CompositionBrushKind::ConicGradient(_, props) => props.angle(),
            _ => not_of_kind("Angle"),
        }
    }
}

// --- the concrete classes ------------------------------------------------------

/// A composition brush that paints with a color.
#[derive(Clone)]
pub struct CompositionSolidColorBrush(Rc<CompositionBrush>);

impl Deref for CompositionSolidColorBrush {
    type Target = Rc<CompositionBrush>;

    fn deref(&self) -> &Rc<CompositionBrush> {
        &self.0
    }
}

impl CompositionSolidColorBrush {
    fn create(compositor: &Rc<Compositor>) -> Rc<CompositionBrush> {
        CompositionBrush::create(
            compositor,
            CompositionBrushKind::SolidColor(CompositionSolidColorBrushProps::new()),
            |c| ServerCompositionSolidColorBrush::new(c),
        )
    }

    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionSolidColorBrush {
        let brush = Self::create(compositor);
        brush.initialize_defaults();
        CompositionSolidColorBrush(brush)
    }

    pub(crate) fn with_color(compositor: &Rc<Compositor>, color: Color) -> CompositionSolidColorBrush {
        let brush = Self::create(compositor);
        let this = CompositionSolidColorBrush(brush);
        // The constructor of the base class, then the color, then the
        // defaults of the class.
        this.props().base().initialize_defaults(&*this.0);
        this.set_color(color);
        this.props().initialize_own_defaults(&*this.0);
        this
    }

    /// The handle of `brush` as a solid color brush, if it is one.
    pub fn from_brush(brush: &Rc<CompositionBrush>) -> Option<CompositionSolidColorBrush> {
        matches!(brush.kind, CompositionBrushKind::SolidColor(_)).then(|| CompositionSolidColorBrush(brush.clone()))
    }

    fn props(&self) -> &CompositionSolidColorBrushProps {
        match &self.0.kind {
            CompositionBrushKind::SolidColor(props) => props,
            _ => unreachable!("a solid color brush handle wraps a solid color brush"),
        }
    }

    pub fn color(&self) -> Color {
        self.props().color()
    }

    pub fn set_color(&self, value: Color) {
        self.props().set_color(&*self.0, value)
    }
}

/// The base of the composition gradient brushes: the gradient stops and the
/// spread method.
#[derive(Clone)]
pub struct CompositionGradientBrush(Rc<CompositionBrush>);

impl Deref for CompositionGradientBrush {
    type Target = Rc<CompositionBrush>;

    fn deref(&self) -> &Rc<CompositionBrush> {
        &self.0
    }
}

impl CompositionGradientBrush {
    /// The handle of `brush` as a gradient brush, if it is one.
    pub fn from_brush(brush: &Rc<CompositionBrush>) -> Option<CompositionGradientBrush> {
        brush.gradient_data().map(|_| CompositionGradientBrush(brush.clone()))
    }

    fn data(&self) -> &CompositionGradientBrushData {
        match self.0.gradient_data() {
            Some(data) => data,
            None => unreachable!("a gradient brush handle wraps a gradient brush"),
        }
    }

    /// The gradient stops. Mutations of the list itself are not tracked - assign
    /// the property to ship in-place edits made after a commit. Stops created via
    /// [`Compositor::create_gradient_stop_with`] stay live
    /// on the server and animate individually; other stops are snapshotted at
    /// serialization time.
    pub fn gradient_stops(&self) -> FerroList<Rc<dyn IGradientStop>> {
        self.data().gradient_stops.borrow().clone()
    }

    pub fn set_gradient_stops(&self, value: FerroList<Rc<dyn IGradientStop>>) {
        if self.data().gradient_stops.borrow().ptr_eq(&value) {
            return;
        }
        *self.data().gradient_stops.borrow_mut() = value;
        self.0.register_for_serialization();
    }

    /// How the gradient repeats outside the stop range.
    pub fn spread_method(&self) -> GradientSpreadMethod {
        self.data().spread_method.get()
    }

    pub fn set_spread_method(&self, value: GradientSpreadMethod) {
        if self.data().spread_method.get() == value {
            return;
        }
        self.data().spread_method.set(value);
        self.0.register_for_serialization();
    }
}

macro_rules! gradient_brush_class {
    ($(#[$meta:meta])* $name:ident, $variant:ident, $props:ident, $server:ident) => {
        $(#[$meta])*
        #[derive(Clone)]
        pub struct $name(CompositionGradientBrush);

        impl Deref for $name {
            type Target = CompositionGradientBrush;

            fn deref(&self) -> &CompositionGradientBrush {
                &self.0
            }
        }

        impl $name {
            pub(crate) fn new(compositor: &Rc<Compositor>) -> $name {
                let brush = CompositionBrush::create(
                    compositor,
                    CompositionBrushKind::$variant(CompositionGradientBrushData::new(), $props::new()),
                    |c| $server::new(c),
                );
                brush.initialize_defaults();
                $name(CompositionGradientBrush(brush))
            }

            /// The handle of `brush` as this class, if it is one.
            pub fn from_brush(brush: &Rc<CompositionBrush>) -> Option<$name> {
                matches!(brush.kind, CompositionBrushKind::$variant(..))
                    .then(|| $name(CompositionGradientBrush(brush.clone())))
            }

            fn props(&self) -> &$props {
                match &self.0 .0.kind {
                    CompositionBrushKind::$variant(_, props) => props,
                    _ => unreachable!("the handle wraps a brush of its class"),
                }
            }

            fn host(&self) -> &CompositionBrush {
                &self.0 .0
            }
        }
    };
}

gradient_brush_class!(
    /// A composition brush that paints with a linear gradient.
    CompositionLinearGradientBrush,
    LinearGradient,
    CompositionLinearGradientBrushProps,
    ServerCompositionLinearGradientBrush
);

impl CompositionLinearGradientBrush {
    pub fn start_point(&self) -> RelativePoint {
        self.props().start_point()
    }

    pub fn set_start_point(&self, value: RelativePoint) {
        self.props().set_start_point(self.host(), value)
    }

    pub fn end_point(&self) -> RelativePoint {
        self.props().end_point()
    }

    pub fn set_end_point(&self, value: RelativePoint) {
        self.props().set_end_point(self.host(), value)
    }
}

gradient_brush_class!(
    /// A composition brush that paints with a radial gradient.
    CompositionRadialGradientBrush,
    RadialGradient,
    CompositionRadialGradientBrushProps,
    ServerCompositionRadialGradientBrush
);

impl CompositionRadialGradientBrush {
    pub fn center(&self) -> RelativePoint {
        self.props().center()
    }

    pub fn set_center(&self, value: RelativePoint) {
        self.props().set_center(self.host(), value)
    }

    pub fn gradient_origin(&self) -> RelativePoint {
        self.props().gradient_origin()
    }

    pub fn set_gradient_origin(&self, value: RelativePoint) {
        self.props().set_gradient_origin(self.host(), value)
    }

    pub fn radius_x(&self) -> RelativeScalar {
        self.props().radius_x()
    }

    pub fn set_radius_x(&self, value: RelativeScalar) {
        self.props().set_radius_x(self.host(), value)
    }

    pub fn radius_y(&self) -> RelativeScalar {
        self.props().radius_y()
    }

    pub fn set_radius_y(&self, value: RelativeScalar) {
        self.props().set_radius_y(self.host(), value)
    }

    pub fn radius(&self) -> f64 {
        self.radius_x().scalar
    }
}

gradient_brush_class!(
    /// A composition brush that paints with a conic gradient.
    CompositionConicGradientBrush,
    ConicGradient,
    CompositionConicGradientBrushProps,
    ServerCompositionConicGradientBrush
);

impl CompositionConicGradientBrush {
    pub fn angle(&self) -> f64 {
        self.props().angle()
    }

    pub fn set_angle(&self, value: f64) {
        self.props().set_angle(self.host(), value)
    }

    pub fn center(&self) -> RelativePoint {
        self.props().center()
    }

    pub fn set_center(&self, value: RelativePoint) {
        self.props().set_center(self.host(), value)
    }
}
