use crate::data::{BindingMode, BindingPriority};
use crate::property_store::{EffectiveValueDyn, IValueEntry};
use crate::reactive::{Disposable, IDisposable, IObservable};
use crate::utilities::HandlerList;
use crate::{
    AttachedProperty, DirectProperty, DirectPropertyMetadata, FerroObject, FerroPropertyChangedEventArgs,
    FerroPropertyMetadata, FerroPropertyRegistry, ObjectType, StaticType, StyledProperty, StyledPropertyMetadata,
    TypeInfo,
};
use std::any::{Any, TypeId};
use std::cell::{Cell, OnceCell};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

/// The bounds every property value type must satisfy.
pub trait PropertyValue: Clone + PartialEq + 'static {}

impl<T: Clone + PartialEq + 'static> PropertyValue for T {}

/// A value of any property value type, for untyped access to properties.
///
/// Implemented for every type that can be a property value. Unlike
/// [`Any`], it supports equality, so untyped values can themselves be
/// property values (for example a data context).
pub trait AnyValue: Any {
    /// The value as [`Any`]. Call [`as_any`](trait.AnyValue.html#method.as_any)
    /// on `dyn AnyValue` instead.
    ///
    /// The trait is implemented for every `PartialEq` type, which includes
    /// `Rc<dyn AnyValue>` itself. The trait methods therefore have names that
    /// cannot be reached by accident on a boxed value: the inherent methods
    /// of `dyn AnyValue` are the API, and they dereference through the box.
    #[doc(hidden)]
    fn any_value_as_any(&self) -> &dyn Any;

    /// Compares with another untyped value: equal if both hold the same type
    /// and the values are equal. Use `==` on `dyn AnyValue` instead.
    #[doc(hidden)]
    fn any_value_eq(&self, other: &dyn AnyValue) -> bool;

    /// The Rust type name of the contained value. Use `type_name` on
    /// `dyn AnyValue` instead.
    #[doc(hidden)]
    fn any_value_type_name(&self) -> &'static str;
}

impl<T: PartialEq + 'static> AnyValue for T {
    #[inline]
    fn any_value_as_any(&self) -> &dyn Any {
        self
    }

    fn any_value_eq(&self, other: &dyn AnyValue) -> bool {
        other.any_value_as_any().downcast_ref::<T>().is_some_and(|other| self == other)
    }

    fn any_value_type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

impl dyn AnyValue {
    /// The contained value as [`Any`], for downcasting.
    #[inline]
    pub fn as_any(&self) -> &dyn Any {
        self.any_value_as_any()
    }

    /// The [`TypeId`] of the contained value.
    #[inline]
    pub fn value_type_id(&self) -> TypeId {
        self.any_value_as_any().type_id()
    }

    /// The Rust type name of the contained value.
    #[inline]
    pub fn type_name(&self) -> &'static str {
        self.any_value_type_name()
    }

    /// Whether the contained value is of type `T`.
    #[inline]
    pub fn is<T: 'static>(&self) -> bool {
        self.any_value_as_any().is::<T>()
    }

    /// The contained value, if it is of type `T`.
    #[inline]
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.any_value_as_any().downcast_ref::<T>()
    }

    /// Compares with another untyped value: equal if both hold the same type
    /// and the values are equal.
    #[inline]
    pub fn value_eq(&self, other: &dyn AnyValue) -> bool {
        self.any_value_eq(other)
    }
}

impl PartialEq for dyn AnyValue {
    fn eq(&self, other: &Self) -> bool {
        self.any_value_eq(other)
    }
}

impl fmt::Debug for dyn AnyValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AnyValue<{}>", self.any_value_type_name())
    }
}

/// An untyped, shared property value. The contained value is always exactly
/// of the property's value type (or one of the marker types
/// [`UnsetValueType`] and [`DoNothingType`]).
pub type BoxedValue = Rc<dyn AnyValue>;

/// The type of the marker value that represents an unset property value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsetValueType;

impl fmt::Display for UnsetValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(unset)")
    }
}

/// The type of the marker value that tells a binding target to do nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoNothingType;

impl fmt::Display for DoNothingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("(do nothing)")
    }
}

/// A callback invoked before and after a property change notification is
/// raised on an object (`true` before, `false` after).
pub type NotifyingCallback = Box<dyn Fn(&FerroObject, bool)>;

/// Untyped operations that a property routes to its typed implementation.
pub(crate) trait PropertyRoutes {
    fn create_effective_value(&'static self, o: &FerroObject) -> Rc<dyn EffectiveValueDyn>;
    fn route_clear_value(&'static self, o: &FerroObject);
    fn route_coerce_default_value(&'static self, o: &FerroObject);
    fn route_get_value(&'static self, o: &FerroObject) -> BoxedValue;
    fn route_get_base_value(&'static self, o: &FerroObject) -> BoxedValue;
    fn route_get_default_value(&'static self, type_: &'static TypeInfo) -> BoxedValue;
    fn route_set_value(&'static self, o: &FerroObject, value: &dyn Any, priority: BindingPriority)
        -> Option<Rc<dyn IDisposable>>;
    fn route_set_current_value(&'static self, o: &FerroObject, value: &dyn Any);
    fn route_set_direct_value_unchecked(&'static self, o: &FerroObject, entry: &dyn IValueEntry);
    fn route_bind(
        &'static self,
        o: &FerroObject,
        source: Rc<dyn IObservable<BoxedValue>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable>;
    fn route_get_metadata(&'static self, type_: &'static TypeInfo) -> FerroPropertyMetadata;
    fn route_is_valid_value(&'static self, value: &dyn Any) -> bool;
    /// The typed property behind the routes, for downcasting.
    fn as_any(&'static self) -> &'static dyn Any;
    /// Clones `value`, which must hold exactly the property's value type `T`,
    /// into `out`, which must be an `&mut Option<T>`. Returns false if either
    /// has a different type.
    fn route_copy_value(&'static self, value: &dyn Any, out: &mut dyn Any) -> bool;
    /// Creates a value entry for `frame` that reads its values from `source`.
    /// Only supported by styled properties.
    fn route_create_binding_entry(
        &'static self,
        o: &FerroObject,
        frame: std::rc::Weak<dyn crate::property_store::ValueFrame>,
        source: Rc<dyn IObservable<BoxedValue>>,
    ) -> Rc<dyn IValueEntry>;
}

type ChangedHandler = dyn Fn(&FerroPropertyChangedEventArgs<'_>);

/// The stream of change notifications of a property across all objects.
pub struct PropertyChangedObservable {
    handlers: HandlerList<ChangedHandler>,
}

impl PropertyChangedObservable {
    fn new() -> Self {
        Self { handlers: HandlerList::new() }
    }

    /// Subscribes to changes of the property on any object.
    pub fn subscribe(&'static self, handler: impl Fn(&FerroPropertyChangedEventArgs<'_>) + 'static) -> Rc<dyn IDisposable> {
        let token = self.handlers.add(Rc::new(handler));
        Disposable::create(move || {
            self.handlers.remove(token);
        })
    }

    /// Subscribes a handler that is only invoked for objects of class
    /// `TTarget` (or classes derived from it).
    pub fn add_class_handler<TTarget: ObjectType>(
        &'static self,
        handler: impl Fn(&TTarget, &FerroPropertyChangedEventArgs<'_>) + 'static,
    ) -> Rc<dyn IDisposable> {
        self.subscribe(move |e| {
            if let Some(target) = e.sender().downcast_ref::<TTarget>() {
                handler(target, e);
            }
        })
    }

    #[inline]
    pub(crate) fn notify(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if self.handlers.is_empty() {
            return;
        }
        for (_, handler) in self.handlers.snapshot().iter() {
            handler(e);
        }
    }
}

static NEXT_ID: AtomicU32 = AtomicU32::new(0);

/// Base class for properties: the untyped identity and description of a
/// property that can be set on a [`FerroObject`].
pub struct FerroProperty {
    name: Box<str>,
    property_type: TypeId,
    property_type_name: &'static str,
    owner_type: &'static TypeInfo,
    id: u32,
    inherits: bool,
    is_attached: bool,
    is_direct: bool,
    is_read_only: bool,
    assign_binding: Cell<bool>,
    host_type: Cell<Option<&'static TypeInfo>>,
    notifying: Option<Rc<dyn Fn(&FerroObject, bool)>>,
    changed: Rc<PropertyChangedObservable>,
    routes: OnceCell<&'static dyn PropertyRoutes>,
}

pub(crate) struct FerroPropertyInit {
    pub inherits: bool,
    pub is_attached: bool,
    pub is_direct: bool,
    pub is_read_only: bool,
}

impl FerroProperty {
    pub(crate) fn new<T: 'static>(
        name: &str,
        owner_type: &'static TypeInfo,
        init: FerroPropertyInit,
        notifying: Option<NotifyingCallback>,
    ) -> Self {
        assert!(!name.contains('.'), "'name' may not contain periods.");
        Self {
            name: name.into(),
            property_type: TypeId::of::<T>(),
            property_type_name: std::any::type_name::<T>(),
            owner_type,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            inherits: init.inherits,
            is_attached: init.is_attached,
            is_direct: init.is_direct,
            is_read_only: init.is_read_only,
            assign_binding: Cell::new(false),
            host_type: Cell::new(None),
            notifying: notifying.map(Rc::from),
            changed: Rc::new(PropertyChangedObservable::new()),
            routes: OnceCell::new(),
        }
    }

    /// Creates the base of a property that shares its identity with `source`
    /// but has a different owner.
    pub(crate) fn for_added_owner(source: &FerroProperty, owner_type: &'static TypeInfo, is_read_only: bool) -> Self {
        Self {
            name: source.name.clone(),
            property_type: source.property_type,
            property_type_name: source.property_type_name,
            owner_type,
            id: source.id,
            inherits: source.inherits,
            is_attached: source.is_attached,
            is_direct: source.is_direct,
            is_read_only,
            assign_binding: source.assign_binding.clone(),
            host_type: source.host_type.clone(),
            notifying: source.notifying.clone(),
            changed: source.changed.clone(),
            routes: OnceCell::new(),
        }
    }

    pub(crate) fn set_routes(&self, routes: &'static dyn PropertyRoutes) {
        if self.routes.set(routes).is_err() {
            panic!("property routes are already set");
        }
    }

    #[inline]
    pub(crate) fn routes(&self) -> &'static dyn PropertyRoutes {
        *self.routes.get().expect("property is not fully registered")
    }

    /// The class an attached property is declared for: the `THost` of its
    /// registration, the type of the objects its accessors (`GetX`/`SetX`)
    /// take. `None` for properties that are not attached.
    #[inline]
    pub fn host_type(&self) -> Option<&'static TypeInfo> {
        self.host_type.get()
    }

    /// The name of the property.
    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The type of the property's value.
    #[inline]
    pub fn property_type(&self) -> TypeId {
        self.property_type
    }

    /// The Rust type name of the property's value type.
    #[inline]
    pub fn property_type_name(&self) -> &'static str {
        self.property_type_name
    }

    /// The class that registered the property.
    #[inline]
    pub fn owner_type(&self) -> &'static TypeInfo {
        self.owner_type
    }

    /// Whether the property inherits its value.
    #[inline]
    pub fn inherits(&self) -> bool {
        self.inherits
    }

    /// Whether this is an attached property.
    #[inline]
    pub fn is_attached(&self) -> bool {
        self.is_attached
    }

    /// Whether this is a direct property.
    #[inline]
    pub fn is_direct(&self) -> bool {
        self.is_direct
    }

    /// Whether this is a readonly property.
    #[inline]
    pub fn is_read_only(&self) -> bool {
        self.is_read_only
    }

    /// Whether a binding given for this property in markup is assigned to
    /// the property as its value rather than bound to it (the "assign
    /// binding" attribute of the managed property). The property's value
    /// type is then a binding type. A fact about the definition: it is the
    /// same for every owner and is not part of the metadata.
    #[inline]
    pub fn assign_binding(&self) -> bool {
        self.assign_binding.get()
    }

    /// Marks the property as one that bindings are assigned to. Part of the
    /// definition of the property: call it where the property is registered
    /// (styled and attached properties state it through
    /// [`StyledPropertyOptions::assign_binding`]).
    pub fn set_assign_binding(&self, value: bool) {
        self.assign_binding.set(value);
    }

    /// An integer ID that uniquely identifies the property; shared by all
    /// owners added with `add_owner`.
    #[inline]
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Notifications of changes to this property on any object.
    ///
    /// The handlers are invoked after the object's own change handling.
    #[inline]
    pub fn changed(&'static self) -> &'static PropertyChangedObservable {
        // The observable is shared between a property and its added owners.
        &self.changed
    }

    #[inline]
    pub(crate) fn notify_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        self.changed.notify(e)
    }

    #[inline]
    pub(crate) fn notifying(&self) -> Option<&dyn Fn(&FerroObject, bool)> {
        self.notifying.as_deref()
    }

    /// Gets the base metadata of the property on the specified type.
    pub fn get_metadata(&self, type_: &'static TypeInfo) -> FerroPropertyMetadata {
        self.routes().route_get_metadata(type_)
    }

    /// Gets the base metadata of the property on the type of `owner`.
    pub fn get_metadata_for(&self, owner: &FerroObject) -> FerroPropertyMetadata {
        self.routes().route_get_metadata(owner.get_type())
    }

    /// Checks whether `value` is valid for the property: of the property's
    /// value type and accepted by its validation callback.
    pub fn is_valid_value(&self, value: &dyn Any) -> bool {
        self.routes().route_is_valid_value(value)
    }

    /// The property as a styled (or attached) property with value type `T`:
    /// the typed view needed to read, set and bind it without boxing.
    /// `None` for direct properties and when `T` is not the property's value
    /// type.
    pub fn as_styled<T: crate::PropertyValue>(&'static self) -> Option<&'static StyledProperty<T>> {
        self.routes.get().and_then(|routes| routes.as_any().downcast_ref::<StyledProperty<T>>())
    }

    /// The marker value that represents an unset property value.
    pub fn unset_value() -> BoxedValue {
        thread_local! {
            static UNSET: BoxedValue = Rc::new(UnsetValueType);
        }
        UNSET.with(Rc::clone)
    }

    /// Registers a styled property with a default value.
    pub fn register<TOwner: ObjectType, TValue: PropertyValue>(
        name: &str,
        default_value: TValue,
    ) -> &'static StyledProperty<TValue> {
        Self::register_with::<TOwner, TValue>(name, StyledPropertyOptions::new(default_value))
    }

    /// Registers a styled property.
    pub fn register_with<TOwner: ObjectType, TValue: PropertyValue>(
        name: &str,
        options: StyledPropertyOptions<TValue>,
    ) -> &'static StyledProperty<TValue> {
        let assign_binding = options.assign_binding;
        let (metadata, inherits, validate, notifying) = options.into_parts();
        let result = StyledProperty::create(name, TOwner::TYPE, TOwner::TYPE, metadata, inherits, validate, notifying, false);
        result.set_assign_binding(assign_binding);
        FerroPropertyRegistry::instance().register(TOwner::TYPE, result);
        result
    }

    /// Registers an attached property with a default value.
    ///
    /// `TOwner` is the class that registers the property and `THost` the class
    /// of objects the property can be set on.
    pub fn register_attached<TOwner: StaticType, THost: ObjectType, TValue: PropertyValue>(
        name: &str,
        default_value: TValue,
    ) -> &'static AttachedProperty<TValue> {
        Self::register_attached_with::<TOwner, THost, TValue>(name, StyledPropertyOptions::new(default_value))
    }

    /// Registers an attached property.
    pub fn register_attached_with<TOwner: StaticType, THost: ObjectType, TValue: PropertyValue>(
        name: &str,
        options: StyledPropertyOptions<TValue>,
    ) -> &'static AttachedProperty<TValue> {
        let assign_binding = options.assign_binding;
        let (metadata, inherits, validate, notifying) = options.into_parts();
        let result = AttachedProperty::create(name, TOwner::TYPE, THost::TYPE, metadata, inherits, validate, notifying);
        result.set_assign_binding(assign_binding);
        result.host_type.set(Some(THost::TYPE));
        let registry = FerroPropertyRegistry::instance();
        registry.register(TOwner::TYPE, result);
        registry.register_attached(THost::TYPE, result);
        result
    }

    /// Registers a direct property: a property whose value is stored in a
    /// field of the owner and accessed through a getter and setter.
    pub fn register_direct<TOwner: ObjectType, TValue: PropertyValue>(
        name: &str,
        getter: fn(&TOwner) -> TValue,
        setter: Option<fn(&TOwner, TValue)>,
        unset_value: TValue,
    ) -> &'static DirectProperty<TOwner, TValue> {
        Self::register_direct_with(name, getter, setter, DirectPropertyMetadata::new(Some(unset_value)))
    }

    /// Registers a direct property with explicit metadata.
    pub fn register_direct_with<TOwner: ObjectType, TValue: PropertyValue>(
        name: &str,
        getter: fn(&TOwner) -> TValue,
        setter: Option<fn(&TOwner, TValue)>,
        metadata: DirectPropertyMetadata<TValue>,
    ) -> &'static DirectProperty<TOwner, TValue> {
        let result = DirectProperty::create(name, getter, setter, metadata);
        FerroPropertyRegistry::instance().register(TOwner::TYPE, result);
        result
    }
}

impl PartialEq for FerroProperty {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for FerroProperty {}

impl Hash for FerroProperty {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl fmt::Debug for FerroProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl fmt::Display for FerroProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

/// Options for registering a styled or attached property.
pub struct StyledPropertyOptions<T> {
    default_value: T,
    inherits: bool,
    default_binding_mode: BindingMode,
    validate: Option<Box<dyn Fn(&T) -> bool>>,
    coerce: Option<Rc<dyn Fn(&FerroObject, T) -> T>>,
    enable_data_validation: bool,
    assign_binding: bool,
    notifying: Option<NotifyingCallback>,
}

impl<T: PropertyValue> StyledPropertyOptions<T> {
    pub fn new(default_value: T) -> Self {
        Self {
            default_value,
            inherits: false,
            default_binding_mode: BindingMode::OneWay,
            validate: None,
            coerce: None,
            enable_data_validation: false,
            assign_binding: false,
            notifying: None,
        }
    }

    /// Whether the property inherits its value from the parent object.
    pub fn inherits(mut self, inherits: bool) -> Self {
        self.inherits = inherits;
        self
    }

    /// The default binding mode for the property.
    pub fn default_binding_mode(mut self, mode: BindingMode) -> Self {
        self.default_binding_mode = mode;
        self
    }

    /// A value validation callback. Setting a value for which it returns
    /// false is rejected.
    pub fn validate(mut self, validate: impl Fn(&T) -> bool + 'static) -> Self {
        self.validate = Some(Box::new(validate));
        self
    }

    /// A value coercion callback.
    pub fn coerce(mut self, coerce: impl Fn(&FerroObject, T) -> T + 'static) -> Self {
        self.coerce = Some(Rc::new(coerce));
        self
    }

    /// Whether the property is interested in data validation.
    pub fn enable_data_validation(mut self, enable: bool) -> Self {
        self.enable_data_validation = enable;
        self
    }

    /// Whether a binding given for the property in markup is assigned to the
    /// property instead of bound to it; see [`FerroProperty::assign_binding`].
    pub fn assign_binding(mut self, assign_binding: bool) -> Self {
        self.assign_binding = assign_binding;
        self
    }

    /// A callback invoked before and after change notifications are raised.
    pub fn notifying(mut self, notifying: impl Fn(&FerroObject, bool) + 'static) -> Self {
        self.notifying = Some(Box::new(notifying));
        self
    }

    #[allow(clippy::type_complexity)]
    fn into_parts(self) -> (StyledPropertyMetadata<T>, bool, Option<Box<dyn Fn(&T) -> bool>>, Option<NotifyingCallback>) {
        let mut metadata = StyledPropertyMetadata::new(Some(self.default_value))
            .with_default_binding_mode(self.default_binding_mode)
            .with_enable_data_validation(self.enable_data_validation);
        if let Some(coerce) = self.coerce {
            metadata = metadata.with_coerce(move |o, v| coerce(o, v));
        }
        (metadata, self.inherits, self.validate, self.notifying)
    }
}

/// Records the accessor `$name` of `$owner` returning `$property`
/// (`metadata::record_property_accessor`) with the `compiler-metadata`
/// feature of this crate; expands to nothing without it.
#[cfg(feature = "compiler-metadata")]
#[doc(hidden)]
#[macro_export]
macro_rules! __record_property_accessor {
    ($owner:ident, $name:ident, $property:expr) => {
        $crate::metadata::record_property_accessor(
            <$owner as $crate::StaticType>::TYPE,
            ::std::stringify!($name),
            $crate::Registrable::as_registered_property($property),
        )
    };
}

/// Records the accessor `$name` of `$owner` returning `$property` with the
/// `compiler-metadata` feature of this crate; expands to nothing without it.
#[cfg(not(feature = "compiler-metadata"))]
#[doc(hidden)]
#[macro_export]
macro_rules! __record_property_accessor {
    ($owner:ident, $name:ident, $property:expr) => {};
}

/// Declares the accessor of a single property definition.
///
/// ```ignore
/// impl Border {
///     ferro_property!(pub fn background_property() -> StyledProperty<Option<Ref<Brush>>> {
///         FerroProperty::register::<Border, _>("Background", None)
///     });
/// }
/// ```
///
/// The property is registered the first time the accessor is called on a
/// thread. Property definitions, like the objects that use them, belong to
/// the thread that created them.
///
/// On its own the macro registers nothing until the accessor is called, so a
/// lookup by name can miss the property. Declare the properties of a type in
/// a [`ferro_properties!`] block instead, which also registers them when the
/// type is initialised.
///
/// Where an accessor cannot be written inside the block (it is generated by
/// a local macro, or declared between other items), name the type whose
/// initialisation it belongs to and list it in the block of that type:
///
/// ```ignore
/// ferro_property!(for XYFocus; pub fn left_property() -> AttachedProperty<..> { .. });
///
/// ferro_properties! { impl XYFocus, also [XYFocus::left_property] {} }
/// ```
#[macro_export]
macro_rules! ferro_property {
    // An accessor that belongs to the static initialisation of `$owner`: the
    // first call on a thread first initialises the owner (as reading a
    // static field of a class runs its static initialisation), which
    // registers the properties of the owner in declaration order, this one
    // included.
    (for $owner:ident; $(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block) => {
        $(#[$meta])*
        #[inline]
        $vis fn $name() -> &'static $ty {
            ::std::thread_local! {
                static CELL: ::std::cell::Cell<::std::option::Option<&'static $ty>> =
                    const { ::std::cell::Cell::new(::std::option::Option::None) };
            }
            match CELL.get() {
                ::std::option::Option::Some(property) => property,
                ::std::option::Option::None => {
                    <$owner as $crate::StaticType>::TYPE.ensure_class_init();
                    match CELL.get() {
                        ::std::option::Option::Some(property) => property,
                        // Reached while the owner is being initialised (this
                        // is the registration in declaration order), or for
                        // an owner whose runtime type has no initialisation.
                        ::std::option::Option::None => {
                            let property: &'static $ty = $body;
                            CELL.set(::std::option::Option::Some(property));
                            $crate::__record_property_accessor!($owner, $name, property);
                            property
                        }
                    }
                }
            }
        }
    };

    ($(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block) => {
        $(#[$meta])*
        #[inline]
        $vis fn $name() -> &'static $ty {
            ::std::thread_local! {
                static CELL: ::std::cell::Cell<::std::option::Option<&'static $ty>> =
                    const { ::std::cell::Cell::new(::std::option::Option::None) };
            }
            match CELL.get() {
                ::std::option::Option::Some(property) => property,
                ::std::option::Option::None => {
                    let property: &'static $ty = $body;
                    CELL.set(::std::option::Option::Some(property));
                    property
                }
            }
        }
    };
}

/// Declares the property definitions of a type: the accessors, as
/// [`ferro_property!`] declares them one by one, and their registration as
/// part of the static initialisation of the type.
///
/// ```ignore
/// ferro_properties! {
///     impl Border {
///         /// Defines the `Background` property.
///         pub fn background_property() -> StyledProperty<Option<Ref<Brush>>> {
///             FerroProperty::register::<Border, _>("Background", None)
///         }
///
///         /// Defines the `Child` property (declared by another class).
///         pub fn child_property() -> StyledProperty<Option<Ref<Control>>> {
///             Decorator::child_property().add_owner::<Border>()
///         }
///     }
/// }
/// ```
///
/// All the properties are registered, in declaration order (the order of the
/// static field initialisers of the managed class), when the type is
/// initialised on a thread: when the first instance of the class or of a
/// class deriving from it is created, when one of the accessors is first
/// called, or when the property registry is asked about the type, whichever
/// comes first. A lookup by name therefore finds every property, whether or
/// not its accessor was called and whether or not an instance exists. After
/// that an accessor is a thread-local read, exactly as with
/// [`ferro_property!`].
///
/// The owner is a class ([`ferro_class!`](crate::ferro_class)) or a type
/// declared with [`ferro_static_type!`](crate::ferro_static_type).
///
/// Existing `ferro_property!(..);` invocations can be wrapped in the block
/// unchanged:
///
/// ```ignore
/// ferro_properties! { impl Border {
///     ferro_property!(pub fn background_property() -> StyledProperty<Option<Ref<Brush>>> {
///         FerroProperty::register::<Border, _>("Background", None)
///     });
/// } }
/// ```
///
/// A type has one such block. Properties of the type that are declared
/// elsewhere are named by it: `impl Owner, also [path, ..] { .. }` registers
/// the properties of the block first and then calls the listed functions in
/// order. A listed function is either
///
/// * the registration function of another block of accessors, declared with
///   `ferro_properties! { impl Owner, fn name { .. } }` (an upstream partial
///   class in a second file): `also [InputElement::register_gesture_properties]`;
/// * or a single accessor declared with `ferro_property!(for Owner; ..)`
///   (accessors generated by a local macro or interleaved with other items):
///   `also [XYFocus::left_property, XYFocus::right_property]`.
#[macro_export]
macro_rules! ferro_properties {
    (@accessors $owner:ident { $( $(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block )* }) => {
        $(
            $crate::ferro_property!(for $owner; $(#[$meta])* $vis fn $name() -> $ty $body);
        )*
    };

    // The main block, wrapping `ferro_property!(..);` invocations. The
    // accessors are declared through the macro name written at the call
    // site, so the block works with the import the file already has.
    (
        impl $owner:ident $(, also [$($more:path),* $(,)?])? {
            $( $macro_:ident ! ( $(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block ) ; )+
        }
    ) => {
        impl $owner {
            $( $macro_!(for $owner; $(#[$meta])* $vis fn $name() -> $ty $body); )+

            /// Registers the properties of the type in declaration order.
            /// Called by the static initialisation of the type.
            #[doc(hidden)]
            pub fn __register_properties() {
                $( let _ = <$owner>::$name(); )+
                $($( $more(); )*)?
            }
        }
    };

    // A part of the properties of a type, wrapping `ferro_property!(..);`.
    (
        impl $owner:ident, fn $part:ident {
            $( $macro_:ident ! ( $(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block ) ; )+
        }
    ) => {
        impl $owner {
            $( $macro_!(for $owner; $(#[$meta])* $vis fn $name() -> $ty $body); )+

            /// Registers a part of the properties of the type; named by its
            /// main `ferro_properties!` block.
            #[doc(hidden)]
            pub fn $part() {
                $( let _ = <$owner>::$name(); )+
            }
        }
    };

    // A part of the properties of a type.
    (
        impl $owner:ident, fn $part:ident {
            $( $(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block )*
        }
    ) => {
        impl $owner {
            $crate::ferro_properties!(@accessors $owner { $( $(#[$meta])* $vis fn $name() -> $ty $body )* });

            /// Registers a part of the properties of the type; named by its
            /// main `ferro_properties!` block.
            #[doc(hidden)]
            pub fn $part() {
                $( let _ = <$owner>::$name(); )*
            }
        }
    };

    // The main block.
    (
        impl $owner:ident $(, also [$($more:path),* $(,)?])? {
            $( $(#[$meta:meta])* $vis:vis fn $name:ident() -> $ty:ty $body:block )*
        }
    ) => {
        impl $owner {
            $crate::ferro_properties!(@accessors $owner { $( $(#[$meta])* $vis fn $name() -> $ty $body )* });

            /// Registers the properties of the type in declaration order.
            /// Called by the static initialisation of the type.
            #[doc(hidden)]
            pub fn __register_properties() {
                $( let _ = <$owner>::$name(); )*
                $($( $more(); )*)?
            }
        }
    };
}
