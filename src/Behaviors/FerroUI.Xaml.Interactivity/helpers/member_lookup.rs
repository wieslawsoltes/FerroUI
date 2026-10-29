//! Not from the reference: the lookups of members by name that replace the reflection of the
//! reference (`GetRuntimeProperty`, `GetRuntimeMethods`, `GetRuntimeEvent`, the scan of the loaded
//! assemblies for the owner of an attached property, `IComparable`), in one place.
//!
//! Deviation (DEVIATIONS.md, Behaviors): members are found through the markup metadata of the
//! types (the plain properties, methods and events a type declares), the registered properties
//! and routed events, and the known types; only types that declare metadata are reachable, which
//! is the rule string-path bindings follow already.

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::interactivity::{RoutedEvent, RoutedEventRegistry};
use ferroui_base::metadata::{MarkupDelegate, MarkupEvent, MarkupMethod, MarkupProperty};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{AnyValue, BoxedValue, FerroObject, FerroProperty, FerroPropertyRegistry, Ref, TypeInfo};
use std::cmp::Ordering;
use std::rc::Rc;

/// The object of the class model behind an untyped value, if it holds one.
pub fn object_of(value: &BoxedValue) -> Option<Ref<FerroObject>> {
    ValueTypes::as_object(&**value)
}

/// The name of the type of an untyped value, for messages (`GetType().Name`).
pub fn type_name_of(value: &BoxedValue) -> String {
    match object_of(value) {
        Some(object) => object.get_type().name().to_string(),
        None => {
            let name = ValueType::of_value(&**value).name();
            name.rsplit("::").next().unwrap_or(name).to_string()
        }
    }
}

/// The value type of a registered property (`property.PropertyType`).
pub fn property_value_type(property: &'static FerroProperty) -> ValueType {
    ValueType::new(property.property_type(), property.property_type_name())
}

/// A plain (non-registered) property of the value by name (`GetRuntimeProperty(name)`).
pub fn find_plain_property(value: &dyn AnyValue, name: &str) -> Option<&'static MarkupProperty> {
    ferroui_base::metadata::members::find_markup_property(value, name)
}

/// The methods of the value with the name (`GetRuntimeMethods()` filtered by name).
pub fn find_methods(value: &dyn AnyValue, name: &str) -> Vec<&'static MarkupMethod> {
    ferroui_base::metadata::members::find_markup_methods(value, name).into_iter().collect()
}

/// A plain (non-routed) event of the value by name (`GetRuntimeEvent(name)`).
pub fn find_plain_event(value: &dyn AnyValue, name: &str) -> Option<&'static MarkupEvent> {
    ferroui_base::metadata::members::find_markup_event(value, name)
}

/// Subscribes `handler` to a plain event of `instance`; the subscription is ended by disposing the
/// returned handle (`EventInfo.AddEventHandler` / `RemoveEventHandler`).
pub fn subscribe_plain_event(
    event: &'static MarkupEvent,
    instance: &BoxedValue,
    handler: MarkupDelegate,
) -> Option<Rc<dyn IDisposable>> {
    // `AddEventHandler` throws for a target that is not an instance of the declaring type.
    event.subscribe(Some(instance.clone()), handler).unwrap_or_else(|error| panic!("{error}"))
}

/// A routed event of the class of `owner` or of its base classes by name, with or without the
/// `Event` suffix (the routed event behind a CLR event of the reference).
pub fn find_routed_event(owner: &'static TypeInfo, name: &str) -> Option<RoutedEvent> {
    RoutedEventRegistry::instance().find_by_name(owner, name)
}

/// Compares two values that are comparable with each other (`IComparable.CompareTo`): numbers,
/// text, characters, booleans, enumerations and the types with a registered comparer. `None` when
/// they are not comparable with each other.
pub fn compare_values(left: &BoxedValue, right: &BoxedValue) -> Option<Ordering> {
    ValueTypes::compare(left, right)
}

/// Whether the value is comparable (`value is IComparable`): it compares with itself.
pub fn is_comparable(value: &BoxedValue) -> bool {
    compare_values(value, value).is_some()
}

/// The registered properties of `type_` that are attached properties (`GetRegisteredAttached`).
pub fn registered_attached(type_: &'static TypeInfo) -> Vec<&'static FerroProperty> {
    FerroPropertyRegistry::instance().get_registered_attached(type_).iter().copied().collect()
}

/// The registered properties of `type_` that inherit their value (`GetRegisteredInherited`).
pub fn registered_inherited(type_: &'static TypeInfo) -> Vec<&'static FerroProperty> {
    FerroPropertyRegistry::instance().get_registered_inherited(type_).iter().copied().collect()
}

/// The known types with the name, initialised: the owner type of an attached property named in
/// markup's `(Owner.Property)` syntax (the reference scans the loaded assemblies for it and reads
/// its static field `{Name}Property`, which runs its static constructor). A dotted name is a full
/// name (namespace and name).
pub fn find_owner_types(owner_type_name: &str) -> Vec<&'static TypeInfo> {
    let types: Vec<&'static TypeInfo> = match owner_type_name.rsplit_once('.') {
        Some((namespace, name)) => TypeInfo::find(namespace, name).into_iter().collect(),
        None => TypeInfo::registered_types().into_iter().filter(|type_| type_.name() == owner_type_name).collect(),
    };
    for type_ in &types {
        type_.ensure_class_init();
    }
    types
}
