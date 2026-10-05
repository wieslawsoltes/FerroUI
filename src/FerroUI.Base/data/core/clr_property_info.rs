use super::{IPropertyInfo, ValueType, ValueTypes};
use crate::data::BindingError;
use crate::{AnyValue, BoxedValue, PropertyValue};
use std::marker::PhantomData;
use std::rc::Rc;

/// The untyped getter of a [`ClrPropertyInfo`].
pub type PropertyGetter = Rc<dyn Fn(&dyn AnyValue) -> Option<BoxedValue>>;

/// The untyped getter of a [`ClrPropertyInfo`] that can fail: an `Err` is the
/// equivalent of the getter throwing (an index out of range, a missing key).
pub type FalliblePropertyGetter = Rc<dyn Fn(&dyn AnyValue) -> Result<Option<BoxedValue>, BindingError>>;

/// The untyped setter of a [`ClrPropertyInfo`].
pub type PropertySetter = Rc<dyn Fn(&dyn AnyValue, Option<&BoxedValue>) -> Result<(), BindingError>>;

/// A property description built from accessor closures. This is what
/// compiled bindings use to read and write plain properties.
pub struct ClrPropertyInfo {
    name: Box<str>,
    getter: Option<PropertyGetter>,
    /// The getter of a property whose read can fail; used instead of `getter`.
    fallible_getter: Option<FalliblePropertyGetter>,
    setter: Option<PropertySetter>,
    property_type: ValueType,
    /// The type that declares the property, when the accessors are typed:
    /// its name and the test of whether a value is of that type.
    owner: Option<(&'static str, fn(&dyn AnyValue) -> bool)>,
    /// Accessors that take the owner as an untyped handle; used by the boxed
    /// forms of the contract instead of the accessors above.
    boxed_getter: Option<BoxedPropertyGetter>,
    boxed_setter: Option<BoxedPropertySetter>,
}

/// The getter of a [`ClrPropertyInfo`] that takes the owner as an untyped
/// handle (see [`ClrPropertyInfo::with_boxed_accessors`]).
pub type BoxedPropertyGetter = Rc<dyn Fn(&BoxedValue) -> Result<Option<BoxedValue>, BindingError>>;

/// The setter of a [`ClrPropertyInfo`] that takes the owner as an untyped
/// handle.
pub type BoxedPropertySetter = Rc<dyn Fn(&BoxedValue, Option<&BoxedValue>) -> Result<(), BindingError>>;

/// States how the typed value of a property maps to the untyped values that
/// flow through a binding.
pub trait PropertyKind: 'static {
    /// The type the property's accessors work with.
    type Typed;

    /// The value type reported for the property.
    fn value_type() -> ValueType;

    /// Called once when a property of this kind is described.
    fn register() {}

    fn to_untyped(value: Self::Typed) -> Option<BoxedValue>;

    fn from_untyped(value: Option<&BoxedValue>) -> Option<Self::Typed>;
}

/// A property of value type `T` that is never null.
pub struct Value<T>(PhantomData<T>);

impl<T: PropertyValue> PropertyKind for Value<T> {
    type Typed = T;

    fn value_type() -> ValueType {
        ValueType::of::<T>()
    }

    #[inline]
    fn to_untyped(value: T) -> Option<BoxedValue> {
        Some(Rc::new(value))
    }

    fn from_untyped(value: Option<&BoxedValue>) -> Option<T> {
        let value = value?;
        if let Some(v) = value.downcast_ref::<T>() {
            return Some(v.clone());
        }
        let converted = ValueTypes::try_convert(Some(value), ValueType::of::<T>())??;
        converted.downcast_ref::<T>().cloned()
    }
}

/// A property of type `Option<T>`: `None` is null in bindings.
pub struct Maybe<T>(PhantomData<T>);

impl<T: PropertyValue> PropertyKind for Maybe<T> {
    type Typed = Option<T>;

    fn value_type() -> ValueType {
        ValueType::of::<Option<T>>()
    }

    fn register() {
        ValueTypes::register_nullable::<T>();
    }

    #[inline]
    fn to_untyped(value: Option<T>) -> Option<BoxedValue> {
        value.map(|v| Rc::new(v) as BoxedValue)
    }

    fn from_untyped(value: Option<&BoxedValue>) -> Option<Option<T>> {
        let Some(value) = value else { return Some(None) };
        if let Some(v) = value.downcast_ref::<Option<T>>() {
            return Some(v.clone());
        }
        Value::<T>::from_untyped(Some(value)).map(Some)
    }
}

/// A property that refers to a shared model object: `Option<Rc<M>>`. The
/// object itself travels through bindings (no copy, identity preserved).
pub struct ModelRef<M>(PhantomData<M>);

impl<M: PartialEq + 'static> PropertyKind for ModelRef<M> {
    type Typed = Option<Rc<M>>;

    fn value_type() -> ValueType {
        ValueType::of::<M>()
    }

    fn register() {
        ValueTypes::register_reference::<M>();
    }

    #[inline]
    fn to_untyped(value: Option<Rc<M>>) -> Option<BoxedValue> {
        value.map(|v| v as BoxedValue)
    }

    fn from_untyped(value: Option<&BoxedValue>) -> Option<Option<Rc<M>>> {
        let Some(value) = value else { return Some(None) };
        if let Some(v) = value.downcast_ref::<Option<Rc<M>>>() {
            return Some(v.clone());
        }
        let any: Rc<dyn std::any::Any> = value.clone();
        any.downcast::<M>().ok().map(Some)
    }
}

/// A property holding an untyped value (`Option<BoxedValue>`), passed
/// through as it is.
pub struct Untyped;

impl PropertyKind for Untyped {
    type Typed = Option<BoxedValue>;

    fn value_type() -> ValueType {
        ValueType::object()
    }

    #[inline]
    fn to_untyped(value: Option<BoxedValue>) -> Option<BoxedValue> {
        value
    }

    fn from_untyped(value: Option<&BoxedValue>) -> Option<Option<BoxedValue>> {
        match value {
            None => Some(None),
            Some(v) => match v.downcast_ref::<Option<BoxedValue>>() {
                Some(inner) => Some(inner.clone()),
                None => Some(Some(v.clone())),
            },
        }
    }
}

fn invalid_cast(name: &str, value: Option<&BoxedValue>, target: ValueType) -> BindingError {
    BindingError::message(format!(
        "Object of type '{}' cannot be converted to type '{}' for property '{}'.",
        value.map_or("null", |v| (**v).type_name()),
        target,
        name
    ))
}

impl ClrPropertyInfo {
    /// Creates a property description from untyped accessors.
    pub fn new(
        name: &str,
        getter: Option<PropertyGetter>,
        setter: Option<PropertySetter>,
        property_type: ValueType,
    ) -> Self {
        Self { name: name.into(), getter, fallible_getter: None, setter, property_type, owner: None, boxed_getter: None, boxed_setter: None }
    }

    /// Creates a property description from untyped accessors, with a getter
    /// that can fail.
    pub fn new_fallible(
        name: &str,
        getter: Option<FalliblePropertyGetter>,
        setter: Option<PropertySetter>,
        property_type: ValueType,
    ) -> Self {
        Self { name: name.into(), getter: None, fallible_getter: getter, setter, property_type, owner: None, boxed_getter: None, boxed_setter: None }
    }

    /// Adds accessors that take the owner as an untyped handle. A property
    /// description built over accessors that need the handle of the owner
    /// and not only a view of it (the invokers of markup metadata) is read
    /// and written through them by the boxed forms of the contract
    /// ([`IPropertyInfo::get_boxed`], [`IPropertyInfo::try_get_boxed`],
    /// [`IPropertyInfo::set_boxed`]).
    pub fn with_boxed_accessors(
        mut self,
        getter: Option<BoxedPropertyGetter>,
        setter: Option<BoxedPropertySetter>,
    ) -> Self {
        self.boxed_getter = getter;
        self.boxed_setter = setter;
        self
    }

    fn with_owner<O: 'static>(mut self) -> Self {
        fn is_owner<O: 'static>(value: &dyn AnyValue) -> bool {
            value.downcast_ref::<O>().is_some()
        }
        self.owner = Some((ValueType::of::<O>().name(), is_owner::<O>));
        self
    }

    fn typed_getter<O: 'static, K: PropertyKind>(get: impl Fn(&O) -> K::Typed + 'static) -> PropertyGetter {
        Rc::new(move |o: &dyn AnyValue| o.downcast_ref::<O>().and_then(|o| K::to_untyped(get(o))))
    }

    fn typed_fallible_getter<O: 'static, K: PropertyKind>(
        get: impl Fn(&O) -> Result<K::Typed, BindingError> + 'static,
    ) -> FalliblePropertyGetter {
        Rc::new(move |o: &dyn AnyValue| match o.downcast_ref::<O>() {
            Some(o) => get(o).map(K::to_untyped),
            None => Ok(None),
        })
    }

    fn typed_setter<O: 'static, K: PropertyKind>(
        name: &str,
        set: impl Fn(&O, K::Typed) -> Result<(), BindingError> + 'static,
    ) -> PropertySetter {
        let owned_name: Box<str> = name.into();
        Rc::new(move |o: &dyn AnyValue, v: Option<&BoxedValue>| {
            let Some(o) = o.downcast_ref::<O>() else {
                return Err(BindingError::message("The property owner is of the wrong type."));
            };
            match K::from_untyped(v) {
                Some(v) => set(o, v),
                None => Err(invalid_cast(&owned_name, v, K::value_type())),
            }
        })
    }

    /// A read-only property of owner type `O` whose getter can fail, the way
    /// the getter of an indexer throws for a bad index or key.
    pub fn read_only_fallible<O: 'static, K: PropertyKind>(
        name: &str,
        get: impl Fn(&O) -> Result<K::Typed, BindingError> + 'static,
    ) -> Self {
        K::register();
        Self::new_fallible(name, Some(Self::typed_fallible_getter::<O, K>(get)), None, K::value_type())
            .with_owner::<O>()
    }

    /// A read-write property of owner type `O` whose getter and setter can
    /// fail.
    pub fn read_write_fallible<O: 'static, K: PropertyKind>(
        name: &str,
        get: impl Fn(&O) -> Result<K::Typed, BindingError> + 'static,
        set: impl Fn(&O, K::Typed) -> Result<(), BindingError> + 'static,
    ) -> Self {
        K::register();
        Self::new_fallible(
            name,
            Some(Self::typed_fallible_getter::<O, K>(get)),
            Some(Self::typed_setter::<O, K>(name, set)),
            K::value_type(),
        )
        .with_owner::<O>()
    }

    /// A read-only property of owner type `O`, from a typed getter.
    pub fn read_only<O: 'static, K: PropertyKind>(name: &str, get: impl Fn(&O) -> K::Typed + 'static) -> Self {
        K::register();
        Self::new(name, Some(Self::typed_getter::<O, K>(get)), None, K::value_type()).with_owner::<O>()
    }

    /// A read-write property of owner type `O`, from typed accessors.
    pub fn read_write<O: 'static, K: PropertyKind>(
        name: &str,
        get: impl Fn(&O) -> K::Typed + 'static,
        set: impl Fn(&O, K::Typed) + 'static,
    ) -> Self {
        Self::read_write_validated::<O, K>(name, get, move |o, v| {
            set(o, v);
            Ok(())
        })
    }

    /// A read-write property whose setter can reject values.
    pub fn read_write_validated<O: 'static, K: PropertyKind>(
        name: &str,
        get: impl Fn(&O) -> K::Typed + 'static,
        set: impl Fn(&O, K::Typed) -> Result<(), BindingError> + 'static,
    ) -> Self {
        K::register();
        let setter = Self::typed_setter::<O, K>(name, set);
        Self::new(name, Some(Self::typed_getter::<O, K>(get)), Some(setter), K::value_type()).with_owner::<O>()
    }
}

impl IPropertyInfo for ClrPropertyInfo {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn get(&self, target: &dyn AnyValue) -> Option<BoxedValue> {
        match (&self.getter, &self.fallible_getter) {
            (Some(getter), _) => getter(target),
            // The failure of a fallible getter is only visible through
            // `try_get`.
            (None, Some(getter)) => getter(target).ok().flatten(),
            (None, None) => panic!("Property {} doesn't have a getter.", self.name),
        }
    }

    fn try_get(&self, target: &dyn AnyValue) -> Result<Option<BoxedValue>, BindingError> {
        if let Some((owner_name, is_owner)) = self.owner {
            if !is_owner(target) {
                return Err(BindingError::message(format!(
                    "Unable to cast object of type '{}' to type '{}'.",
                    target.type_name(),
                    owner_name
                )));
            }
        }
        match &self.fallible_getter {
            Some(getter) if self.getter.is_none() => getter(target),
            _ => Ok(self.get(target)),
        }
    }

    fn set(&self, target: &dyn AnyValue, value: Option<&BoxedValue>) -> Result<(), BindingError> {
        match &self.setter {
            Some(setter) => setter(target, value),
            None => panic!("Property {} doesn't have a setter.", self.name),
        }
    }

    fn get_boxed(&self, target: &BoxedValue) -> Option<BoxedValue> {
        match &self.boxed_getter {
            Some(getter) => getter(target).ok().flatten(),
            None => self.get(&**target),
        }
    }

    fn try_get_boxed(&self, target: &BoxedValue) -> Result<Option<BoxedValue>, BindingError> {
        match &self.boxed_getter {
            Some(getter) => getter(target),
            None => self.try_get(&**target),
        }
    }

    fn set_boxed(&self, target: &BoxedValue, value: Option<&BoxedValue>) -> Result<(), BindingError> {
        match &self.boxed_setter {
            Some(setter) => setter(target, value),
            None => self.set(&**target, value),
        }
    }

    fn can_set(&self) -> bool {
        self.setter.is_some() || self.boxed_setter.is_some()
    }

    fn can_get(&self) -> bool {
        self.getter.is_some() || self.fallible_getter.is_some() || self.boxed_getter.is_some()
    }

    fn property_type(&self) -> ValueType {
        self.property_type
    }
}
