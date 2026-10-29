//! Port of `Validation/Contract/IValidationRule.cs` (`IValidationRule<in T>`).

use ferroui_base::{BoxedValue, FerroObject, ObjectType, TypeInfo};
use std::sync::RwLock;

/// Defines a validation rule.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference contract is generic over the type of the
/// validated value (`IValidationRule<in T>`); here the value is untyped (the value of the
/// validated property, null when the property holds no value), and a rule that expects another
/// type of value does not validate it. The rules are objects of the class model, so a class states
/// that it implements the contract in its static constructor
/// (`<dyn IValidationRule>::register::<MyRule>()`), and the contract finds the implementation of
/// an object through its runtime type (`<dyn IValidationRule>::of(object)`), as the contracts of
/// the interactivity crate do.
pub trait IValidationRule {
    /// Gets the error message for failed validation.
    fn error_message(&self) -> Option<String>;

    /// Sets the error message for failed validation.
    fn set_error_message(&self, value: Option<String>);

    /// Validates the specified value (null when the property holds no value).
    ///
    /// Returns `true` if value is valid.
    fn validate(&self, value: Option<&BoxedValue>) -> bool;
}

type Cast = for<'a> fn(&'a FerroObject) -> Option<&'a (dyn IValidationRule + 'static)>;

/// The classes that implement the contract and their casts (process wide: runtime types are
/// static data; the casts are plain functions).
static TABLE: RwLock<Vec<(&'static TypeInfo, Cast)>> = RwLock::new(Vec::new());

fn cast<'a, T: ObjectType + IValidationRule>(object: &'a FerroObject) -> Option<&'a (dyn IValidationRule + 'static)> {
    object.downcast_ref::<T>().map(|object| object as &(dyn IValidationRule + 'static))
}

impl dyn IValidationRule {
    /// States that the objects of class `T` (and of the classes deriving from it) implement the
    /// contract. Registering a class again replaces its cast.
    pub fn register<T: ObjectType + IValidationRule>() {
        let mut entries = TABLE.write().unwrap_or_else(|e| e.into_inner());
        match entries.iter_mut().find(|entry| std::ptr::eq(entry.0, T::TYPE)) {
            Some(entry) => entry.1 = cast::<T>,
            None => entries.push((T::TYPE, cast::<T>)),
        }
    }

    /// The implementation of the contract for `object`, if its class or a base class of it
    /// registered one (`object as IValidationRule<T>`).
    pub fn of(object: &FerroObject) -> Option<&dyn IValidationRule> {
        let mut current = Some(object.get_type());
        while let Some(type_) = current {
            let found = {
                let entries = TABLE.read().unwrap_or_else(|e| e.into_inner());
                entries.iter().find(|entry| std::ptr::eq(entry.0, type_)).map(|entry| entry.1)
            };
            if let Some(cast) = found {
                return cast(object);
            }
            current = type_.base_type();
        }
        None
    }
}

/// The text a validated value holds (a `string` value of the reference), if it is text.
pub(crate) fn text_of(value: Option<&BoxedValue>) -> Option<String> {
    let value = value?;
    if let Some(text) = value.downcast_ref::<String>() {
        return Some(text.clone());
    }
    if let Some(text) = value.downcast_ref::<Option<String>>() {
        return text.clone();
    }
    if let Some(text) = value.downcast_ref::<&'static str>() {
        return Some((*text).to_string());
    }
    None
}
