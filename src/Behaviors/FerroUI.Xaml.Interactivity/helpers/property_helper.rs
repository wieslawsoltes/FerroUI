//! Port of `Helpers/PropertyHelper.cs`.

use super::member_lookup::{
    find_owner_types, find_plain_property, object_of, property_value_type, registered_attached, registered_inherited,
    type_name_of,
};
use super::TypeConverterHelper;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::BindingPriority;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, FerroPropertyRegistry, TypeInfo};
use std::rc::Rc;

const TRIM_CHARS: &[char] = &['(', ')'];

/// Reads and writes the properties of objects by name: the registered properties of objects of
/// the class model (`Name`, and attached properties as `(Owner.Name)`), and the plain properties
/// of any value that declares them in its markup metadata.
///
/// Errors are the argument exceptions of the reference, as their messages.
pub struct PropertyHelper;

impl PropertyHelper {
    fn is_type_name_in_hierarchy(target_type: &'static TypeInfo, type_name: &str) -> bool {
        let mut current_type = Some(target_type);
        while let Some(type_) = current_type {
            if type_.name() == type_name {
                return true;
            }

            current_type = type_.base_type();
        }

        false
    }

    /// Finds the attached property `property_name` of the owner type named `owner_type_name`
    /// that objects of `target_type` can have, after initialising the owner type: the scan of the
    /// loaded assemblies of the reference for the owner type and its static field
    /// `{Name}Property`.
    fn initialize_owner_and_find_attached_property(
        target_type: &'static TypeInfo,
        owner_type_name: &str,
        property_name: &str,
    ) -> Option<&'static FerroProperty> {
        for owner_type in find_owner_types(owner_type_name) {
            let candidate = FerroPropertyRegistry::instance()
                .get_declared(owner_type)
                .into_iter()
                .find(|property| property.name() == property_name && std::ptr::eq(property.owner_type(), owner_type));
            let Some(candidate) = candidate else {
                continue;
            };

            if registered_attached(target_type).into_iter().any(|property| std::ptr::eq(property, candidate)) {
                return Some(candidate);
            }
        }

        None
    }

    fn find_ferro_attached_property(target_object: &FerroObject, property_name: &str) -> Option<&'static FerroProperty> {
        let property_names: Vec<&str> = property_name.trim().trim_matches(TRIM_CHARS).split('.').collect();
        if property_names.len() != 2 {
            return None;
        }
        let target_property_type_name = property_names[0];
        let target_property_name = property_names[1];
        let target_type = target_object.get_type();

        for ferro_property in registered_attached(target_type) {
            if ferro_property.owner_type().name() == target_property_type_name && ferro_property.name() == target_property_name {
                return Some(ferro_property);
            }
        }

        if let Some(initialized_attached_property) =
            Self::initialize_owner_and_find_attached_property(target_type, target_property_type_name, target_property_name)
        {
            return Some(initialized_attached_property);
        }

        for ferro_property in registered_inherited(target_type) {
            if (ferro_property.owner_type().name() == target_property_type_name
                || Self::is_type_name_in_hierarchy(target_type, target_property_type_name))
                && ferro_property.name() == target_property_name
            {
                return Some(ferro_property);
            }
        }

        None
    }

    /// Converts `value` for a plain property of type `property_type`: null to the null of the
    /// type, a value of the type (or of a type assignable to it) as it is, anything else through
    /// its text.
    fn convert_value(property_type: ValueType, value: Option<&BoxedValue>) -> Result<Option<BoxedValue>, String> {
        let Some(value) = value else {
            // The result can be null if the type is nullable, or the default value of the type.
            return ValueTypes::try_convert(None, property_type)
                .ok_or_else(|| format!("Cannot assign null to a value of type {}.", property_type.name()));
        };

        if let Some(converted) = ValueTypes::try_cast(value, property_type) {
            return Ok(Some(converted));
        }

        let Some(value_as_string) = ValueTypes::try_to_string(&**value) else {
            return Ok(None);
        };

        match TypeConverterHelper::convert(&value_as_string, property_type) {
            Ok(Some(converted)) => Ok(Some(converted)),
            Ok(None) => Ok(Some(value.clone())),
            Err(error) => Err(error.to_string()),
        }
    }

    fn update_clr_property_value(target_object: &BoxedValue, property_name: &str, value: Option<&BoxedValue>) -> Result<(), String> {
        let target_type_name = type_name_of(target_object);
        let Some(property_info) = find_plain_property(&**target_object, property_name) else {
            return Err(format!("Cannot find a property named {property_name} on type {target_type_name}."));
        };
        let Some(set) = property_info.set else {
            return Err(format!("Property {property_name} on type {target_type_name} is read-only."));
        };

        let property_type = (property_info.type_)();
        let invalid_assignment = || {
            format!(
                "Cannot assign value of type {} to property {} of type {}. The {} property can be assigned only values of type {}.",
                value.map_or_else(|| "null".to_string(), type_name_of),
                property_name,
                property_type.name(),
                property_name,
                property_type.name()
            )
        };

        let result = Self::convert_value(property_type, value).map_err(|_| invalid_assignment())?;
        set(&[Some(target_object.clone()), result]).map(|_| ()).map_err(|_| invalid_assignment())
    }

    fn try_get_clr_property_value(target_object: &BoxedValue, property_name: &str) -> Result<Option<Option<BoxedValue>>, String> {
        let target_type_name = type_name_of(target_object);
        let Some(property_info) = find_plain_property(&**target_object, property_name) else {
            return Err(format!("Cannot find a property named {property_name} on type {target_type_name}."));
        };

        let Some(get) = property_info.get else {
            return Ok(None);
        };

        let value = get(&[Some(target_object.clone())]).map_err(|error| format!("{error:?}"))?;
        Ok(Some(value.and_then(ValueTypes::normalize)))
    }

    fn validate_ferro_property(property: Option<&'static FerroProperty>, property_name: &str) -> Result<&'static FerroProperty, String> {
        match property {
            None => Err(format!("Cannot find a property named {property_name}.")),
            Some(property) if property.is_read_only() => Err(format!("Property {property_name} is read-only.")),
            Some(property) => Ok(property),
        }
    }

    fn update_ferro_property_value(
        ferro_object: &FerroObject,
        property: &'static FerroProperty,
        property_name: &str,
        value: Option<&BoxedValue>,
        preserve_value_source: bool,
    ) -> Result<(), String> {
        let property = Self::validate_ferro_property(Some(property), property_name)?;

        let result = Self::convert_ferro_property_value(property, value)
            .map_err(|_| Self::create_invalid_assignment_exception(ferro_object, property_name, value))?;

        if preserve_value_source {
            ferro_object.set_current_value_untyped(property, result.as_any());
        } else {
            let _ = ferro_object.set_value_untyped(property, result.as_any(), BindingPriority::LocalValue);
        }

        Ok(())
    }

    /// Sets a registered property of `target_object` with the animation priority, so that
    /// disposing the returned handle reverts the change. `Ok(None)` when the target is not an
    /// object, has no such property or the property is direct.
    pub fn try_set_temporary_ferro_property_value(
        target_object: &BoxedValue,
        property_name: &str,
        value: Option<&BoxedValue>,
    ) -> Result<Option<Rc<dyn IDisposable>>, String> {
        let Some(ferro_object) = object_of(target_object) else {
            return Ok(None);
        };

        let property = if property_name.contains('.') {
            Self::find_ferro_attached_property(&ferro_object, property_name)
        } else {
            FerroPropertyRegistry::instance().find_registered_for(&ferro_object, property_name)
        };
        let Some(property) = property.filter(|property| !property.is_direct()) else {
            return Ok(None);
        };

        let property = Self::validate_ferro_property(Some(property), property_name)?;
        let result = Self::convert_ferro_property_value(property, value)
            .map_err(|_| Self::create_invalid_assignment_exception(&ferro_object, property_name, value))?;
        Ok(ferro_object.set_value_untyped(property, result.as_any(), BindingPriority::Animation))
    }

    /// Whether `property_name` names a direct property of `target_object`.
    pub fn is_direct_ferro_property(target_object: &BoxedValue, property_name: &str) -> bool {
        let Some(ferro_object) = object_of(target_object) else {
            return false;
        };

        let property = if property_name.contains('.') {
            Self::find_ferro_attached_property(&ferro_object, property_name)
        } else {
            FerroPropertyRegistry::instance().find_registered_for(&ferro_object, property_name)
        };
        property.is_some_and(FerroProperty::is_direct)
    }

    /// Converts `value` to exactly the value type of `property` (what the untyped setters of the
    /// framework take).
    fn convert_ferro_property_value(property: &'static FerroProperty, value: Option<&BoxedValue>) -> Result<BoxedValue, String> {
        let property_type = property_value_type(property);
        let converted = Self::convert_value(property_type, value)?;
        ValueTypes::try_convert(converted.as_ref(), property_type)
            .flatten()
            .or_else(|| ValueTypes::null_value(property_type).filter(|_| converted.is_none()))
            .ok_or_else(|| format!("The value is not of type {}.", property_type.name()))
    }

    fn create_invalid_assignment_exception(ferro_object: &FerroObject, property_name: &str, value: Option<&BoxedValue>) -> String {
        format!(
            "Cannot assign value of type {} to property {} of type {}. The {} property can be assigned only values of type {}.",
            value.map_or_else(|| "null".to_string(), type_name_of),
            property_name,
            ferro_object.get_type().name(),
            property_name,
            ferro_object.get_type().name()
        )
    }

    /// Sets the property `property_name` of `target_object` to `value`, converting the value to
    /// the type of the property (`preserve_value_source` sets the current value of a registered
    /// property instead of its local value). Returns false when an attached property named
    /// `(Owner.Name)` is not found.
    ///
    /// # Errors
    /// The property is not found, is read-only, or the value does not convert to its type.
    pub fn update_property_value(
        target_object: &BoxedValue,
        property_name: &str,
        value: Option<&BoxedValue>,
        preserve_value_source: bool,
    ) -> Result<bool, String> {
        if let Some(ferro_object) = object_of(target_object) {
            if property_name.contains('.') {
                if let Some(ferro_property) = Self::find_ferro_attached_property(&ferro_object, property_name) {
                    Self::update_ferro_property_value(&ferro_object, ferro_property, property_name, value, preserve_value_source)?;
                    return Ok(true);
                }

                return Ok(false);
            } else if let Some(ferro_property) =
                FerroPropertyRegistry::instance().find_registered_for(&ferro_object, property_name)
            {
                Self::update_ferro_property_value(&ferro_object, ferro_property, property_name, value, preserve_value_source)?;
                return Ok(true);
            }
        }

        Self::update_clr_property_value(target_object, property_name, value)?;
        Ok(true)
    }

    /// Reads the property `property_name` of `target_object`: `Ok(Some((value,
    /// preserve_value_source)))` when it is read (`preserve_value_source` is true for a registered
    /// property), `Ok(None)` for an attached property that is not found or a plain property that
    /// cannot be read.
    ///
    /// # Errors
    /// A plain property of the name is not found.
    pub fn try_get_property_value(
        target_object: &BoxedValue,
        property_name: &str,
    ) -> Result<Option<(Option<BoxedValue>, bool)>, String> {
        if let Some(ferro_object) = object_of(target_object) {
            if property_name.contains('.') {
                if let Some(ferro_property) = Self::find_ferro_attached_property(&ferro_object, property_name) {
                    let value = ValueTypes::normalize(ferro_object.get_value_untyped(ferro_property));
                    return Ok(Some((value, true)));
                }

                return Ok(None);
            }

            if let Some(registered_property) = FerroPropertyRegistry::instance().find_registered_for(&ferro_object, property_name) {
                let value = ValueTypes::normalize(ferro_object.get_value_untyped(registered_property));
                return Ok(Some((value, true)));
            }
        }

        Ok(Self::try_get_clr_property_value(target_object, property_name)?.map(|value| (value, false)))
    }
}
