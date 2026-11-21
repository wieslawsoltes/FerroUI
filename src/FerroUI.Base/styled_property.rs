use crate::data::{BindingError, BindingNotification, BindingPriority, BindingValue, BindingValueType};
use crate::ferro_property::{
    copy_converted_property_value, take_boxed_value, unpack_binding_value, BoxedPayloadSlot,
    FerroPropertyInit, NotifyingCallback, PayloadSlot, PropertyRoutes,
};
use crate::ferro_property_metadata::MetadataTable;
use crate::property_store::{EffectiveValueDyn, IValueEntry};
use crate::reactive::{IDisposable, IObservable};
use crate::{
    BoxedValue, ObjectType, DoNothingType, FerroObject, FerroProperty, FerroPropertyMetadata,
    FerroPropertyRegistry, PropertyValue, StyledPropertyMetadata, StyledValue, TypeInfo, UnsetValueType,
};
use std::any::{Any, TypeId};
use std::cell::{OnceCell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// A styled property: a property whose value is held by the property store
/// and can come from local values, styles, templates, animations, bindings and
/// inheritance, resolved by priority.
pub struct StyledProperty<T: PropertyValue> {
    base: FerroProperty,
    /// The routes of the property, shared code for every value type.
    routes: StyledRoutes,
    metadata: MetadataTable<StyledPropertyMetadata<T>>,
    /// For performance, the default value is cached while there is only one,
    /// avoiding a metadata lookup that may walk the class hierarchy.
    single_default_value: RefCell<Option<T>>,
    validate: Option<Box<dyn Fn(&T) -> bool>>,
}

impl<T: PropertyValue> Deref for StyledProperty<T> {
    type Target = FerroProperty;
    #[inline]
    fn deref(&self) -> &FerroProperty {
        &self.base
    }
}

impl<T: PropertyValue> StyledProperty<T> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        name: &str,
        owner_type: &'static TypeInfo,
        host_type: &'static TypeInfo,
        metadata: StyledPropertyMetadata<T>,
        inherits: bool,
        validate: Option<Box<dyn Fn(&T) -> bool>>,
        notifying: Option<NotifyingCallback>,
        is_attached: bool,
        members: &'static [(i64, &'static dyn Any)],
    ) -> Self {
        let default_value = metadata.default_value().clone();
        if let Some(validate) = &validate {
            if !validate(&default_value) {
                default_not_valid(name);
            }
        }
        Self {
            base: FerroProperty::new::<T>(
                name,
                owner_type,
                FerroPropertyInit { inherits, is_attached, is_direct: false, is_read_only: false },
                notifying,
            )
            .with_members(members)
            .with_validation(validate.is_some()),
            routes: StyledRoutes::new(),
            metadata: MetadataTable::new(host_type, metadata),
            single_default_value: RefCell::new(Some(default_value)),
            validate,
        }
    }

    /// The untyped base of the property.
    #[inline]
    pub fn as_property(&'static self) -> &'static FerroProperty {
        &self.base
    }

    /// The value validation callback for the property.
    #[inline]
    pub fn validate_value(&self) -> Option<&dyn Fn(&T) -> bool> {
        self.validate.as_deref()
    }

    #[inline]
    pub(crate) fn is_valid(&self, value: &T) -> bool {
        match &self.validate {
            Some(validate) => validate(value),
            None => true,
        }
    }

    /// Registers the property on another class.
    pub fn add_owner<TOwner: ObjectType>(&'static self) -> &'static Self {
        FerroPropertyRegistry::instance().register(TOwner::TYPE, self);
        self
    }

    /// Registers the property on another class, with metadata for that class.
    pub fn add_owner_with<TOwner: ObjectType>(&'static self, metadata: StyledPropertyMetadata<T>) -> &'static Self {
        FerroPropertyRegistry::instance().register(TOwner::TYPE, self);
        self.override_metadata::<TOwner>(metadata);
        self
    }

    /// Coerces `base_value` with the coercion callback registered for the
    /// type of `instance`, if any.
    pub fn coerce_value(&self, instance: &FerroObject, base_value: T) -> T {
        let metadata = self.get_metadata_for(instance);
        match metadata.coerce_value() {
            Some(coerce) => coerce(instance, base_value),
            None => base_value,
        }
    }

    /// Gets the default value for the property on the specified type.
    #[inline]
    pub fn get_default_value(&self, type_: &'static TypeInfo) -> T {
        self.default_for(|| type_)
    }

    /// Gets the default value for the property on the type of `owner`.
    #[inline]
    pub fn get_default_value_for(&self, owner: &FerroObject) -> T {
        self.default_for(|| owner.get_type())
    }

    #[inline]
    fn default_for(&self, host: impl FnOnce() -> &'static TypeInfo) -> T {
        if let Some(value) = &*self.single_default_value.borrow() {
            return value.clone();
        }
        self.get_metadata(host()).default_value().clone()
    }

    /// A binding keeps the first fallback it observes for its owner, even if
    /// the property's metadata is subsequently overridden.
    pub(crate) fn binding_default(&self, owner: &FerroObject, cache: &RefCell<Option<T>>) -> T {
        if let Some(value) = &*cache.borrow() {
            return value.clone();
        }
        let value = self.get_default_value_for(owner);
        *cache.borrow_mut() = Some(value.clone());
        value
    }

    /// Gets the property metadata for the specified type.
    #[inline]
    pub fn get_metadata(&self, type_: &'static TypeInfo) -> Rc<StyledPropertyMetadata<T>> {
        self.metadata.get(type_)
    }

    /// Gets the property metadata for the type of `owner`.
    #[inline]
    pub fn get_metadata_for(&self, owner: &FerroObject) -> Rc<StyledPropertyMetadata<T>> {
        self.metadata.get(owner.get_type())
    }

    /// Overrides the default value for the property on class `TOwner`.
    pub fn override_default_value<TOwner: ObjectType>(&self, default_value: T) {
        self.override_default_value_for(TOwner::TYPE, default_value);
    }

    /// Overrides the default value for the property on the specified type.
    pub fn override_default_value_for(&self, type_: &'static TypeInfo, default_value: T) {
        self.override_metadata_for(type_, StyledPropertyMetadata::new(Some(default_value)));
    }

    /// Overrides the metadata for the property on class `TOwner`.
    pub fn override_metadata<TOwner: ObjectType>(&self, metadata: StyledPropertyMetadata<T>) {
        self.override_metadata_for(TOwner::TYPE, metadata)
    }

    /// Overrides the metadata for the property on the specified type.
    pub fn override_metadata_for(&self, type_: &'static TypeInfo, metadata: StyledPropertyMetadata<T>) {
        if metadata.has_default_value() && !self.is_valid(metadata.default_value()) {
            default_not_valid(self.name());
        }
        let new_default = metadata.has_default_value().then(|| metadata.default_value().clone());
        self.metadata.override_metadata(self.name(), type_, metadata);
        let mut single = self.single_default_value.borrow_mut();
        if let Some(new_default) = new_default {
            if single.as_ref() != Some(&new_default) {
                *single = None;
            }
        }
    }

    /// Removes the metadata registered for the specified type.
    pub fn unregister(&self, type_: &'static TypeInfo) {
        self.metadata.unregister(type_);
    }
}

// The failures of the registration of a property, compiled once instead of once for each value
// type.

#[cold]
#[inline(never)]
fn default_not_valid(name: &str) -> ! {
    panic!("default value is not valid for property '{name}'")
}

#[cold]
#[inline(never)]
fn routes_already_set() -> ! {
    panic!("property routes are already set")
}

/// Interprets a value of an untyped binding of a styled property: a value of
/// the property's value type, a `BindingValue` of it, the unset and do-nothing
/// markers, a binding notification, or a value that converts implicitly, in
/// that order. The value stays boxed: a value of the property's value type is
/// the box that was given, and any other value is copied into a box of its
/// own, which then holds exactly the property's value type. Compiled once; the
/// typed operations are the routes of the property.
pub(crate) fn resolve_boxed_binding_value(property: &FerroProperty, value: &BoxedValue) -> BindingValue<BoxedValue> {
    if value.value_type_id() == property.property_type() {
        BindingValue::new(value.clone())
    } else if let Some(value) = property.routes().route_unpack_binding_value(value.as_any()) {
        value
    } else {
        let mut slot = BoxedPayloadSlot::new(property);
        let (kind, error) = resolve_untyped_binding(property, value.as_any(), &mut slot);
        BindingValue::from_notification_parts(kind, slot.into_value(), error)
    }
}

// The marker and conversion policies are common, with the existing
// notification snapshot scope retained around the payload copy.
#[inline(never)]
fn resolve_untyped_binding(
    property: &FerroProperty,
    value: &dyn Any,
    copy_value: &mut dyn PayloadSlot,
) -> (BindingValueType, Option<BindingError>) {
    if value.is::<UnsetValueType>() {
        return (BindingValueType::UNSET_VALUE, None);
    }
    if value.is::<DoNothingType>() {
        return (BindingValueType::DO_NOTHING, None);
    }
    if let Some(notification) = value.downcast_ref::<BindingNotification>() {
        return notification.resolve_binding_value(property.property_type_name(), copy_value);
    }
    if copy_converted_property_value(property, value, copy_value) {
        (BindingValueType::VALUE, None)
    } else {
        (BindingValueType::BINDING_ERROR, Some(BindingError::message(property.invalid_value_message())))
    }
}

impl<T: StyledValue> StyledProperty<T> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn create(
        name: &str,
        owner_type: &'static TypeInfo,
        host_type: &'static TypeInfo,
        metadata: StyledPropertyMetadata<T>,
        inherits: bool,
        validate: Option<Box<dyn Fn(&T) -> bool>>,
        notifying: Option<NotifyingCallback>,
        is_attached: bool,
    ) -> &'static Self {
        let property: &'static Self = Box::leak(Box::new(Self::new(
            name, owner_type, host_type, metadata, inherits, validate, notifying, is_attached, T::MEMBERS,
        )));
        property.install_routes();
        property
    }

    /// Gives the property its routes and its codec, once it has its address.
    #[inline]
    pub(crate) fn install_routes(&'static self) {
        install_styled_routes(&self.base, &self.routes, self);
    }
}

/// [`StyledProperty::install_routes`], compiled once.
#[inline(never)]
fn install_styled_routes(base: &'static FerroProperty, routes: &'static StyledRoutes, codec: &'static dyn StyledCodec) {
    if routes.property.set(base).is_err() {
        routes_already_set();
    }
    base.set_styled_codec(codec);
    base.set_routes(routes);
}

/// A stored value of a property whose value type is `T`, as a `T`.
#[inline]
fn stored_value<T: StyledValue>(value: &dyn Any) -> T {
    match value.downcast_ref::<T::Storage>() {
        Some(value) => T::from_storage(value.clone()),
        None => panic!("a stored value of a property is not of its storage type"),
    }
}

/// The operations of the routes of a styled property that depend on its value type: implemented
/// by `StyledProperty<T>`, one table for each value type. Everything else the routes do is in
/// [`StyledRoutes`], compiled once (#997).
pub(crate) trait StyledCodec {
    // The untyped write and read of a value of the value type are typed, as the routes of each
    // value type were: a shared route cost an untyped write or read of an enumeration 7 to 8 %
    // more instructions (docs/binary-size.md, section 4.27).

    /// The write of `value`, a value of the value type, with `priority`; `None` for a value of
    /// another type.
    fn set_value(
        &'static self,
        o: &FerroObject,
        value: &dyn Any,
        priority: BindingPriority,
    ) -> Option<Option<Rc<dyn IDisposable>>>;
    /// The current value from `value`, a value of the value type; whether it was one.
    fn set_current_value(&'static self, o: &FerroObject, value: &dyn Any) -> bool;
    /// The value of the property on `o` in a box of its own.
    fn get_value_boxed(&'static self, o: &FerroObject) -> BoxedValue;
    /// The local value from a box of a value of the value type (the value of a binding).
    fn set_local_boxed(&'static self, o: &FerroObject, value: BoxedValue);
    /// A new effective value of the property on `o`.
    fn create_effective_value(&'static self, o: &FerroObject) -> Rc<dyn EffectiveValueDyn>;
    /// Coerces the default value of the property on `o`.
    fn coerce_default_value(&'static self, o: &FerroObject);
    /// The default value of the property on `type_`, boxed.
    fn default_value_boxed(&'static self, type_: &'static TypeInfo) -> BoxedValue;
    /// The base metadata of the property on `type_`.
    fn metadata(&'static self, type_: &'static TypeInfo) -> FerroPropertyMetadata;
    /// Removes the metadata of the property for `type_`.
    fn unregister(&'static self, type_: &'static TypeInfo);
    /// Whether `value` is of the value type and accepted by the validation of the property.
    fn is_valid_value(&'static self, value: &dyn Any) -> bool;
    /// [`PropertyRoutes::route_copy_value`].
    fn copy_value(&'static self, value: &dyn Any, out: &mut dyn Any) -> bool;
    /// [`PropertyRoutes::route_box_value`].
    fn box_value(&'static self, value: &dyn Any) -> Option<BoxedValue>;
    /// [`PropertyRoutes::route_with_values`].
    fn with_values(
        &'static self,
        old: Option<&dyn Any>,
        new: &dyn Any,
        f: &mut dyn FnMut(Option<&dyn Any>, &dyn Any),
    );
    /// [`PropertyRoutes::route_enable_data_validation`].
    fn enable_data_validation(&'static self, owner: &FerroObject) -> Option<bool>;
    /// [`PropertyRoutes::route_unpack_binding_value`].
    fn unpack_binding_value(&'static self, value: &dyn Any) -> Option<BindingValue<BoxedValue>>;
    /// The typed property, for downcasting.
    fn typed(&'static self) -> &'static dyn Any;
}

impl<T: StyledValue> StyledCodec for StyledProperty<T> {
    fn set_value(
        &'static self,
        o: &FerroObject,
        value: &dyn Any,
        priority: BindingPriority,
    ) -> Option<Option<Rc<dyn IDisposable>>> {
        let value = value.downcast_ref::<T>()?;
        Some(o.set_value_with_priority(self, value.clone(), priority))
    }

    fn set_current_value(&'static self, o: &FerroObject, value: &dyn Any) -> bool {
        match value.downcast_ref::<T>() {
            Some(value) => {
                o.set_current_value(self, value.clone());
                true
            }
            None => false,
        }
    }

    fn get_value_boxed(&'static self, o: &FerroObject) -> BoxedValue {
        Rc::new(o.get_value(self))
    }

    fn set_local_boxed(&'static self, o: &FerroObject, value: BoxedValue) {
        if let Some(value) = take_boxed_value::<T>(value) {
            o.values().set_local_value(o, self, value);
        }
    }

    fn create_effective_value(&'static self, o: &FerroObject) -> Rc<dyn EffectiveValueDyn> {
        o.values().create_effective_value(o, self)
    }

    fn coerce_default_value(&'static self, o: &FerroObject) {
        o.values().coerce_default_value(o, self);
    }

    fn default_value_boxed(&'static self, type_: &'static TypeInfo) -> BoxedValue {
        Rc::new(self.get_default_value(type_))
    }

    fn metadata(&'static self, type_: &'static TypeInfo) -> FerroPropertyMetadata {
        let metadata = self.get_metadata(type_);
        let base: &FerroPropertyMetadata = &metadata;
        base.clone()
    }

    fn unregister(&'static self, type_: &'static TypeInfo) {
        StyledProperty::unregister(self, type_)
    }

    fn is_valid_value(&'static self, value: &dyn Any) -> bool {
        match value.downcast_ref::<T>() {
            Some(v) => self.is_valid(v),
            None => false,
        }
    }

    fn copy_value(&'static self, value: &dyn Any, out: &mut dyn Any) -> bool {
        match (value.downcast_ref::<T>(), out.downcast_mut::<Option<T::Storage>>()) {
            (Some(value), Some(out)) => {
                *out = Some(value.clone().into_storage());
                true
            }
            _ => false,
        }
    }

    fn box_value(&'static self, value: &dyn Any) -> Option<BoxedValue> {
        value.downcast_ref::<T>().map(|value| Rc::new(value.clone()) as BoxedValue)
    }

    fn with_values(
        &'static self,
        old: Option<&dyn Any>,
        new: &dyn Any,
        f: &mut dyn FnMut(Option<&dyn Any>, &dyn Any),
    ) {
        // A type that is its own storage type is never asked (`with_values`): its codec stays the
        // one line it needs (decided when the function is compiled).
        if !T::Storage::SHARED_STORAGE {
            return f(old, new);
        }
        let new = stored_value::<T>(new);
        match old {
            Some(old) => {
                let old = stored_value::<T>(old);
                f(Some(&old), &new);
            }
            None => f(None, &new),
        }
    }

    fn enable_data_validation(&'static self, owner: &FerroObject) -> Option<bool> {
        self.get_metadata_for(owner).enable_data_validation()
    }

    fn unpack_binding_value(&'static self, value: &dyn Any) -> Option<BindingValue<BoxedValue>> {
        unpack_binding_value::<T>(value)
    }

    fn typed(&'static self) -> &'static dyn Any {
        self
    }
}

/// The routes of a styled (or attached) property, the same code for every value type: the
/// untyped paths read and write the store through its untyped interface (the effective values
/// and their entries) and ask the codec of the property ([`StyledCodec`]) only for what depends
/// on the value type (#997). Kept in the property, which it names.
pub(crate) struct StyledRoutes {
    property: OnceCell<&'static FerroProperty>,
}

impl StyledRoutes {
    fn new() -> Self {
        Self { property: OnceCell::new() }
    }

    #[inline]
    fn property(&self) -> &'static FerroProperty {
        self.property.get().expect("the routes of a styled property know it")
    }

    #[inline]
    fn codec(&self) -> &'static dyn StyledCodec {
        self.property().styled_codec()
    }
}

impl StyledRoutes {
    /// `value`, of another type, converted to the value type of the property (`TryConvert` of
    /// the property) and copied into a box of its own: the temporary of the conversion is released
    /// before the value is validated, coerced or published, as the typed routes released it.
    /// Panics as they did for a value that does not convert.
    fn converted(&self, value: &dyn Any) -> BoxedValue {
        converted_value(self.property(), self.codec(), value)
    }
}

impl PropertyRoutes for StyledRoutes {
    fn create_effective_value(&'static self, o: &FerroObject) -> Rc<dyn EffectiveValueDyn> {
        self.codec().create_effective_value(o)
    }

    fn route_clear_value(&'static self, o: &FerroObject) {
        o.values().clear_value(o, self.property());
    }

    fn route_coerce_default_value(&'static self, o: &FerroObject) {
        self.codec().coerce_default_value(o);
    }

    fn route_get_value(&'static self, o: &FerroObject) -> BoxedValue {
        self.codec().get_value_boxed(o)
    }

    fn route_get_base_value(&'static self, o: &FerroObject) -> BoxedValue {
        match o.values().get_effective_value(self.property()).and_then(|effective| effective.boxed_base_value()) {
            Some(value) => value,
            None => FerroProperty::unset_value(),
        }
    }

    fn route_get_default_value(&'static self, type_: &'static TypeInfo) -> BoxedValue {
        self.codec().default_value_boxed(type_)
    }

    fn route_set_value(
        &'static self,
        o: &FerroObject,
        value: &dyn Any,
        priority: BindingPriority,
    ) -> Option<Rc<dyn IDisposable>> {
        set_styled_value(self.property(), self.codec(), o, value, priority)
    }

    fn route_set_current_value(&'static self, o: &FerroObject, value: &dyn Any) {
        let property = self.property();
        let type_id = value.type_id();
        if type_id != property.property_type() {
            if type_id == TypeId::of::<UnsetValueType>() {
                o.values().clear_value(o, property);
                return;
            }
            if type_id == TypeId::of::<DoNothingType>() {
                return;
            }
            let converted = self.converted(value);
            return self.route_set_current_value(o, converted.as_any());
        }
        if !self.codec().set_current_value(o, value) {
            property.invalid_value_type();
        }
    }

    fn route_set_direct_value_unchecked(&'static self, _o: &FerroObject, _entry: &dyn IValueEntry) {
        panic!("not supported for styled properties");
    }

    fn route_bind(
        &'static self,
        o: &FerroObject,
        source: Rc<dyn IObservable<BoxedValue>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        FerroObject::validate_priority(priority);
        o.values().add_boxed_binding(o, self.property(), source, priority)
    }

    fn route_get_metadata(&'static self, type_: &'static TypeInfo) -> FerroPropertyMetadata {
        self.codec().metadata(type_)
    }

    fn route_unregister(&'static self, type_: &'static TypeInfo) {
        self.codec().unregister(type_)
    }

    fn route_is_valid_value(&'static self, value: &dyn Any) -> bool {
        self.codec().is_valid_value(value)
    }

    fn as_any(&'static self) -> &'static dyn Any {
        self.codec().typed()
    }

    fn route_copy_value(&'static self, value: &dyn Any, out: &mut dyn Any) -> bool {
        self.codec().copy_value(value, out)
    }

    fn route_with_values(
        &'static self,
        old: Option<&dyn Any>,
        new: &dyn Any,
        f: &mut dyn FnMut(Option<&dyn Any>, &dyn Any),
    ) {
        self.codec().with_values(old, new, f)
    }

    fn route_box_value(&'static self, value: &dyn Any) -> Option<BoxedValue> {
        self.codec().box_value(value)
    }

    fn route_enable_data_validation(&'static self, owner: &FerroObject) -> Option<bool> {
        self.codec().enable_data_validation(owner)
    }

    fn route_unpack_binding_value(&'static self, value: &dyn Any) -> Option<BindingValue<BoxedValue>> {
        self.codec().unpack_binding_value(value)
    }

    fn route_set_local_value(&'static self, o: &FerroObject, value: BoxedValue) {
        self.codec().set_local_boxed(o, value);
    }
}

/// The untyped write of a styled property: the route, and `FerroObject::set_value_untyped`
/// directly (one call through a vtable, the codec's, as the typed route of a value type was).
#[inline]
pub(crate) fn set_styled_value(
    property: &'static FerroProperty,
    codec: &'static dyn StyledCodec,
    o: &FerroObject,
    value: &dyn Any,
    priority: BindingPriority,
) -> Option<Rc<dyn IDisposable>> {
    // The type of the value, asked once: a value of the value type is the common case.
    if value.type_id() == property.property_type() {
        return match codec.set_value(o, value, priority) {
            Some(result) => result,
            None => property.invalid_value_type(),
        };
    }
    set_other_styled_value(property, codec, o, value, priority)
}

/// [`set_styled_value`] of a marker or of a value of another type, which is converted.
#[inline(never)]
fn set_other_styled_value(
    property: &'static FerroProperty,
    codec: &'static dyn StyledCodec,
    o: &FerroObject,
    value: &dyn Any,
    priority: BindingPriority,
) -> Option<Rc<dyn IDisposable>> {
    let type_id = value.type_id();
    if type_id == TypeId::of::<UnsetValueType>() {
        if priority == BindingPriority::LocalValue {
            o.values().clear_value(o, property);
        }
        return None;
    }
    if type_id == TypeId::of::<DoNothingType>() {
        return None;
    }
    let converted = converted_value(property, codec, value);
    set_styled_value(property, codec, o, converted.as_any(), priority)
}

/// [`StyledRoutes::converted`] of `property`.
fn converted_value(property: &'static FerroProperty, codec: &'static dyn StyledCodec, value: &dyn Any) -> BoxedValue {
    let converted = crate::data::core::ValueTypes::try_convert_implicit(value, property.property_type());
    let copy = converted.as_ref().and_then(|converted| codec.box_value(converted.as_any()));
    drop(converted);
    match copy {
        Some(copy) => copy,
        None => property.invalid_value_type(),
    }
}

/// An attached property: a styled property that is defined by one class and
/// set on objects of another.
pub struct AttachedProperty<T: PropertyValue> {
    base: StyledProperty<T>,
}

impl<T: PropertyValue> Deref for AttachedProperty<T> {
    type Target = StyledProperty<T>;
    #[inline]
    fn deref(&self) -> &StyledProperty<T> {
        &self.base
    }
}

impl<T: StyledValue> AttachedProperty<T> {
    pub(crate) fn create(
        name: &str,
        owner_type: &'static TypeInfo,
        host_type: &'static TypeInfo,
        metadata: StyledPropertyMetadata<T>,
        inherits: bool,
        validate: Option<Box<dyn Fn(&T) -> bool>>,
        notifying: Option<NotifyingCallback>,
    ) -> &'static Self {
        let property: &'static Self = Box::leak(Box::new(Self {
            base: StyledProperty::new(
                name, owner_type, host_type, metadata, inherits, validate, notifying, true, T::MEMBERS,
            ),
        }));
        property.base.install_routes();
        property
    }
}

impl<T: PropertyValue> AttachedProperty<T> {
    /// Attaches the property as a non-attached property on class `TOwner`.
    pub fn add_owner<TOwner: ObjectType>(&'static self) -> &'static Self {
        FerroPropertyRegistry::instance().register(TOwner::TYPE, &self.base);
        self
    }

    /// Attaches the property as a non-attached property on class `TOwner`,
    /// with metadata for that class.
    pub fn add_owner_with<TOwner: ObjectType>(&'static self, metadata: StyledPropertyMetadata<T>) -> &'static Self {
        FerroPropertyRegistry::instance().register(TOwner::TYPE, &self.base);
        self.base.override_metadata::<TOwner>(metadata);
        self
    }
}
