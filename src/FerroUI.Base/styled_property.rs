use crate::data::{BindingError, BindingNotification, BindingPriority, BindingValue, BindingValueType};
use crate::ferro_property::{
    convert_property_value, copy_converted_property_value, take_boxed_value, BoxedPayloadSlot, FerroPropertyInit,
    NotifyingCallback, PayloadSlot, PropertyRoutes, StyledRoutes,
};
use crate::ferro_property_metadata::MetadataTable;
use crate::property_store::EffectiveValueDyn;
use crate::reactive::IDisposable;
use crate::{
    BoxedValue, ObjectType, DoNothingType, FerroObject, FerroProperty, FerroPropertyRegistry, PropertyValue,
    StyledPropertyMetadata, TypeInfo, UnsetValueType,
};
use std::any::Any;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

/// A styled property: a property whose value is held by the property store
/// and can come from local values, styles, templates, animations, bindings and
/// inheritance, resolved by priority.
pub struct StyledProperty<T: PropertyValue> {
    base: FerroProperty,
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
    ) -> Self {
        let default_value = metadata.default_value().clone();
        if let Some(validate) = &validate {
            if !validate(&default_value) {
                panic!("default value is not valid for property '{name}'");
            }
        }
        Self {
            base: FerroProperty::new::<T>(
                name,
                owner_type,
                FerroPropertyInit { inherits, is_attached, is_direct: false, is_read_only: false },
                notifying,
            ),
            metadata: MetadataTable::new(host_type, metadata),
            single_default_value: RefCell::new(Some(default_value)),
            validate,
        }
    }

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
            name, owner_type, host_type, metadata, inherits, validate, notifying, is_attached,
        )));
        property.install_routes();
        property
    }

    /// Gives the base of the property its routes: the shared routes over
    /// this property, its metadata and the typed operations of `T`.
    pub(crate) fn install_routes(&'static self) {
        self.base.set_routes(PropertyRoutes::styled(&self.base, self, self.metadata.core(), Self::ROUTES));
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
            panic!("default value is not valid for property '{}'", self.name());
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

// The typed operations of the routes (`StyledRoutes`): what reads or writes
// the typed store. Everything else of a route is shared (`PropertyRoutes`).
impl<T: PropertyValue> StyledProperty<T> {
    const ROUTES: &'static StyledRoutes = &StyledRoutes {
        create_effective_value: Self::route_create_effective_value,
        coerce_default_value: Self::route_coerce_default_value,
        get_base_value: Self::route_get_base_value,
        get_default_value: Self::route_get_default_value,
        set_value: Self::route_set_value,
        set_current_value: Self::route_set_current_value,
        set_local_value: Self::route_set_local_value,
        is_valid_value: Self::route_is_valid_value,
    };

    /// The property of a route.
    #[inline]
    fn of_routes(typed: &'static dyn Any) -> &'static Self {
        debug_assert!(typed.is::<Self>());
        // SAFETY: the routes of a styled property are made only by
        // `install_routes`, with the property itself as their typed view, and
        // a property holds the routes of its own value type.
        unsafe { &*(typed as *const dyn Any as *const Self) }
    }

    /// The value of a route that writes: a copy of a value of the value type,
    /// or the conversion of another value; panics for any other value.
    fn route_value(&self, value: &dyn Any) -> T {
        match value.downcast_ref::<T>() {
            Some(value) => value.clone(),
            None => match convert_property_value::<T>(self, value) {
                Some(value) => value,
                None => self.invalid_value_type(),
            },
        }
    }

    fn route_create_effective_value(typed: &'static dyn Any, o: &FerroObject) -> Rc<dyn EffectiveValueDyn> {
        let this = Self::of_routes(typed);
        o.values().create_effective_value(o, this)
    }

    fn route_coerce_default_value(typed: &'static dyn Any, o: &FerroObject) {
        o.values().coerce_default_value(o, Self::of_routes(typed));
    }

    fn route_get_base_value(typed: &'static dyn Any, o: &FerroObject) -> Option<BoxedValue> {
        o.get_base_value(Self::of_routes(typed)).map(|value| Rc::new(value) as BoxedValue)
    }

    fn route_get_default_value(typed: &'static dyn Any, type_: &'static TypeInfo) -> BoxedValue {
        Rc::new(Self::of_routes(typed).get_default_value(type_))
    }

    fn route_set_value(
        typed: &'static dyn Any,
        o: &FerroObject,
        value: &dyn Any,
        priority: BindingPriority,
    ) -> Option<Rc<dyn IDisposable>> {
        let this = Self::of_routes(typed);
        o.set_value_with_priority(this, this.route_value(value), priority)
    }

    fn route_set_current_value(typed: &'static dyn Any, o: &FerroObject, value: &dyn Any) {
        let this = Self::of_routes(typed);
        o.set_current_value(this, this.route_value(value));
    }

    fn route_set_local_value(typed: &'static dyn Any, o: &FerroObject, value: BoxedValue) {
        if let Some(value) = take_boxed_value::<T>(value) {
            o.values().set_local_value(o, Self::of_routes(typed), value);
        }
    }

    fn route_is_valid_value(typed: &'static dyn Any, value: &dyn Any) -> bool {
        match value.downcast_ref::<T>() {
            Some(v) => Self::of_routes(typed).is_valid(v),
            None => false,
        }
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

impl<T: PropertyValue> AttachedProperty<T> {
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
            base: StyledProperty::new(name, owner_type, host_type, metadata, inherits, validate, notifying, true),
        }));
        property.base.install_routes();
        property
    }

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
