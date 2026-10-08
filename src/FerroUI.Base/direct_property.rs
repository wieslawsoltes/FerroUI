use crate::data::{BindingPriority, BindingValue};
use crate::ferro_property::{FerroPropertyInit, PropertyRoutes};
use crate::ferro_property_metadata::MetadataTable;
use crate::property_store::{EffectiveValueDyn, IValueEntry};
use crate::reactive::{IDisposable, IObservable};
use crate::{
    BoxedValue, ObjectType, DirectPropertyMetadata, DoNothingType, FerroObject, FerroProperty,
    FerroPropertyMetadata, FerroPropertyRegistry, PropertyValue, TypeInfo, UnsetValueType,
};
use std::any::Any;
use std::cell::OnceCell;
use std::ops::Deref;
use std::rc::Rc;

/// The owner-specific accessors of a direct property.
pub(crate) trait DirectPropertyAccessor<T> {
    fn invoke_getter(&self, instance: &FerroObject) -> T;
    fn invoke_setter(&self, instance: &FerroObject, value: BindingValue<T>);
}

/// Base class for direct properties.
///
/// Holds everything about a direct property that does not depend on the type
/// of its owner.
pub struct DirectPropertyBase<T: PropertyValue> {
    base: FerroProperty,
    owner: &'static TypeInfo,
    metadata: MetadataTable<DirectPropertyMetadata<T>>,
    accessor: OnceCell<&'static dyn DirectPropertyAccessor<T>>,
}

impl<T: PropertyValue> Deref for DirectPropertyBase<T> {
    type Target = FerroProperty;
    #[inline]
    fn deref(&self) -> &FerroProperty {
        &self.base
    }
}

impl<T: PropertyValue> DirectPropertyBase<T> {
    /// The untyped base of the property.
    #[inline]
    pub fn as_property(&'static self) -> &'static FerroProperty {
        &self.base
    }

    /// The class that registered this instance of the property.
    #[inline]
    pub fn owner(&self) -> &'static TypeInfo {
        self.owner
    }

    #[inline]
    fn accessor(&self) -> &'static dyn DirectPropertyAccessor<T> {
        *self.accessor.get().expect("property is not fully registered")
    }

    /// Gets the value of the property on the instance.
    #[inline]
    pub(crate) fn invoke_getter(&self, instance: &FerroObject) -> T {
        self.accessor().invoke_getter(instance)
    }

    /// Sets the value of the property on the instance.
    #[inline]
    pub(crate) fn invoke_setter(&self, instance: &FerroObject, value: BindingValue<T>) {
        self.accessor().invoke_setter(instance, value)
    }

    /// Gets the unset value for the property on the specified type.
    pub fn get_unset_value(&self, type_: &'static TypeInfo) -> T {
        self.get_metadata(type_).unset_value().clone()
    }

    /// Gets the unset value for the property on the type of `owner`.
    pub fn get_unset_value_for(&self, owner: &FerroObject) -> T {
        self.get_unset_value(owner.get_type())
    }

    /// Gets the property metadata for the specified type.
    pub fn get_metadata(&self, type_: &'static TypeInfo) -> Rc<DirectPropertyMetadata<T>> {
        self.metadata.get(type_)
    }

    /// Gets the property metadata for the type of `owner`.
    pub fn get_metadata_for(&self, owner: &FerroObject) -> Rc<DirectPropertyMetadata<T>> {
        self.metadata.get(owner.get_type())
    }

    /// Overrides the metadata for the property on class `TOwner`.
    pub fn override_metadata<TOwner: ObjectType>(&self, metadata: DirectPropertyMetadata<T>) {
        self.override_metadata_for(TOwner::TYPE, metadata);
    }

    /// Overrides the metadata for the property on the specified type.
    pub fn override_metadata_for(&self, type_: &'static TypeInfo, metadata: DirectPropertyMetadata<T>) {
        self.metadata.override_metadata(self.name(), type_, metadata);
    }

    /// Removes the metadata registered for the specified type.
    pub fn unregister(&self, type_: &'static TypeInfo) {
        self.metadata.unregister(type_);
    }

    #[track_caller]
    fn invalid_value(&self) -> ! {
        self.base.invalid_value_type()
    }
}

impl<T: PropertyValue> PropertyRoutes for DirectPropertyBase<T> {
    fn create_effective_value(&'static self, _o: &FerroObject) -> Rc<dyn EffectiveValueDyn> {
        panic!("direct properties have no effective value");
    }

    fn route_clear_value(&'static self, o: &FerroObject) {
        o.clear_direct_value(self);
    }

    fn route_coerce_default_value(&'static self, _o: &FerroObject) {
        // Direct properties are not coerced.
    }

    fn route_get_value(&'static self, o: &FerroObject) -> BoxedValue {
        Rc::new(o.get_direct_value(self))
    }

    fn route_get_base_value(&'static self, o: &FerroObject) -> BoxedValue {
        Rc::new(o.get_direct_value(self))
    }

    fn route_get_default_value(&'static self, type_: &'static TypeInfo) -> BoxedValue {
        Rc::new(self.get_unset_value(type_))
    }

    fn route_set_value(
        &'static self,
        o: &FerroObject,
        value: &dyn Any,
        _priority: BindingPriority,
    ) -> Option<Rc<dyn IDisposable>> {
        let this = self;
        if value.is::<UnsetValueType>() {
            o.clear_direct_value(this);
        } else if value.is::<DoNothingType>() {
        } else if let Some(v) = value.downcast_ref::<T>() {
            o.set_direct_value(this, v.clone());
        } else {
            self.invalid_value();
        }
        None
    }

    fn route_set_current_value(&'static self, o: &FerroObject, value: &dyn Any) {
        self.route_set_value(o, value, BindingPriority::LocalValue);
    }

    fn route_set_direct_value_unchecked(&'static self, o: &FerroObject, entry: &dyn IValueEntry) {
        let mut value: Option<T> = None;
        if entry.try_get_value(&mut value) {
            if let Some(value) = value {
                o.set_direct_value_unchecked(self, BindingValue::new(value));
            }
        } else if let Some(boxed) = entry.get_value_boxed() {
            // An untyped entry (a binding expression): the value is already
            // converted to the property type, or is a marker. Data validation
            // is reported separately by the expression's sink.
            if let Some(value) = boxed.downcast_ref::<T>() {
                self.invoke_setter(o, BindingValue::new(value.clone()));
            } else if boxed.is::<UnsetValueType>() {
                self.invoke_setter(o, BindingValue::new(self.get_unset_value_for(o)));
            }
        }
    }

    fn route_bind(
        &'static self,
        o: &FerroObject,
        source: Rc<dyn IObservable<BoxedValue>>,
        _priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        o.bind_direct_untyped(self, source)
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
        value.is::<T>()
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
        _o: &FerroObject,
        _frame: std::rc::Weak<dyn crate::property_store::ValueFrame>,
        _source: Rc<dyn IObservable<BoxedValue>>,
    ) -> Rc<dyn IValueEntry> {
        panic!("direct properties have no value entries");
    }
}

/// A direct property: a property whose value is stored in a field of its
/// owner and accessed through a getter and an optional setter.
pub struct DirectProperty<TOwner: ObjectType, T: PropertyValue> {
    base: DirectPropertyBase<T>,
    getter: fn(&TOwner) -> T,
    setter: Option<fn(&TOwner, T)>,
}

impl<TOwner: ObjectType, T: PropertyValue> Deref for DirectProperty<TOwner, T> {
    type Target = DirectPropertyBase<T>;
    #[inline]
    fn deref(&self) -> &DirectPropertyBase<T> {
        &self.base
    }
}

impl<TOwner: ObjectType, T: PropertyValue> DirectProperty<TOwner, T> {
    pub(crate) fn create(
        name: &str,
        getter: fn(&TOwner) -> T,
        setter: Option<fn(&TOwner, T)>,
        metadata: DirectPropertyMetadata<T>,
    ) -> &'static Self {
        let base = FerroProperty::new::<T>(
            name,
            TOwner::TYPE,
            FerroPropertyInit { inherits: false, is_attached: false, is_direct: true, is_read_only: setter.is_none() },
            None,
        );
        Self::leak(Self {
            base: DirectPropertyBase {
                base,
                owner: TOwner::TYPE,
                metadata: MetadataTable::new(TOwner::TYPE, metadata),
                accessor: OnceCell::new(),
            },
            getter,
            setter,
        })
    }

    fn leak(property: Self) -> &'static Self {
        let property: &'static Self = Box::leak(Box::new(property));
        let _ = property.base.accessor.set(property);
        property.base.base.set_routes(&property.base);
        property
    }

    /// The getter function.
    #[inline]
    pub fn getter(&self) -> fn(&TOwner) -> T {
        self.getter
    }

    /// The setter function, if the property is writable.
    #[inline]
    pub fn setter(&self) -> Option<fn(&TOwner, T)> {
        self.setter
    }

    /// Registers the direct property on another class, with accessors for
    /// that class. The new property shares the identity of this one.
    pub fn add_owner<TNewOwner: ObjectType>(
        &'static self,
        getter: fn(&TNewOwner) -> T,
        setter: Option<fn(&TNewOwner, T)>,
        metadata: Option<DirectPropertyMetadata<T>>,
    ) -> &'static DirectProperty<TNewOwner, T> {
        let metadata = metadata.map(|mut m| {
            use crate::ferro_property_metadata::PropertyMetadata;
            m.merge(&self.base.metadata.get(TOwner::TYPE));
            m.freeze();
            m
        });
        let result = DirectProperty::<TNewOwner, T>::leak(DirectProperty {
            base: DirectPropertyBase {
                base: FerroProperty::for_added_owner(&self.base.base, TNewOwner::TYPE, setter.is_none()),
                owner: TNewOwner::TYPE,
                metadata: MetadataTable::for_added_owner(&self.base.metadata, TNewOwner::TYPE, metadata),
                accessor: OnceCell::new(),
            },
            getter,
            setter,
        });
        FerroPropertyRegistry::instance().register(TNewOwner::TYPE, result);
        result
    }

    fn owner_of<'a>(&self, instance: &'a FerroObject) -> &'a TOwner {
        instance.downcast_ref::<TOwner>().unwrap_or_else(|| {
            panic!(
                "property '{}' is registered on {}, which {} does not derive from",
                self.name(),
                TOwner::TYPE,
                instance.get_type()
            )
        })
    }
}

impl<TOwner: ObjectType, T: PropertyValue> DirectPropertyAccessor<T> for DirectProperty<TOwner, T> {
    fn invoke_getter(&self, instance: &FerroObject) -> T {
        (self.getter)(self.owner_of(instance))
    }

    fn invoke_setter(&self, instance: &FerroObject, value: BindingValue<T>) {
        let Some(setter) = self.setter else {
            panic!("The property {} is readonly.", self.name());
        };
        if let Some(value) = value.into_option() {
            setter(self.owner_of(instance), value);
        }
    }
}

/// Type-erased view of a direct property used by the registry.
pub(crate) trait DirectPropertyDyn: 'static {
    fn as_property(&self) -> &FerroProperty;
    fn as_any(&self) -> &dyn Any;
}

impl<T: PropertyValue> DirectPropertyDyn for DirectPropertyBase<T> {
    fn as_property(&self) -> &FerroProperty {
        &self.base
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
