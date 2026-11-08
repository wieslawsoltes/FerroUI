use crate::data::{BindingMode, BindingPriority, BindingValue};
use crate::ferro_property_metadata::{Coercion, ErasedMetadata, MetadataTableCore};
use crate::property_store::{EffectiveValue, EffectiveValueDyn, IValueEntry};
use crate::reactive::{Disposable, IDisposable, IObservable, Subscription};
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
use std::marker::PhantomData;
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
        other.any_value_as_any().downcast_ref::<T>().is_some_and(|other| value_equals(self, other))
    }

    fn any_value_type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

/// Deviation (DEVIATIONS.md, Property system): the comparer of the reference
/// in place of `PartialEq` alone.
///
/// Whether two property values are equal as the reference compares them
/// (`EqualityComparer<T>.Default.Equals`, and `object.Equals` for boxed
/// values): `==`, and a floating-point value that is not a number equals
/// another that is not a number, bare or nullable. With `==` alone a value
/// store whose value is NaN (a `Width` that is not set) reports a change
/// every time the value is set again.
#[inline]
pub fn value_equals<T: PartialEq + 'static>(a: &T, b: &T) -> bool {
    a == b || both_not_a_number(a, b)
}

#[inline]
fn both_not_a_number<T: 'static>(a: &T, b: &T) -> bool {
    let (a, b): (&dyn Any, &dyn Any) = (a, b);
    if let (Some(a), Some(b)) = (a.downcast_ref::<f64>(), b.downcast_ref::<f64>()) {
        return a.is_nan() && b.is_nan();
    }
    if let (Some(a), Some(b)) = (a.downcast_ref::<f32>(), b.downcast_ref::<f32>()) {
        return a.is_nan() && b.is_nan();
    }
    if let (Some(Some(a)), Some(Some(b))) = (a.downcast_ref::<Option<f64>>(), b.downcast_ref::<Option<f64>>()) {
        return a.is_nan() && b.is_nan();
    }
    if let (Some(Some(a)), Some(Some(b))) = (a.downcast_ref::<Option<f32>>(), b.downcast_ref::<Option<f32>>()) {
        return a.is_nan() && b.is_nan();
    }
    false
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

// Installed only by an explicit inheritance registration. The property owns
// this capability; its metadata may be overridden without changing whether
// the property inherits or which Rust value type its entries contain.
pub(crate) type InheritedValueChanged = fn(
    &dyn EffectiveValueDyn,
    &FerroObject,
    Option<&dyn EffectiveValueDyn>,
    Option<&dyn EffectiveValueDyn>,
);

/// Only the payload copy depends on T. Keep implicit conversion lookup and
/// the converted box's ownership in one cold implementation (the `TryConvert`
/// of the property, see
/// [`ValueTypes::try_convert_implicit`](crate::data::core::ValueTypes::try_convert_implicit)).
pub(crate) fn convert_property_value<T: PropertyValue>(property: &FerroProperty, value: &dyn Any) -> Option<T> {
    let mut typed = None::<T>;
    copy_converted_property_value(property, value, &mut typed);
    typed
}

/// The typed destination of an untyped value: `Option<T>`, which takes a copy of
/// the payload when it is a `T`. The one typed operation of the untyped routes
/// of a property (conversion, binding values, notifications), shared by all of
/// them: one implementation per value type.
pub(crate) trait PayloadSlot {
    /// Stores a copy of `value` if it is of the type of the slot (and nothing
    /// otherwise); whether it was.
    fn copy_payload(&mut self, value: &dyn Any) -> bool;
}

impl<T: Clone + 'static> PayloadSlot for Option<T> {
    fn copy_payload(&mut self, value: &dyn Any) -> bool {
        *self = value.downcast_ref::<T>().cloned();
        self.is_some()
    }
}

/// The untyped destination of an untyped value: a box of its own, which takes
/// a copy of the payload when it is of the property's value type. The slot of
/// the type-erased binding observers, which keep their values boxed; the copy
/// is the [`box_value`](ValueTable::box_value) of the value type of the
/// property.
pub(crate) struct BoxedPayloadSlot {
    table: &'static ValueTable,
    value: Option<BoxedValue>,
}

impl BoxedPayloadSlot {
    #[inline]
    pub(crate) fn new(property: &FerroProperty) -> Self {
        Self { table: property.value_table(), value: None }
    }

    /// The copy of the payload, if there is one.
    #[inline]
    pub(crate) fn into_value(self) -> Option<BoxedValue> {
        self.value
    }
}

impl PayloadSlot for BoxedPayloadSlot {
    fn copy_payload(&mut self, value: &dyn Any) -> bool {
        self.value = (self.table.box_value)(value);
        self.value.is_some()
    }
}

/// The value of `value` if it holds exactly a `T`: moved out of the box when
/// no one else holds it, else a clone. The typed end of the type-erased
/// binding observers, which hand a box they made themselves (the copy of a
/// converted or notified value) without a second copy.
pub(crate) fn take_boxed_value<T: PropertyValue>(value: BoxedValue) -> Option<T> {
    let value: Rc<dyn Any> = value;
    let value = value.downcast::<T>().ok()?;
    Some(Rc::try_unwrap(value).unwrap_or_else(|value| T::clone(&value)))
}

/// The parts of `value` if it is a `BindingValue<T>`, with its value copied
/// into a box of its own: the binding value case of an untyped binding, the
/// one that names `T` (the [`unpack_binding_value`](ValueTable::unpack_binding_value)
/// of the value type).
pub(crate) fn unpack_binding_value<T: PropertyValue>(value: &dyn Any) -> Option<BindingValue<BoxedValue>> {
    let value = value.downcast_ref::<BindingValue<T>>()?;
    let payload = value.has_value().then(|| Rc::new(value.value().clone()) as BoxedValue);
    Some(BindingValue::from_notification_parts(value.value_type(), payload, value.error().cloned()))
}

#[inline(never)]
pub(crate) fn copy_converted_property_value(
    property: &FerroProperty,
    value: &dyn Any,
    copy_value: &mut dyn PayloadSlot,
) -> bool {
    let Some(converted) = crate::data::core::ValueTypes::try_convert_implicit(value, property.property_type()) else {
        return false;
    };
    let copied = copy_value.copy_payload(converted.as_any());
    // A converter's temporary may have an observable destructor. Release it
    // before validation, coercion or publication; it also drops if Clone panics.
    drop(converted);
    copied
}

/// The operations on the values of one property value type that the shared
/// property core needs: one static table per value type, shared by all the
/// properties of that type, styled, attached and direct (#576). The routes of
/// a property and the type-erased observers, entries and notifications are
/// compiled once and use the table; a value type contributes these functions,
/// not a copy of every algorithm that moves its values.
pub(crate) struct ValueTable {
    /// Clones `value` into a box of its own if it holds exactly the value
    /// type; `None` otherwise.
    pub(crate) box_value: fn(&dyn Any) -> Option<BoxedValue>,
    /// Clones `value`, which must hold exactly the value type `T`, into
    /// `out`, which must be an `&mut Option<T>`. False if either has another
    /// type.
    pub(crate) copy_value: fn(&dyn Any, &mut dyn Any) -> bool,
    /// The parts of `value` if it is a `BindingValue<T>` of the value type
    /// `T`, with the value in a box of its own; `None` if it is not.
    pub(crate) unpack_binding_value: fn(&dyn Any) -> Option<BindingValue<BoxedValue>>,
    /// Clones `value` into `out` and returns the copy. Unchecked: `value`
    /// must be a `T` and `out` an `Option<T>` of the value type `T`, and the
    /// copy replaces what `out` held.
    pub(crate) clone_into: for<'a, 'b> unsafe fn(&'b dyn Any, &'a mut dyn Any) -> &'a dyn Any,
}

impl ValueTable {
    /// The table of the value type `T`. Not inlined: the properties of a
    /// value type a crate registers share one table.
    #[inline(never)]
    pub(crate) fn of<T: PropertyValue>() -> &'static ValueTable {
        TypedValueTable::<T>::TABLE
    }
}

struct TypedValueTable<T>(PhantomData<T>);

impl<T: PropertyValue> TypedValueTable<T> {
    const TABLE: &'static ValueTable = &ValueTable {
        box_value: box_value::<T>,
        copy_value: copy_value::<T>,
        unpack_binding_value: unpack_binding_value::<T>,
        clone_into: clone_into::<T>,
    };
}

fn box_value<T: PropertyValue>(value: &dyn Any) -> Option<BoxedValue> {
    value.downcast_ref::<T>().map(|value| Rc::new(value.clone()) as BoxedValue)
}

fn copy_value<T: PropertyValue>(value: &dyn Any, out: &mut dyn Any) -> bool {
    match (value.downcast_ref::<T>(), out.downcast_mut::<Option<T>>()) {
        (Some(value), Some(out)) => {
            *out = Some(value.clone());
            true
        }
        _ => false,
    }
}

/// # Safety
///
/// `value` must be a `T` and `out` an `Option<T>`.
unsafe fn clone_into<'a, T: PropertyValue>(value: &dyn Any, out: &'a mut dyn Any) -> &'a dyn Any {
    debug_assert!(value.is::<T>() && out.is::<Option<T>>());
    // SAFETY: the types are the caller's contract; the casts drop only the
    // vtables. The checked downcasts would call through them on a hot path.
    let (value, out) =
        unsafe { (&*(value as *const dyn Any as *const T), &mut *(out as *mut dyn Any as *mut Option<T>)) };
    let copy: &'a T = out.insert(value.clone());
    copy
}

/// The typed operations of a styled (or attached) property with value type
/// `T`: one static table per value type, the part of the routes that reads or
/// writes the typed store. Each takes the typed property as the
/// [`typed`](PropertyRoutes::as_any) view of its routes.
pub(crate) struct StyledRoutes {
    pub(crate) create_effective_value: fn(&'static dyn Any, &FerroObject) -> Rc<dyn EffectiveValueDyn>,
    pub(crate) coerce_default_value: fn(&'static dyn Any, &FerroObject),
    pub(crate) get_base_value: fn(&'static dyn Any, &FerroObject) -> Option<BoxedValue>,
    pub(crate) get_default_value: fn(&'static dyn Any, &'static TypeInfo) -> BoxedValue,
    /// Writes a value of the value type, or one that converts to it, with a
    /// priority; panics for any other value.
    pub(crate) set_value: fn(&'static dyn Any, &FerroObject, &dyn Any, BindingPriority) -> Option<Rc<dyn IDisposable>>,
    /// `set_current_value` of a value of the value type, or one that converts
    /// to it; panics for any other value.
    pub(crate) set_current_value: fn(&'static dyn Any, &FerroObject, &dyn Any),
    /// The typed `set_local_value` of the store from a box that holds exactly
    /// the value type, moved out of the box when no one else holds it.
    pub(crate) set_local_value: fn(&'static dyn Any, &FerroObject, BoxedValue),
    pub(crate) is_valid_value: fn(&'static dyn Any, &dyn Any) -> bool,
}

/// The typed operations of a direct property with value type `T`: one static
/// table per value type, the part of the routes that calls the accessors of the
/// property.
pub(crate) struct DirectRoutes {
    pub(crate) clear_value: fn(&'static dyn Any, &FerroObject),
    pub(crate) get_value: fn(&'static dyn Any, &FerroObject) -> BoxedValue,
    pub(crate) get_default_value: fn(&'static dyn Any, &'static TypeInfo) -> BoxedValue,
    /// Sets a value of the value type, or one that converts to it; panics for
    /// any other value.
    pub(crate) set_value: fn(&'static dyn Any, &FerroObject, &dyn Any),
    pub(crate) set_direct_value_unchecked: fn(&'static dyn Any, &FerroObject, &dyn IValueEntry),
    pub(crate) bind: fn(&'static dyn Any, &FerroObject, Rc<dyn IObservable<BoxedValue>>) -> Rc<dyn IDisposable>,
    pub(crate) set_direct_binding_value: fn(&'static dyn Any, &FerroObject, BindingValue<BoxedValue>),
}

#[derive(Clone, Copy)]
enum RoutesKind {
    Styled(&'static StyledRoutes),
    Direct(&'static DirectRoutes),
}

/// Untyped operations that a property routes to its typed implementation.
///
/// The routes of every property are these functions, compiled once (#576): what
/// does not depend on the value type (the unset and do-nothing markers, the
/// metadata, the store's lookups and the boxed bindings) is done here, the
/// copies of values by the [`ValueTable`] of the value type, and the typed reads
/// and writes of the store or the accessors by the [`StyledRoutes`] or
/// [`DirectRoutes`] of the value type. A property holds its routes from its
/// registration.
#[derive(Clone, Copy)]
pub(crate) struct PropertyRoutes {
    property: &'static FerroProperty,
    typed: &'static dyn Any,
    metadata: &'static MetadataTableCore,
    kind: RoutesKind,
}

impl PropertyRoutes {
    /// The routes of a styled (or attached) property: `property` is the base of
    /// `typed`, the `StyledProperty<T>`, `metadata` its metadata and `routes`
    /// the typed operations of `T`.
    pub(crate) fn styled(
        property: &'static FerroProperty,
        typed: &'static dyn Any,
        metadata: &'static MetadataTableCore,
        routes: &'static StyledRoutes,
    ) -> Self {
        Self { property, typed, metadata, kind: RoutesKind::Styled(routes) }
    }

    /// The routes of a direct property: `property` is the base of `typed`,
    /// the `DirectPropertyBase<T>`, `metadata` its metadata and `routes` the
    /// typed operations of `T`.
    pub(crate) fn direct(
        property: &'static FerroProperty,
        typed: &'static dyn Any,
        metadata: &'static MetadataTableCore,
        routes: &'static DirectRoutes,
    ) -> Self {
        Self { property, typed, metadata, kind: RoutesKind::Direct(routes) }
    }

    pub(crate) fn create_effective_value(&self, o: &FerroObject) -> Rc<dyn EffectiveValueDyn> {
        match self.kind {
            RoutesKind::Styled(routes) => (routes.create_effective_value)(self.typed, o),
            RoutesKind::Direct(_) => panic!("direct properties have no effective value"),
        }
    }

    pub(crate) fn route_clear_value(&self, o: &FerroObject) {
        match self.kind {
            RoutesKind::Styled(_) => o.values().clear_value(o, self.property),
            RoutesKind::Direct(routes) => (routes.clear_value)(self.typed, o),
        }
    }

    pub(crate) fn route_coerce_default_value(&self, o: &FerroObject) {
        match self.kind {
            RoutesKind::Styled(routes) => (routes.coerce_default_value)(self.typed, o),
            // Direct properties are not coerced.
            RoutesKind::Direct(_) => {}
        }
    }

    /// The value of the property on `o`, in a box of its own. For a styled
    /// property: the effective value, else the inherited value, else the
    /// default value for the type of `o`, as the typed read.
    pub(crate) fn route_get_value(&self, o: &FerroObject) -> BoxedValue {
        match self.kind {
            RoutesKind::Styled(routes) => {
                let values = o.values();
                if let Some(effective) = values.get_effective_value(self.property) {
                    return effective.boxed_value();
                }
                if self.property.inherits() {
                    if let Some(inherited) = values.try_get_inherited_value(self.property) {
                        return inherited.boxed_value();
                    }
                }
                (routes.get_default_value)(self.typed, o.get_type())
            }
            RoutesKind::Direct(routes) => (routes.get_value)(self.typed, o),
        }
    }

    pub(crate) fn route_get_base_value(&self, o: &FerroObject) -> BoxedValue {
        match self.kind {
            RoutesKind::Styled(routes) => match (routes.get_base_value)(self.typed, o) {
                Some(value) => value,
                None => FerroProperty::unset_value(),
            },
            RoutesKind::Direct(routes) => (routes.get_value)(self.typed, o),
        }
    }

    pub(crate) fn route_get_default_value(&self, type_: &'static TypeInfo) -> BoxedValue {
        match self.kind {
            RoutesKind::Styled(routes) => (routes.get_default_value)(self.typed, type_),
            RoutesKind::Direct(routes) => (routes.get_default_value)(self.typed, type_),
        }
    }

    pub(crate) fn route_set_value(
        &self,
        o: &FerroObject,
        value: &dyn Any,
        priority: BindingPriority,
    ) -> Option<Rc<dyn IDisposable>> {
        match self.kind {
            RoutesKind::Styled(routes) => {
                if value.is::<UnsetValueType>() {
                    if priority == BindingPriority::LocalValue {
                        o.values().clear_value(o, self.property);
                    }
                    None
                } else if value.is::<DoNothingType>() {
                    None
                } else {
                    (routes.set_value)(self.typed, o, value, priority)
                }
            }
            RoutesKind::Direct(routes) => {
                if value.is::<UnsetValueType>() {
                    (routes.clear_value)(self.typed, o);
                } else if !value.is::<DoNothingType>() {
                    (routes.set_value)(self.typed, o, value);
                }
                None
            }
        }
    }

    pub(crate) fn route_set_current_value(&self, o: &FerroObject, value: &dyn Any) {
        match self.kind {
            RoutesKind::Styled(routes) => {
                if value.is::<UnsetValueType>() {
                    o.values().clear_value(o, self.property);
                } else if !value.is::<DoNothingType>() {
                    (routes.set_current_value)(self.typed, o, value);
                }
            }
            RoutesKind::Direct(_) => {
                self.route_set_value(o, value, BindingPriority::LocalValue);
            }
        }
    }

    pub(crate) fn route_set_direct_value_unchecked(&self, o: &FerroObject, entry: &dyn IValueEntry) {
        match self.kind {
            RoutesKind::Styled(_) => panic!("not supported for styled properties"),
            RoutesKind::Direct(routes) => (routes.set_direct_value_unchecked)(self.typed, o, entry),
        }
    }

    pub(crate) fn route_bind(
        &self,
        o: &FerroObject,
        source: Rc<dyn IObservable<BoxedValue>>,
        priority: BindingPriority,
    ) -> Rc<dyn IDisposable> {
        match self.kind {
            RoutesKind::Styled(_) => o.bind_boxed(self.property, source, priority),
            RoutesKind::Direct(routes) => (routes.bind)(self.typed, o, source),
        }
    }

    pub(crate) fn route_get_metadata(&self, type_: &'static TypeInfo) -> FerroPropertyMetadata {
        let metadata = self.metadata.get(type_);
        metadata.erased_base().clone()
    }

    pub(crate) fn route_unregister(&self, type_: &'static TypeInfo) {
        self.metadata.unregister(type_)
    }

    pub(crate) fn route_is_valid_value(&self, value: &dyn Any) -> bool {
        match self.kind {
            RoutesKind::Styled(routes) => (routes.is_valid_value)(self.typed, value),
            RoutesKind::Direct(_) => Any::type_id(value) == self.property.property_type(),
        }
    }

    /// The typed property behind the routes, for downcasting.
    #[inline]
    pub(crate) fn as_any(&self) -> &'static dyn Any {
        self.typed
    }

    /// Clones `value`, which must hold exactly the property's value type `T`,
    /// into `out`, which must be an `&mut Option<T>`. Returns false if either
    /// has a different type.
    #[inline]
    pub(crate) fn route_copy_value(&self, value: &dyn Any, out: &mut dyn Any) -> bool {
        (self.property.value_table().copy_value)(value, out)
    }

    /// Clones `value`, which must hold exactly the property's value type `T`,
    /// into a box of its own. `None` if it has a different type.
    #[inline]
    pub(crate) fn route_box_value(&self, value: &dyn Any) -> Option<BoxedValue> {
        (self.property.value_table().box_value)(value)
    }

    /// Whether the metadata of the property for the type of `owner` enables
    /// data validation. The binding observers of untyped sources read it
    /// once, when they are created.
    pub(crate) fn route_enable_data_validation(&self, owner: &FerroObject) -> Option<bool> {
        let metadata = self.metadata.get(owner.get_type());
        metadata.erased_base().enable_data_validation()
    }

    /// The parts of `value` if it is a `BindingValue<T>` of the property's
    /// value type `T`, with the value in a box of its own; `None` if it is not.
    #[inline]
    pub(crate) fn route_unpack_binding_value(&self, value: &dyn Any) -> Option<BindingValue<BoxedValue>> {
        (self.property.value_table().unpack_binding_value)(value)
    }

    /// Sets the local value of a styled property from a box that holds exactly
    /// the property's value type: the typed `set_local_value` of the store,
    /// for an untyped local binding. Only supported by styled properties.
    pub(crate) fn route_set_local_value(&self, o: &FerroObject, value: BoxedValue) {
        match self.kind {
            RoutesKind::Styled(routes) => (routes.set_local_value)(self.typed, o, value),
            RoutesKind::Direct(_) => panic!("not supported for direct properties"),
        }
    }

    /// Sets a direct property from an untyped binding value whose value, if
    /// any, holds exactly the property's value type: the typed
    /// `set_direct_value_unchecked` of an untyped direct binding. Only
    /// supported by direct properties.
    pub(crate) fn route_set_direct_binding_value(&self, o: &FerroObject, value: BindingValue<BoxedValue>) {
        match self.kind {
            RoutesKind::Styled(_) => panic!("not supported for styled properties"),
            RoutesKind::Direct(routes) => (routes.set_direct_binding_value)(self.typed, o, value),
        }
    }
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
    ///
    /// Dropping the returned guard removes the handler; call
    /// [`detach`](Subscription::detach) to keep it for the life of the
    /// property.
    pub fn subscribe(&'static self, handler: impl Fn(&FerroPropertyChangedEventArgs<'_>) + 'static) -> Subscription {
        let token = self.handlers.add(Rc::new(handler));
        Subscription::new(Disposable::create(move || {
            self.handlers.remove(token);
        }))
    }

    /// Subscribes a handler that is only invoked for objects of class
    /// `TTarget` (or classes derived from it).
    ///
    /// Dropping the returned guard removes the handler; call
    /// [`detach`](Subscription::detach) to keep it for the life of the
    /// property, as a class handler registered once for its class does.
    pub fn add_class_handler<TTarget: ObjectType>(
        &'static self,
        handler: impl Fn(&TTarget, &FerroPropertyChangedEventArgs<'_>) + 'static,
    ) -> Subscription {
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
        crate::perf_count!(PropertyChangesWithPropertyHandlers);
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
    inherited_value_changed: Cell<Option<InheritedValueChanged>>,
    is_attached: bool,
    is_direct: bool,
    is_read_only: bool,
    assign_binding: Cell<bool>,
    host_type: Cell<Option<&'static TypeInfo>>,
    notifying: Option<Rc<dyn Fn(&FerroObject, bool)>>,
    changed: Rc<PropertyChangedObservable>,
    value_table: &'static ValueTable,
    routes: OnceCell<PropertyRoutes>,
}

pub(crate) struct FerroPropertyInit {
    pub inherits: bool,
    pub is_attached: bool,
    pub is_direct: bool,
    pub is_read_only: bool,
}

impl FerroProperty {
    pub(crate) fn new<T: PropertyValue>(
        name: &str,
        owner_type: &'static TypeInfo,
        init: FerroPropertyInit,
        notifying: Option<NotifyingCallback>,
    ) -> Self {
        Self::new_core(
            name,
            owner_type,
            init,
            notifying,
            TypeId::of::<T>(),
            std::any::type_name::<T>(),
            ValueTable::of::<T>(),
        )
    }

    // Identity, allocation and notification setup are independent of the value
    // type. Keep one copy while preserving the original typed property facade.
    #[inline(never)]
    fn new_core(
        name: &str,
        owner_type: &'static TypeInfo,
        init: FerroPropertyInit,
        notifying: Option<NotifyingCallback>,
        property_type: TypeId,
        property_type_name: &'static str,
        value_table: &'static ValueTable,
    ) -> Self {
        assert!(!name.contains('.'), "'name' may not contain periods.");
        Self {
            name: name.into(),
            property_type,
            property_type_name,
            owner_type,
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            inherits: init.inherits,
            inherited_value_changed: Cell::new(None),
            is_attached: init.is_attached,
            is_direct: init.is_direct,
            is_read_only: init.is_read_only,
            assign_binding: Cell::new(false),
            host_type: Cell::new(None),
            notifying: notifying.map(Rc::from),
            changed: Rc::new(PropertyChangedObservable::new()),
            value_table,
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
            inherited_value_changed: source.inherited_value_changed.clone(),
            is_attached: source.is_attached,
            is_direct: source.is_direct,
            is_read_only,
            assign_binding: source.assign_binding.clone(),
            host_type: source.host_type.clone(),
            notifying: source.notifying.clone(),
            changed: source.changed.clone(),
            value_table: source.value_table,
            routes: OnceCell::new(),
        }
    }

    pub(crate) fn set_routes(&self, routes: PropertyRoutes) {
        if self.routes.set(routes).is_err() {
            panic!("property routes are already set");
        }
    }

    // Registration fills this after construction and before exposing the
    // property through the registry. Private inheriting constructors must do
    // the same explicitly; a universal typed constructor must not name the
    // algorithm and thereby retain it for non-inheriting value types.
    pub(crate) fn set_inherited_value_changed(&self, callback: Option<InheritedValueChanged>) {
        debug_assert_eq!(self.inherits, callback.is_some());
        debug_assert!(self.inherited_value_changed.get().is_none());
        self.inherited_value_changed.set(callback);
    }

    pub(crate) fn inherited_value_changed(&self) -> InheritedValueChanged {
        self.inherited_value_changed.get().expect("inheriting property has no change notifier")
    }

    #[inline]
    pub(crate) fn routes(&self) -> &PropertyRoutes {
        self.routes.get().expect("property is not fully registered")
    }

    /// The table of the value type of the property, shared by all the
    /// properties of that type.
    #[inline]
    pub(crate) fn value_table(&self) -> &'static ValueTable {
        self.value_table
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

    // The errors of the typed property routes. They only depend on the
    // property, so they are compiled once instead of once per value type.

    /// The message of a value that is not of the value type of the property.
    #[cold]
    pub(crate) fn invalid_value_message(&self) -> String {
        format!("Invalid value for property '{}': expected {}.", self.name(), self.property_type_name())
    }

    /// Panics for a value that is not of the value type of the property.
    #[cold]
    #[track_caller]
    pub(crate) fn invalid_value_type(&self) -> ! {
        panic!("Invalid value for property '{}': expected {}.", self.name(), self.property_type_name())
    }

    /// Panics for a value that the validation of the property rejects.
    #[cold]
    #[track_caller]
    pub(crate) fn value_not_valid(&self) -> ! {
        panic!("the value is not valid for property '{}'", self.name())
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

    /// The default value of the property for `type_`, untyped: the default of
    /// the metadata of a styled or attached property (upstream's internal
    /// `IStyledPropertyAccessor.GetDefaultValue(Type)`), the unset value of a
    /// direct property. For tests only: the snapshot of the property defaults
    /// (`property_defaults_tests` of `ferroui-controls`) reads it.
    #[cfg(feature = "testing")]
    #[doc(hidden)]
    pub fn default_value_boxed(&self, type_: &'static TypeInfo) -> BoxedValue {
        self.routes().route_get_default_value(type_)
    }

    /// Removes the metadata registered for the property on the specified
    /// type.
    pub fn unregister(&self, type_: &'static TypeInfo) {
        self.routes().route_unregister(type_)
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
        crate::ferro_thread_local! {
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
    #[inline]
    pub fn register_with<TOwner: ObjectType, TValue: PropertyValue>(
        name: &str,
        options: StyledPropertyOptions<TValue>,
    ) -> &'static StyledProperty<TValue> {
        Self::register_with_core(TOwner::TYPE, name, options)
    }

    // Ownership is descriptor data. Only the value's metadata, validation and
    // coercion specialize this installation path; every owner of that value
    // type shares it without changing the typed read/write implementation.
    #[inline(never)]
    fn register_with_core<TValue: PropertyValue>(
        owner_type: &'static TypeInfo,
        name: &str,
        options: StyledPropertyOptions<TValue>,
    ) -> &'static StyledProperty<TValue> {
        let assign_binding = options.assign_binding;
        let inherited_value_changed = options.inherited_value_changed;
        let (metadata, inherits, validate, notifying) = options.into_parts();
        let result = StyledProperty::create(name, owner_type, owner_type, metadata, inherits, validate, notifying, false);
        result.set_inherited_value_changed(inherited_value_changed);
        result.set_assign_binding(assign_binding);
        FerroPropertyRegistry::instance().register(owner_type, result);
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
    #[inline]
    pub fn register_attached_with<TOwner: StaticType, THost: ObjectType, TValue: PropertyValue>(
        name: &str,
        options: StyledPropertyOptions<TValue>,
    ) -> &'static AttachedProperty<TValue> {
        Self::register_attached_with_core(TOwner::TYPE, THost::TYPE, name, options)
    }

    #[inline(never)]
    fn register_attached_with_core<TValue: PropertyValue>(
        owner_type: &'static TypeInfo,
        host_type: &'static TypeInfo,
        name: &str,
        options: StyledPropertyOptions<TValue>,
    ) -> &'static AttachedProperty<TValue> {
        let assign_binding = options.assign_binding;
        let inherited_value_changed = options.inherited_value_changed;
        let (metadata, inherits, validate, notifying) = options.into_parts();
        let result = AttachedProperty::create(name, owner_type, host_type, metadata, inherits, validate, notifying);
        result.set_inherited_value_changed(inherited_value_changed);
        result.set_assign_binding(assign_binding);
        result.host_type.set(Some(host_type));
        let registry = FerroPropertyRegistry::instance();
        registry.register(owner_type, result);
        registry.register_attached(host_type, result);
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
    inherited_value_changed: Option<InheritedValueChanged>,
    default_binding_mode: BindingMode,
    validate: Option<Box<dyn Fn(&T) -> bool>>,
    coerce: Option<Coercion<T>>,
    enable_data_validation: bool,
    assign_binding: bool,
    notifying: Option<NotifyingCallback>,
}

impl<T: PropertyValue> StyledPropertyOptions<T> {
    pub fn new(default_value: T) -> Self {
        Self {
            default_value,
            inherits: false,
            inherited_value_changed: None,
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
        self.inherited_value_changed = if inherits {
            Some(EffectiveValue::<T>::notify_inherited_value_changed)
        } else {
            None
        };
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
        self.coerce = Some(Coercion::new(Rc::new(coerce)));
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
            // Transfer the already registered callback and algorithm. Naming
            // the algorithm here would root it for every options value type,
            // including properties that never supplied a coercer.
            metadata = metadata.with_registered_coercion(coerce);
        }
        (metadata, self.inherits, self.validate, self.notifying)
    }
}

#[cfg(test)]
mod styled_property_options_tests {
    use super::*;

    #[test]
    fn registration_transfers_the_original_coercer() {
        let options = StyledPropertyOptions::new(0_i32).coerce(|_, n| n.max(0));
        let registered = options.coerce.as_ref().unwrap().callback().clone();
        let (metadata, _, _, _) = options.into_parts();
        assert!(Rc::ptr_eq(&registered, metadata.coerce_value().unwrap()));
    }
}

#[cfg(test)]
mod value_table_tests {
    use super::*;
    use std::cell::RefCell;
    use std::sync::atomic::AtomicUsize;

    fn unique_name(prefix: &str) -> String {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        format!("{prefix}{}", NEXT.fetch_add(1, Ordering::Relaxed))
    }

    #[test]
    fn properties_of_one_value_type_share_one_table() {
        let plain = FerroProperty::register::<FerroObject, i32>(&unique_name("SharedTablePlain"), 1);
        let coerced = FerroProperty::register_with::<FerroObject, i32>(
            &unique_name("SharedTableCoerced"),
            StyledPropertyOptions::new(2).coerce(|_, n| n),
        );
        assert!(std::ptr::eq(plain.value_table(), coerced.value_table()));
        assert!(std::ptr::eq(plain.value_table(), ValueTable::of::<i32>()));
    }

    #[test]
    fn values_of_distinct_types_stay_distinct() {
        let number = FerroProperty::register::<FerroObject, i32>(&unique_name("DistinctNumber"), 1);
        let text = FerroProperty::register::<FerroObject, String>(&unique_name("DistinctText"), String::new());
        assert!(!std::ptr::eq(number.value_table(), text.value_table()));

        let table = number.value_table();
        assert!((table.box_value)(&String::from("five")).is_none());
        assert_eq!((table.box_value)(&5_i32).unwrap().downcast_ref::<i32>(), Some(&5));
        let mut text_out = None::<String>;
        assert!(!(table.copy_value)(&5_i32, &mut text_out));
        assert!(text_out.is_none());
        let mut number_out = None::<i32>;
        assert!((table.copy_value)(&7_i32, &mut number_out));
        assert_eq!(number_out, Some(7));
        assert!(!number.is_valid_value(&String::new()));
        assert!(text.is_valid_value(&String::new()));

        // Untyped writes and reads go through the shared routes and the table.
        let o = FerroObject::new();
        o.set_value_untyped(number, &9_i32, BindingPriority::LocalValue);
        o.set_value_untyped(text, &String::from("nine"), BindingPriority::LocalValue);
        assert_eq!(o.get_value_untyped(number).downcast_ref::<i32>(), Some(&9));
        assert_eq!(o.get_value_untyped(text).downcast_ref::<String>().map(String::as_str), Some("nine"));
        assert_eq!(o.get_value(number), 9);
        o.clear_value_untyped(number);
        assert_eq!(o.get_value(number), 1);
        assert_eq!(o.get_value_untyped(number).downcast_ref::<i32>(), Some(&1));
        assert_eq!(o.get_value(text), "nine");
    }

    #[test]
    fn coercion_and_change_notifications_through_the_shared_paths() {
        let calls = Rc::new(Cell::new(0));
        let counted = calls.clone();
        let property = FerroProperty::register_with::<FerroObject, i32>(
            &unique_name("SharedCoercion"),
            StyledPropertyOptions::new(0)
                .coerce(move |_, n: i32| {
                    counted.set(counted.get() + 1);
                    n.clamp(0, 10)
                })
                .inherits(true),
        );
        let parent = FerroObject::new();
        let child = FerroObject::new();
        child.set_inheritance_parent(&parent);

        type Change = (&'static str, Option<i32>, i32, BindingPriority);
        let changes: Rc<RefCell<Vec<Change>>> = Rc::new(RefCell::new(Vec::new()));
        let record = |name: &'static str| {
            let changes = changes.clone();
            move |e: &FerroPropertyChangedEventArgs<'_>| {
                if e.property() == property.as_property() {
                    changes.borrow_mut().push((name, e.get_old_value::<i32>(), e.get_new_value::<i32>(), e.priority()));
                }
            }
        };
        let _parent = parent.property_changed(record("parent"));
        let _child = child.property_changed(record("child"));

        parent.set_value(property, 25);
        assert_eq!((parent.get_value(property), child.get_value(property)), (10, 10));
        // Coerced to the value it has: neither the value nor the base value
        // changes, so nothing is raised.
        parent.set_value(property, 30);
        parent.set_value_untyped(property, &-4_i32, BindingPriority::LocalValue);
        assert_eq!((parent.get_value(property), child.get_value(property)), (0, 0));
        assert_eq!(calls.get(), 3);
        assert_eq!(
            *changes.borrow(),
            vec![
                ("parent", Some(0), 10, BindingPriority::LocalValue),
                ("child", Some(0), 10, BindingPriority::Inherited),
                ("parent", Some(10), 0, BindingPriority::LocalValue),
                ("child", Some(10), 0, BindingPriority::Inherited),
            ]
        );

        // A value that is not coerced (the coerced default) takes the other
        // branch of the write and still notifies through the shared path.
        let plain = FerroProperty::register::<FerroObject, String>(&unique_name("SharedPlain"), String::from("a"));
        let seen = Rc::new(RefCell::new(Vec::new()));
        let observed = seen.clone();
        let _plain = parent.property_changed(move |e| {
            if e.property() == plain.as_property() {
                observed.borrow_mut().push(e.get_old_and_new_value::<String>());
            }
        });
        parent.set_value(plain, String::from("b"));
        parent.set_value(plain, String::from("b"));
        parent.clear_value(plain);
        assert_eq!(
            *seen.borrow(),
            vec![(String::from("a"), String::from("b")), (String::from("b"), String::from("a"))]
        );
    }
}

/// Records the accessor `$name` of `$owner` returning `$property`
/// (`metadata::record_property_accessor`) with the `compiler-metadata`
/// feature of this crate; expands to nothing without it.
#[cfg(feature = "compiler-metadata")]
#[doc(hidden)]
#[macro_export]
macro_rules! __record_property_accessor {
    ($owner:ident, $vis:vis, $name:ident, $property:expr) => {
        $crate::metadata::record_property_accessor(
            <$owner as $crate::StaticType>::TYPE,
            ::std::any::TypeId::of::<Self>(),
            ::std::stringify!($name),
            ::std::stringify!($vis),
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
    ($owner:ident, $vis:vis, $name:ident, $property:expr) => {};
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
            $crate::ferro_thread_local! {
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
                            $crate::__register_value_markup!(property);
                            $crate::__record_property_accessor!($owner, $vis, $name, property);
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
            $crate::ferro_thread_local! {
                static CELL: ::std::cell::Cell<::std::option::Option<&'static $ty>> =
                    const { ::std::cell::Cell::new(::std::option::Option::None) };
            }
            match CELL.get() {
                ::std::option::Option::Some(property) => property,
                ::std::option::Option::None => {
                    let property: &'static $ty = $body;
                    CELL.set(::std::option::Option::Some(property));
                    $crate::__register_value_markup!(property);
                    property
                }
            }
        }
    };
}

/// Registers the markup metadata of the value type of a property that is registered
/// ([`MarkupType::register_used`](crate::metadata::MarkupType::register_used)), when the value
/// type has metadata ([`MarkupTyped`](crate::metadata::MarkupTyped)); nothing otherwise. The
/// choice is made where the property is declared, by the method a reference to a reference to
/// the probe resolves to: the implementation for a value type with metadata (on `&Probe`) takes
/// that receiver as it is and is found first; the one of every type (on `Probe`) is found after
/// one dereference.
#[doc(hidden)]
#[macro_export]
macro_rules! __register_value_markup {
    ($property:expr) => {{
        #[allow(unused_imports)]
        use $crate::{__NoValueMarkup as _, __RegisterValueMarkup as _};
        (&&$crate::__ValueMarkupProbe($property)).__register_value_markup();
    }};
}

#[doc(hidden)]
pub struct __ValueMarkupProbe<'a, P: ?Sized>(pub &'a P);

#[doc(hidden)]
pub trait __RegisterValueMarkup {
    fn __register_value_markup(&self);
}

impl<T: PropertyValue + crate::metadata::MarkupTyped> __RegisterValueMarkup
    for &__ValueMarkupProbe<'_, crate::StyledProperty<T>>
{
    #[inline(always)]
    fn __register_value_markup(&self) {
        crate::metadata::MarkupType::register_used(T::MARKUP);
    }
}

impl<T: PropertyValue + crate::metadata::MarkupTyped> __RegisterValueMarkup
    for &__ValueMarkupProbe<'_, crate::AttachedProperty<T>>
{
    #[inline(always)]
    fn __register_value_markup(&self) {
        crate::metadata::MarkupType::register_used(T::MARKUP);
    }
}

impl<O: crate::ObjectType, T: PropertyValue + crate::metadata::MarkupTyped> __RegisterValueMarkup
    for &__ValueMarkupProbe<'_, crate::DirectProperty<O, T>>
{
    #[inline(always)]
    fn __register_value_markup(&self) {
        crate::metadata::MarkupType::register_used(T::MARKUP);
    }
}

// A nullable value type (`Option<T>`) is found by its metadata too (the nullable form of the type).
impl<T: PropertyValue + crate::metadata::MarkupTyped> __RegisterValueMarkup
    for &__ValueMarkupProbe<'_, crate::StyledProperty<Option<T>>>
{
    #[inline(always)]
    fn __register_value_markup(&self) {
        crate::metadata::MarkupType::register_used(T::MARKUP);
    }
}

impl<T: PropertyValue + crate::metadata::MarkupTyped> __RegisterValueMarkup
    for &__ValueMarkupProbe<'_, crate::AttachedProperty<Option<T>>>
{
    #[inline(always)]
    fn __register_value_markup(&self) {
        crate::metadata::MarkupType::register_used(T::MARKUP);
    }
}

impl<O: crate::ObjectType, T: PropertyValue + crate::metadata::MarkupTyped> __RegisterValueMarkup
    for &__ValueMarkupProbe<'_, crate::DirectProperty<O, Option<T>>>
{
    #[inline(always)]
    fn __register_value_markup(&self) {
        crate::metadata::MarkupType::register_used(T::MARKUP);
    }
}

#[doc(hidden)]
pub trait __NoValueMarkup {
    #[inline(always)]
    fn __register_value_markup(&self) {}
}

impl<P: ?Sized> __NoValueMarkup for __ValueMarkupProbe<'_, P> {}

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
