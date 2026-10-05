use super::plugins::{get_property_value, property_value_type, set_property_value};
use super::{ValueType, ValueTypes};
use crate::data::{BindingError, BindingPriority};
use crate::{AnyValue, BoxedValue, FerroObject, FerroProperty, FerroPropertyRegistry, Ref};

/// Describes a property of an object that is not a registered property: its
/// name, type and untyped accessors.
pub trait IPropertyInfo {
    /// The name of the property.
    fn name(&self) -> &str;

    /// Reads the property from `target`. Null is `None`.
    fn get(&self, target: &dyn AnyValue) -> Option<BoxedValue>;

    /// Reads the property from `target`. An `Err` is the equivalent of the
    /// getter throwing, for example because `target` is not of the type that
    /// declares the property.
    fn try_get(&self, target: &dyn AnyValue) -> Result<Option<BoxedValue>, BindingError> {
        Ok(self.get(target))
    }

    /// Writes the property on `target`. An `Err` is the equivalent of the
    /// setter throwing.
    fn set(&self, target: &dyn AnyValue, value: Option<&BoxedValue>) -> Result<(), BindingError>;

    fn can_set(&self) -> bool;

    fn can_get(&self) -> bool;

    /// The type of the property's value.
    fn property_type(&self) -> ValueType;

    /// The implementing value, for casting to its concrete type
    /// (`as_any()?.downcast_ref::<T>()`): the equivalent of `is`/`as` on the
    /// contract. `None` for an implementation that does not expose itself.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }

    /// [`get`](Self::get) for an owner known as an untyped handle. The
    /// accessors of binding paths read through the boxed forms, so that a
    /// property description whose accessors need the handle of the owner
    /// (not only a view of it) can override them; the defaults forward to
    /// the forms taking a view. Override the three boxed forms together.
    fn get_boxed(&self, target: &BoxedValue) -> Option<BoxedValue> {
        self.get(&**target)
    }

    /// [`try_get`](Self::try_get) for an owner known as an untyped handle.
    fn try_get_boxed(&self, target: &BoxedValue) -> Result<Option<BoxedValue>, BindingError> {
        self.try_get(&**target)
    }

    /// [`set`](Self::set) for an owner known as an untyped handle.
    fn set_boxed(&self, target: &BoxedValue, value: Option<&BoxedValue>) -> Result<(), BindingError> {
        self.set(&**target, value)
    }

    /// The registered property this description is, if it is one (the cast
    /// of a property description back to the registered property class).
    fn as_ferro_property(&self) -> Option<&'static FerroProperty> {
        None
    }
}

/// A registered property is a property info: the untyped accessors read and
/// write the property on an object of the class hierarchy.
impl IPropertyInfo for FerroProperty {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn name(&self) -> &str {
        FerroProperty::name(self)
    }

    /// Panics if `target` is not an object of the class hierarchy.
    fn get(&self, target: &dyn AnyValue) -> Option<BoxedValue> {
        match self.try_get(target) {
            Ok(value) => value,
            Err(error) => panic!("{error}"),
        }
    }

    fn try_get(&self, target: &dyn AnyValue) -> Result<Option<BoxedValue>, BindingError> {
        let object = target_object(self, target)?;
        Ok(get_property_value(&object, registered(self)))
    }

    fn set(&self, target: &dyn AnyValue, value: Option<&BoxedValue>) -> Result<(), BindingError> {
        let object = target_object(self, target)?;
        set_property_value(&object, registered(self), value, BindingPriority::LocalValue)
    }

    fn can_set(&self) -> bool {
        !self.is_read_only()
    }

    fn can_get(&self) -> bool {
        true
    }

    fn property_type(&self) -> ValueType {
        property_value_type(self)
    }

    fn as_ferro_property(&self) -> Option<&'static FerroProperty> {
        Some(registered(self))
    }
}

/// A registered property behind a shared handle (property definitions are
/// `'static` and cannot themselves be put behind an `Rc`).
struct RegisteredPropertyInfo(&'static FerroProperty);

impl IPropertyInfo for RegisteredPropertyInfo {
    fn name(&self) -> &str {
        FerroProperty::name(self.0)
    }

    fn get(&self, target: &dyn AnyValue) -> Option<BoxedValue> {
        IPropertyInfo::get(self.0, target)
    }

    fn try_get(&self, target: &dyn AnyValue) -> Result<Option<BoxedValue>, BindingError> {
        IPropertyInfo::try_get(self.0, target)
    }

    fn set(&self, target: &dyn AnyValue, value: Option<&BoxedValue>) -> Result<(), BindingError> {
        IPropertyInfo::set(self.0, target, value)
    }

    fn can_set(&self) -> bool {
        IPropertyInfo::can_set(self.0)
    }

    fn can_get(&self) -> bool {
        IPropertyInfo::can_get(self.0)
    }

    fn property_type(&self) -> ValueType {
        IPropertyInfo::property_type(self.0)
    }

    fn as_ferro_property(&self) -> Option<&'static FerroProperty> {
        Some(self.0)
    }
}

impl FerroProperty {
    /// The property as a shared property description
    /// (`Rc<dyn IPropertyInfo>`), for the places that take a registered
    /// property through the property description contract.
    /// [`IPropertyInfo::as_ferro_property`] gives the property back.
    pub fn as_property_info(&'static self) -> std::rc::Rc<dyn IPropertyInfo> {
        std::rc::Rc::new(RegisteredPropertyInfo(self))
    }
}

/// The registered (and so `'static`) definition of a property.
fn registered(property: &FerroProperty) -> &'static FerroProperty {
    FerroPropertyRegistry::instance()
        .find_registered_by_id(property.id())
        .expect("a property definition is registered when it is created")
}

fn target_object(property: &FerroProperty, target: &dyn AnyValue) -> Result<Ref<FerroObject>, BindingError> {
    ValueTypes::as_object(target).ok_or_else(|| {
        BindingError::message(format!(
            "Unable to access Property '{}': the target ({}) is not an object.",
            property.name(),
            target.type_name()
        ))
    })
}

/// Handles compare by identity (reference equality), so that they can be
/// held in property and untyped values.
impl PartialEq for dyn IPropertyInfo {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
