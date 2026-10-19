//! `IBindingTypeConverter` (`ReactiveUI.Binding/Bindings/IBindingTypeConverter.cs`), with
//! [`ConverterType`], the type a converter names as its source or target.

use std::any::{Any, TypeId};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// A type as the converters name it (upstream's `Type`): its [`TypeId`], which is the key the
/// registries look converters up by, and its name, for messages.
///
/// Two values are equal when they name the same type; the name takes no part in the comparison.
// Deviation (DEVIATIONS.md, ReactiveUI): without reflection, a type is its `TypeId`; nothing tells a
// value type from a nullable one (`Type.IsValueType`, `Nullable.GetUnderlyingType`), so an
// `Option<T>` is a type of its own.
#[derive(Clone, Copy)]
pub struct ConverterType {
    id: TypeId,
    name: &'static str,
}

impl ConverterType {
    /// The type `T` (`typeof(T)`). `ConverterType::of::<dyn Any>()` is upstream's `typeof(object)`.
    pub fn of<T: ?Sized + 'static>() -> Self {
        ConverterType { id: TypeId::of::<T>(), name: std::any::type_name::<T>() }
    }

    /// The identifier of the type.
    pub fn id(&self) -> TypeId {
        self.id
    }

    /// The name of the type, for messages (`Type.Name`).
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Whether `value` is a value of this type: its run-time type is this one (`value.GetType() ==
    /// type`).
    pub fn is_type_of(&self, value: &dyn Any) -> bool {
        Any::type_id(value) == self.id
    }
}

impl PartialEq for ConverterType {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for ConverterType {}

impl PartialEq<TypeId> for ConverterType {
    fn eq(&self, other: &TypeId) -> bool {
        self.id == *other
    }
}

impl Hash for ConverterType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl fmt::Debug for ConverterType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

impl fmt::Display for ConverterType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

/// Converts the values of one type to another for the bindings (`IBindingTypeConverter`).
///
/// A converter names its source and target types; [`BindingTypeConverterRegistry`] finds it by that
/// pair, and of several converters of one pair the one with the highest affinity wins. Most
/// converters derive from the base [`BindingTypeConverter`], which writes this trait from a typed
/// conversion; a converter that handles its input itself implements this trait directly.
///
/// Values cross this API type-erased: the input is a `&dyn Any` (`None` stands for upstream's
/// `null`), and a converted value is an `Rc<dyn Any>` holding a value of [`to_type`](Self::to_type).
///
/// [`BindingTypeConverterRegistry`]: crate::BindingTypeConverterRegistry
/// [`BindingTypeConverter`]: crate::BindingTypeConverter
// Deviation (DEVIATIONS.md, ReactiveUI): upstream's interface extends `IEnableLogger`; the logger of
// Splat is reached through `ferroui_splat::LogHost` instead, as the other extension points of the
// port do.
// Deviation (DEVIATIONS.md, ReactiveUI): `TryConvertTyped(.., out object? result)` returns the
// converted value, or `None` when the conversion fails. A conversion cannot succeed with `null`: a
// nullable target is an `Option<T>`, whose `None` is a value, and the converters that produce a
// `null` string upstream produce an empty string (see `NullableIntegerToStringTypeConverter`).
pub trait IBindingTypeConverter {
    /// The type this converter converts from (`FromType`).
    fn from_type(&self) -> ConverterType;

    /// The type this converter converts to (`ToType`).
    fn to_type(&self) -> ConverterType;

    /// How well this converter converts from [`from_type`](Self::from_type) to
    /// [`to_type`](Self::to_type): a positive score when it can, where a higher score wins; zero or
    /// less when it cannot (`GetAffinityForObjects`). The built-in converters score
    /// [`BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER`](crate::BindingAffinity::DEFAULT_INTERNAL_TYPE_CONVERTER).
    fn get_affinity_for_objects(&self) -> i32;

    /// Converts `from` (`None` for upstream's `null`), with an optional `conversion_hint` (a
    /// format, a width, a value to compare with: what the converter makes of it), and returns the
    /// converted value, or `None` when `from` is not a value of [`from_type`](Self::from_type) or
    /// cannot be converted (`TryConvertTyped`).
    fn try_convert_typed(&self, from: Option<&dyn Any>, conversion_hint: Option<&dyn Any>) -> Option<Rc<dyn Any>>;
}
