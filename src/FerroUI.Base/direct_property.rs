use crate::data::BindingValue;
use crate::ferro_property::{convert_property_value, take_boxed_value, DirectRoutes, FerroPropertyInit, PropertyRoutes};
use crate::ferro_property_metadata::MetadataTable;
use crate::property_store::IValueEntry;
use crate::reactive::{IDisposable, IObservable};
use crate::{
    BoxedValue, ObjectType, DirectPropertyMetadata, FerroObject, FerroProperty, FerroPropertyRegistry, PropertyValue,
    TypeInfo, UnsetValueType,
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
    // These fields depend on the value type and owner identity, but not the
    // owner's Rust type. Keep their construction shared across owner types.
    #[cold]
    #[inline(never)]
    fn new(
        name: &str,
        owner_type: &'static TypeInfo,
        is_read_only: bool,
        metadata: DirectPropertyMetadata<T>,
    ) -> Self {
        let base = FerroProperty::new::<T>(
            name,
            owner_type,
            FerroPropertyInit { inherits: false, is_attached: false, is_direct: true, is_read_only },
            None,
        );
        Self {
            base,
            owner: owner_type,
            metadata: MetadataTable::new(owner_type, metadata),
            accessor: OnceCell::new(),
        }
    }

    #[cold]
    #[inline(never)]
    fn for_added_owner(
        source: &Self,
        source_owner_type: &'static TypeInfo,
        owner_type: &'static TypeInfo,
        is_read_only: bool,
        metadata: Option<DirectPropertyMetadata<T>>,
    ) -> Self {
        let metadata = metadata.map(|mut m| {
            use crate::ferro_property_metadata::PropertyMetadata;
            m.merge(&source.metadata.get(source_owner_type));
            m.freeze();
            m
        });
        Self {
            base: FerroProperty::for_added_owner(&source.base, owner_type, is_read_only),
            owner: owner_type,
            metadata: MetadataTable::for_added_owner(&source.metadata, owner_type, metadata),
            accessor: OnceCell::new(),
        }
    }

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

    /// Gives the base of the property its routes: the shared routes over
    /// this property, its metadata and the typed operations of `T`.
    fn install_routes(&'static self) {
        self.base.set_routes(PropertyRoutes::direct(&self.base, self, self.metadata.core(), Self::ROUTES));
    }
}

// The typed operations of the routes (`DirectRoutes`): what calls the
// accessors. Everything else of a route is shared (`PropertyRoutes`).
impl<T: PropertyValue> DirectPropertyBase<T> {
    const ROUTES: &'static DirectRoutes = &DirectRoutes {
        clear_value: Self::route_clear_value,
        get_value: Self::route_get_value,
        get_default_value: Self::route_get_default_value,
        set_value: Self::route_set_value,
        set_direct_value_unchecked: Self::route_set_direct_value_unchecked,
        bind: Self::route_bind,
        set_direct_binding_value: Self::route_set_direct_binding_value,
    };

    /// The property of a route.
    #[inline]
    fn of_routes(typed: &'static dyn Any) -> &'static Self {
        debug_assert!(typed.is::<Self>());
        // SAFETY: the routes of a direct property are made only by
        // `install_routes`, with the property itself as their typed view, and
        // a property holds the routes of its own value type.
        unsafe { &*(typed as *const dyn Any as *const Self) }
    }

    fn route_clear_value(typed: &'static dyn Any, o: &FerroObject) {
        o.clear_direct_value(Self::of_routes(typed));
    }

    fn route_get_value(typed: &'static dyn Any, o: &FerroObject) -> BoxedValue {
        Rc::new(o.get_direct_value(Self::of_routes(typed)))
    }

    fn route_get_default_value(typed: &'static dyn Any, type_: &'static TypeInfo) -> BoxedValue {
        Rc::new(Self::of_routes(typed).get_unset_value(type_))
    }

    fn route_set_value(typed: &'static dyn Any, o: &FerroObject, value: &dyn Any) {
        let this = Self::of_routes(typed);
        let value = match value.downcast_ref::<T>() {
            Some(value) => value.clone(),
            None => match convert_property_value::<T>(this, value) {
                Some(value) => value,
                None => this.base.invalid_value_type(),
            },
        };
        o.set_direct_value(this, value);
    }

    fn route_set_direct_value_unchecked(typed: &'static dyn Any, o: &FerroObject, entry: &dyn IValueEntry) {
        let this = Self::of_routes(typed);
        let mut value: Option<T> = None;
        if entry.try_get_value(&mut value) {
            if let Some(value) = value {
                o.set_direct_value_unchecked(this, BindingValue::new(value));
            }
        } else if let Some(boxed) = entry.get_value_boxed() {
            // An untyped entry (a binding expression): the value is already
            // converted to the property type, or is a marker. Data validation
            // is reported separately by the expression's sink.
            if let Some(value) = boxed.downcast_ref::<T>() {
                this.invoke_setter(o, BindingValue::new(value.clone()));
            } else if boxed.is::<UnsetValueType>() {
                this.invoke_setter(o, BindingValue::new(this.get_unset_value_for(o)));
            }
        }
    }

    fn route_bind(
        typed: &'static dyn Any,
        o: &FerroObject,
        source: Rc<dyn IObservable<BoxedValue>>,
    ) -> Rc<dyn IDisposable> {
        o.bind_direct_untyped(Self::of_routes(typed), source)
    }

    fn route_set_direct_binding_value(typed: &'static dyn Any, o: &FerroObject, value: BindingValue<BoxedValue>) {
        let kind = value.value_type();
        let (value, error) = value.into_value_and_error();
        let value = value.and_then(take_boxed_value::<T>);
        o.set_direct_value_unchecked(Self::of_routes(typed), BindingValue::from_notification_parts(kind, value, error));
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
        Self::leak(Self {
            base: DirectPropertyBase::new(name, TOwner::TYPE, setter.is_none(), metadata),
            getter,
            setter,
        })
    }

    fn leak(property: Self) -> &'static Self {
        let property: &'static Self = Box::leak(Box::new(property));
        let _ = property.base.accessor.set(property);
        property.base.install_routes();
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
        let result = DirectProperty::<TNewOwner, T>::leak(DirectProperty {
            base: DirectPropertyBase::for_added_owner(
                &self.base, TOwner::TYPE, TNewOwner::TYPE, setter.is_none(), metadata,
            ),
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
