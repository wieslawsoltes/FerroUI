use super::{AccessorListener, IPropertyAccessor, IPropertyAccessorPlugin, PropertyAccessorBase, PropertyError};
use crate::data::core::{ValueType, ValueTypes, WeakValue};
use crate::data::{BindingError, BindingErrorType, BindingNotification, BindingPriority};
use crate::reactive::IDisposable;
use crate::{AnyValue, BoxedValue, FerroObject, FerroProperty, FerroPropertyRegistry, Ref, WeakRef};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// The value type of a registered property.
pub(crate) fn property_value_type(property: &FerroProperty) -> ValueType {
    ValueType::new(property.property_type(), property.property_type_name())
}

/// Reads a registered property as a binding value (nullable values are
/// normalised to null or their contents).
pub(crate) fn get_property_value(o: &FerroObject, property: &'static FerroProperty) -> Option<BoxedValue> {
    ValueTypes::normalize(o.get_value_untyped(property))
}

/// Writes a binding value to a registered property, converting it to the
/// property's exact type.
pub(crate) fn set_property_value(
    o: &FerroObject,
    property: &'static FerroProperty,
    value: Option<&BoxedValue>,
    priority: BindingPriority,
) -> Result<(), BindingError> {
    if value.is_some_and(|v| v.is::<crate::UnsetValueType>() || v.is::<crate::DoNothingType>()) {
        o.set_value_untyped(property, (**value.expect("checked")).as_any(), priority);
        return Ok(());
    }
    let target = property_value_type(property);
    match ValueTypes::try_convert(value, target) {
        Some(Some(v)) if property.is_valid_value((*v).as_any()) => {
            o.set_value_untyped(property, (*v).as_any(), priority);
            Ok(())
        }
        _ => Err(BindingError::message(format!(
            "Invalid value for Property '{}': '{}' ({})",
            property.name(),
            ValueTypes::to_display_string(value),
            value.map_or("(null)", |v| (**v).type_name())
        ))),
    }
}

/// Reads a registered property from an object of the class hierarchy.
pub struct FerroPropertyAccessorPlugin;

fn lookup_property(o: &FerroObject, property_name: &str) -> Option<&'static FerroProperty> {
    FerroPropertyRegistry::instance().find_registered_for(o, property_name)
}

impl IPropertyAccessorPlugin for FerroPropertyAccessorPlugin {
    fn match_(&self, obj: &dyn AnyValue, property_name: &str) -> bool {
        match ValueTypes::as_object(obj) {
            Some(o) => lookup_property(&o, property_name).is_some(),
            None => false,
        }
    }

    fn start(&self, reference: &WeakValue, property_name: &str) -> Option<Rc<dyn IPropertyAccessor>> {
        let instance = reference.upgrade()?;
        let o = ValueTypes::as_object(&*instance)?;
        match lookup_property(&o, property_name) {
            Some(p) => Some(FerroPropertyAccessor::new(Some(o.downgrade()), p)),
            None => {
                let message = format!("Could not find FerroProperty '{}' on '{}'", property_name, o.get_type());
                Some(Rc::new(PropertyError::new(BindingNotification::with_error(
                    BindingError::message(message),
                    BindingErrorType::Error,
                ))))
            }
        }
    }
}

/// The accessor of a registered property of an object of the class
/// hierarchy: reads and writes the property and follows its changes.
pub struct FerroPropertyAccessor {
    this: Weak<FerroPropertyAccessor>,
    base: PropertyAccessorBase,
    reference: Option<WeakRef<FerroObject>>,
    property: &'static FerroProperty,
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl FerroPropertyAccessor {
    /// Creates an accessor for `property` of the object behind
    /// `reference`. `None` is a reference without a target (the weak
    /// reference to null of the managed original): such an accessor has no
    /// value.
    pub fn new(reference: Option<WeakRef<FerroObject>>, property: &'static FerroProperty) -> Rc<dyn IPropertyAccessor> {
        Rc::new_cyclic(|this| FerroPropertyAccessor {
            this: this.clone(),
            base: PropertyAccessorBase::new(),
            reference,
            property,
            subscription: RefCell::new(None),
        })
    }

    /// The object the accessor reads from, if it is still alive.
    pub fn instance(&self) -> Option<Ref<FerroObject>> {
        self.reference.as_ref()?.upgrade()
    }

    fn send_current_value(&self) {
        let value = self.value();
        self.base.publish_value(value);
    }
}

impl IDisposable for FerroPropertyAccessor {
    fn dispose(&self) {
        if self.base.is_subscribed() {
            self.unsubscribe();
        }
    }
}

impl IPropertyAccessor for FerroPropertyAccessor {
    fn property_type(&self) -> Option<ValueType> {
        Some(property_value_type(self.property))
    }

    fn value(&self) -> Option<BoxedValue> {
        self.instance().and_then(|o| get_property_value(&o, self.property))
    }

    fn set_value(&self, value: Option<&BoxedValue>, priority: BindingPriority) -> Result<bool, BindingError> {
        if self.property.is_read_only() {
            return Ok(false);
        }
        if let Some(o) = self.instance() {
            set_property_value(&o, self.property, value, priority)?;
        }
        Ok(true)
    }

    fn subscribe(&self, listener: AccessorListener) {
        self.base.set_listener(listener);
        if let Some(instance) = self.instance() {
            let weak = self.this.clone();
            let id = self.property.id();
            let subscription = instance.property_changed(move |e| {
                if e.property().id() == id {
                    if let Some(this) = weak.upgrade() {
                        this.send_current_value();
                    }
                }
            });
            *self.subscription.borrow_mut() = Some(subscription);
        }
        self.send_current_value();
    }

    fn unsubscribe(&self) {
        if let Some(s) = self.subscription.borrow_mut().take() {
            s.dispose();
        }
        self.base.clear_listener();
    }
}

impl Drop for FerroPropertyAccessor {
    fn drop(&mut self) {
        if let Some(s) = self.subscription.get_mut().take() {
            s.dispose();
        }
    }
}
