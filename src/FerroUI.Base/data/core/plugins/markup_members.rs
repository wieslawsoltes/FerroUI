//! Member lookup over markup metadata: what the string-path (reflection)
//! bindings use where the managed original reflects over the run-time type
//! of a source object.

use crate::data::core::{IPropertyInfo, ValueType, ValueTypes};
use crate::data::BindingError;
use crate::metadata::{MarkupIndexer, MarkupMethod, MarkupProperty, MarkupType, MarkupValue};
use crate::{AnyValue, BoxedValue};

/// The metadata of the run-time type of `value` and of its base types, most
/// derived first: for an object of the class hierarchy the metadata of its
/// class and base classes, for any other value the registered type the value
/// is a handle of and the chain of its [`MarkupType::base`].
pub(crate) fn types_of(value: &dyn AnyValue) -> Vec<&'static MarkupType> {
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

/// The dotted full name of the run-time type of `value` (the `GetType()` of
/// the managed original, printed).
pub(crate) fn type_name_of(value: &dyn AnyValue) -> String {
    match ValueTypes::as_object(value) {
        Some(object) => object.get_type().full_name(),
        None => ValueTypes::type_full_name(ValueType::of_value(value)),
    }
}

/// The plain property `name` of the type of `value`, or of the nearest base
/// type that declares one.
pub(crate) fn find_property(value: &dyn AnyValue, name: &str) -> Option<&'static MarkupProperty> {
    types_of(value).into_iter().find_map(|markup| markup.find_property(name))
}

/// Whether the type of `value` or one of its base types declares a method
/// named `name`.
pub(crate) fn has_method(value: &dyn AnyValue, name: &str) -> bool {
    types_of(value).into_iter().any(|markup| markup.find_methods(name).next().is_some())
}

/// The indexer of the type of `value`: the one of the most derived type
/// that declares any. Of several indexers of that type, the one that takes
/// `argument_count` arguments, or else the first.
pub(crate) fn find_indexer(value: &dyn AnyValue, argument_count: usize) -> Option<&'static MarkupIndexer> {
    let markup = types_of(value).into_iter().find(|markup| !markup.indexers.is_empty())?;
    markup.find_indexer(argument_count).or_else(|| markup.indexers.first())
}

/// The outcome of looking up the method a binding can turn into a command.
pub(crate) enum MethodLookup {
    /// No method of that name.
    None,
    /// The method and the type that declares it.
    Method(&'static MarkupType, &'static MarkupMethod),
    /// Methods of that name exist but none can be chosen.
    Error(String),
}

fn same_parameters(a: &MarkupMethod, b: &MarkupMethod) -> bool {
    a.parameters.len() == b.parameters.len() && a.parameters.iter().zip(b.parameters).all(|(a, b)| a() == b())
}

/// Finds the method named `name` which can be bound to a command.
///
/// Priority:
///  1. One parameter method
///    1a. "Any value" parameter (amongst several overloads)
///    1b. Single method with one parameter
///  2. Zero parameters method
pub(crate) fn find_best_command_method(value: &dyn AnyValue, name: &str) -> MethodLookup {
    // A method declared again by a deriving type (an override, or a method
    // that hides the one of the base type) counts once: the most derived
    // declaration.
    let mut candidates: Vec<(&'static MarkupType, &'static MarkupMethod)> = Vec::new();
    for (depth, markup) in types_of(value).into_iter().enumerate() {
        for method in markup.find_methods(name) {
            // The static methods of base types are not members of the type.
            if method.is_static && depth > 0 {
                continue;
            }
            if !candidates.iter().any(|(_, known)| same_parameters(known, method)) {
                candidates.push((markup, method));
            }
        }
    }
    if candidates.is_empty() {
        return MethodLookup::None;
    }

    let zero_parameter = candidates.iter().find(|(_, method)| method.parameters.is_empty());
    let one_parameter: Vec<_> = candidates.iter().filter(|(_, method)| method.parameters.len() == 1).collect();

    if !one_parameter.is_empty() {
        // The "any value" parameter always wins.
        if let Some((markup, method)) = one_parameter.iter().find(|(_, method)| method.parameters[0]().is_object()) {
            return MethodLookup::Method(markup, method);
        }
        if let [(markup, method)] = one_parameter[..] {
            return MethodLookup::Method(markup, method);
        }
        let mut parameter_types: Vec<String> = one_parameter
            .iter()
            .map(|(_, method)| format!("'{}'", ValueTypes::type_full_name(method.parameters[0]())))
            .collect();
        parameter_types.sort();
        return MethodLookup::Error(format!(
            "Unable to resolve method of name '{name}' on type '{}'. \
             Found {} overloads accepting one parameter: {}. \
             Expected either a single overload with one parameter, or an overload accepting System.Object.",
            type_name_of(value),
            parameter_types.len(),
            parameter_types.join(", ")
        ));
    }

    if let Some((markup, method)) = zero_parameter {
        return MethodLookup::Method(markup, method);
    }

    MethodLookup::Error(format!(
        "Unable to resolve method of name '{name}' on type '{}'. \
         Found {} overloads accepting more than one parameter. \
         Expected a method with zero or one parameter.",
        type_name_of(value),
        candidates.len()
    ))
}

/// The error of an invoker as a binding error: what the member of the
/// managed original throws.
pub(crate) fn invoke_error(error: crate::metadata::MarkupInvokeError) -> BindingError {
    BindingError::message(error.to_string())
}

/// A plain property declared in markup metadata as a property description.
/// Its accessors take the handle of the owner, so it is read and written
/// through the boxed forms of the contract.
pub(crate) struct MarkupPropertyInfo(pub(crate) &'static MarkupProperty);

impl MarkupPropertyInfo {
    fn needs_handle(&self) -> BindingError {
        BindingError::message(format!(
            "Property '{}' is accessed through the handle of its owner.",
            self.0.name
        ))
    }
}

impl IPropertyInfo for MarkupPropertyInfo {
    fn name(&self) -> &str {
        self.0.name
    }

    fn get(&self, _target: &dyn AnyValue) -> Option<BoxedValue> {
        None
    }

    fn try_get(&self, _target: &dyn AnyValue) -> Result<Option<BoxedValue>, BindingError> {
        Err(self.needs_handle())
    }

    fn set(&self, _target: &dyn AnyValue, _value: Option<&BoxedValue>) -> Result<(), BindingError> {
        Err(self.needs_handle())
    }

    fn get_boxed(&self, target: &BoxedValue) -> Option<BoxedValue> {
        self.try_get_boxed(target).ok().flatten()
    }

    fn try_get_boxed(&self, target: &BoxedValue) -> Result<Option<BoxedValue>, BindingError> {
        let get = self
            .0
            .get
            .ok_or_else(|| BindingError::message(format!("Property '{}' has no getter.", self.0.name)))?;
        get(&[Some(target.clone())]).map_err(invoke_error)
    }

    fn set_boxed(&self, target: &BoxedValue, value: Option<&BoxedValue>) -> Result<(), BindingError> {
        let set = self
            .0
            .set
            .ok_or_else(|| BindingError::message(format!("Property '{}' has no setter.", self.0.name)))?;
        set(&[Some(target.clone()), untyped(value)]).map(|_| ()).map_err(invoke_error)
    }

    fn can_set(&self) -> bool {
        self.0.set.is_some()
    }

    fn can_get(&self) -> bool {
        self.0.get.is_some()
    }

    fn property_type(&self) -> ValueType {
        (self.0.type_)()
    }
}

/// A value converted to the type of a member, as the argument of an
/// invoker: the contents of an "any value" box, null for a null nullable.
pub(crate) fn untyped(value: Option<&BoxedValue>) -> MarkupValue {
    let value = value?;
    if let Some(any) = value.downcast_ref::<Option<BoxedValue>>() {
        return any.clone();
    }
    if let Some(any) = value.downcast_ref::<BoxedValue>() {
        return Some(any.clone());
    }
    Some(value.clone())
}

/// The source of a binding as the instance its members are invoked on: the
/// object form of a reference type held as its handle `Rc<T>`, so that the
/// instance can be held weakly and viewed as a notifier; the value itself
/// otherwise.
pub(crate) fn instance_of(value: &BoxedValue) -> BoxedValue {
    ValueTypes::reference_object(value).unwrap_or_else(|| value.clone())
}
