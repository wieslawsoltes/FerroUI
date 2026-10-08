use crate::data::{BindingPriority, BindingValue};
use crate::ferro_property::{FerroPropertyInit, NotifyingCallback, PropertyRoutes};
use crate::ferro_property_metadata::MetadataTable;
use crate::property_store::{EffectiveValueDyn, IValueEntry};
use crate::reactive::{IDisposable, IObservable};
use crate::{
    BoxedValue, ObjectType, DoNothingType, FerroObject, FerroProperty, FerroPropertyMetadata,
    FerroPropertyRegistry, PropertyValue, StyledPropertyMetadata, TypeInfo, UnsetValueType,
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
        property.base.set_routes(property);
        property
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
        if let Some(value) = &*self.single_default_value.borrow() {
            return value.clone();
        }
        self.get_metadata(type_).default_value().clone()
    }

    /// Gets the default value for the property on the type of `owner`.
    #[inline]
    pub fn get_default_value_for(&self, owner: &FerroObject) -> T {
        self.get_default_value(owner.get_type())
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

    /// Converts an untyped value of another type to the value type of the
    /// property with the implicit conversions of the managed original
    /// (the `TryConvert` of the property, see
    /// [`ValueTypes::try_convert_implicit`](crate::data::core::ValueTypes::try_convert_implicit)).
    fn convert_untyped(value: &dyn Any) -> Option<T> {
        let converted = crate::data::core::ValueTypes::try_convert_implicit(value, std::any::TypeId::of::<T>())?;
        let converted: &dyn crate::AnyValue = &*converted;
        converted.downcast_ref::<T>().cloned()
    }

    /// Interprets an untyped value: a value of type `T`, or one of the unset /
    /// do-nothing markers.
    pub(crate) fn from_untyped(&self, value: &dyn Any) -> BindingValue<T> {
        if let Some(v) = value.downcast_ref::<T>() {
            BindingValue::new(v.clone())
        } else if value.is::<UnsetValueType>() {
            BindingValue::unset()
        } else if value.is::<DoNothingType>() {
            BindingValue::do_nothing()
        } else if let Some(v) = value.downcast_ref::<BindingValue<T>>() {
            v.clone()
        } else if let Some(n) = value.downcast_ref::<crate::data::BindingNotification>() {
            n.to_binding_value(self.property_type_name())
        } else if let Some(v) = Self::convert_untyped(value) {
            BindingValue::new(v)
        } else {
            BindingValue::binding_error(crate::data::BindingError::message(self.invalid_value_message()))
        }
    }
}

impl<T: PropertyValue> PropertyRoutes for StyledProperty<T> {
    fn create_effective_value(&'static self, o: &FerroObject) -> Rc<dyn EffectiveValueDyn> {
        let this = self;
        o.values().create_effective_value(o, this)
    }

    fn route_clear_value(&'static self, o: &FerroObject) {
        o.clear_value(self);
    }

    fn route_coerce_default_value(&'static self, o: &FerroObject) {
        o.values().coerce_default_value(o, self);
    }

    fn route_get_value(&'static self, o: &FerroObject) -> BoxedValue {
        Rc::new(o.get_value(self))
    }

    fn route_get_base_value(&'static self, o: &FerroObject) -> BoxedValue {
        match o.get_base_value(self) {
            Some(v) => Rc::new(v),
            None => FerroProperty::unset_value(),
        }
    }

    fn route_get_default_value(&'static self, type_: &'static TypeInfo) -> BoxedValue {
        Rc::new(self.get_default_value(type_))
    }

    fn route_set_value(
        &'static self,
        o: &FerroObject,
        value: &dyn Any,
        priority: BindingPriority,
    ) -> Option<Rc<dyn IDisposable>> {
        let this = self;
        if value.is::<UnsetValueType>() {
            if priority == BindingPriority::LocalValue {
                o.clear_value(this);
            }
            None
        } else if value.is::<DoNothingType>() {
            None
        } else if let Some(v) = value.downcast_ref::<T>() {
            o.set_value_with_priority(this, v.clone(), priority)
        } else if let Some(v) = Self::convert_untyped(value) {
            o.set_value_with_priority(this, v, priority)
        } else {
            self.invalid_value_type();
        }
    }

    fn route_set_current_value(&'static self, o: &FerroObject, value: &dyn Any) {
        let this = self;
        if value.is::<UnsetValueType>() {
            o.clear_value(this);
        } else if value.is::<DoNothingType>() {
        } else if let Some(v) = value.downcast_ref::<T>() {
            o.set_current_value(this, v.clone());
        } else if let Some(v) = Self::convert_untyped(value) {
            o.set_current_value(this, v);
        } else {
            self.invalid_value_type();
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
        o.bind_untyped(self, source, priority)
    }

    fn route_get_metadata(&'static self, type_: &'static TypeInfo) -> FerroPropertyMetadata {
        let metadata = self.get_metadata(type_);
        let base: &FerroPropertyMetadata = &metadata;
        base.clone()
    }

    fn route_unregister(&'static self, type_: &'static TypeInfo) {
        self.unregister(type_)
    }

    fn route_is_valid_value(&'static self, value: &dyn Any) -> bool {
        match value.downcast_ref::<T>() {
            Some(v) => self.is_valid(v),
            None => false,
        }
    }

    fn as_any(&'static self) -> &'static dyn Any {
        self
    }

    fn route_copy_value(&'static self, value: &dyn Any, out: &mut dyn Any) -> bool {
        match (value.downcast_ref::<T>(), out.downcast_mut::<Option<T>>()) {
            (Some(value), Some(out)) => {
                *out = Some(value.clone());
                true
            }
            _ => false,
        }
    }

    fn route_create_binding_entry(
        &'static self,
        o: &FerroObject,
        frame: std::rc::Weak<dyn crate::property_store::ValueFrame>,
        source: Rc<dyn IObservable<BoxedValue>>,
    ) -> Rc<dyn IValueEntry> {
        crate::property_store::BindingEntry::new(
            o,
            frame,
            self,
            crate::property_store::BindingSource::Untyped(source),
        )
    }
}

impl<T: PropertyValue> StyledProperty<T> {
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
        property.base.base.set_routes(&property.base);
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
