//! Member lookup by name on a value: the object-level counterpart of the
//! reflection calls of the managed original (`GetType().GetRuntimeProperty`,
//! `GetRuntimeMethods`, `GetRuntimeEvent`), answered from markup metadata.
//!
//! A value of the class hierarchy is described by the metadata of its
//! run-time class and of its base classes ([`TypeInfo::markup`](crate::TypeInfo::markup));
//! any other value by the registered metadata of the type it is a handle of
//! ([`MarkupType::find_by_handle`]) and the chain of its
//! [`MarkupType::base_type`]. Only members declared in metadata are found:
//! the same rule the string-path bindings follow.

use super::{MarkupEvent, MarkupMethod, MarkupProperty, MarkupType};
use crate::data::core::ValueTypes;
use crate::AnyValue;

/// The metadata of the run-time type of `value` and of its base types, most
/// derived first: for an object of the class hierarchy the metadata of its
/// class and of each base class that declares any, for any other value the
/// registered type the value is a handle of and the chain of its
/// [`MarkupType::base_type`].
///
/// The contracts (interfaces) a type implements are not part of the list:
/// their members are reached through the handle of the contract.
pub fn markup_types_of(value: &dyn AnyValue) -> Vec<&'static MarkupType> {
    let mut result = Vec::new();
    if let Some(object) = ValueTypes::as_object(value) {
        let mut current = Some(object.get_type());
        while let Some(class) = current {
            if let Some(markup) = class.markup() {
                result.push(markup);
            }
            current = class.base_type();
        }
        return result;
    }
    let mut current = MarkupType::find_by_handle(value.value_type_id());
    while let Some(markup) = current {
        // A base chain that leads back to itself ends the walk.
        if result.iter().any(|seen| std::ptr::eq(*seen, markup)) {
            break;
        }
        result.push(markup);
        current = markup.base_type();
    }
    result
}

/// The plain (non-registered) property `name` of the type of `value`, or of
/// the nearest base type that declares one (`GetRuntimeProperty(name)`).
/// Registered properties are found through the property registry instead.
pub fn find_markup_property(value: &dyn AnyValue, name: &str) -> Option<&'static MarkupProperty> {
    markup_types_of(value).into_iter().find_map(|markup| markup.find_property(name))
}

/// The methods named `name` of the type of `value` and of its base types,
/// every overload, most derived type first (`GetRuntimeMethods()` filtered
/// by name).
///
/// As with reflection over the run-time type, a method that a deriving type
/// declares again with the same parameter types (an override, or a method
/// that hides the one of the base type) is listed once, as the most derived
/// declaration, and the static methods of base types are not members of the
/// type.
pub fn find_markup_methods(value: &dyn AnyValue, name: &str) -> Vec<&'static MarkupMethod> {
    let mut result: Vec<&'static MarkupMethod> = Vec::new();
    for (depth, markup) in markup_types_of(value).into_iter().enumerate() {
        for method in markup.find_methods(name) {
            if method.is_static && depth > 0 {
                continue;
            }
            if !result.iter().any(|known| same_parameters(known, method)) {
                result.push(method);
            }
        }
    }
    result
}

/// The plain (non-routed) event `name` of the type of `value`, or of the
/// nearest base type that declares one (`GetRuntimeEvent(name)`). Routed
/// events are found with
/// [`RoutedEventRegistry::find_by_name`](crate::interactivity::RoutedEventRegistry::find_by_name).
pub fn find_markup_event(value: &dyn AnyValue, name: &str) -> Option<&'static MarkupEvent> {
    markup_types_of(value).into_iter().find_map(|markup| markup.find_event(name))
}

/// Whether two methods take the same parameter types.
pub(crate) fn same_parameters(a: &MarkupMethod, b: &MarkupMethod) -> bool {
    a.parameters.len() == b.parameters.len() && a.parameters.iter().zip(b.parameters).all(|(a, b)| a() == b())
}
