use super::{ISetterInstance, ITemplate};
use crate::property_store::IValueEntry;
use crate::{AnyValue, BoxedValue, FerroProperty};
use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

/// The instance of a setter whose value is a template: the template is built
/// the first time the value is read.
pub(crate) struct PropertySetterTemplateInstance {
    template: Rc<dyn ITemplate>,
    value: RefCell<Option<BoxedValue>>,
    property: &'static FerroProperty,
}

impl PropertySetterTemplateInstance {
    pub fn new(property: &'static FerroProperty, template: Rc<dyn ITemplate>) -> Self {
        Self { template, value: RefCell::new(None), property }
    }

    fn get_value(&self) -> BoxedValue {
        if let Some(value) = &*self.value.borrow() {
            return value.clone();
        }
        let built = self.template.build();
        // The object the template built is a value of the property when it is
        // assignable to its type (the managed runtime casts it): it is held
        // as a value of exactly that type.
        let type_ = crate::data::core::ValueType::new(self.property.property_type(), self.property.property_type_name());
        let value = crate::data::core::ValueTypes::try_cast(&built, type_).unwrap_or(built);
        *self.value.borrow_mut() = Some(value.clone());
        value
    }
}

impl ISetterInstance for PropertySetterTemplateInstance {}

impl IValueEntry for PropertySetterTemplateInstance {
    fn property(&self) -> &'static FerroProperty {
        self.property
    }

    fn has_value(&self) -> bool {
        true
    }

    fn try_get_value(&self, out: &mut dyn Any) -> bool {
        let value = self.get_value();
        let value: &dyn AnyValue = &*value;
        if self.property.routes().route_copy_value(value.as_any(), out) {
            true
        } else {
            panic!(
                "The template of the setter for property '{}' built a value of type {}: expected {}.",
                self.property.name(),
                value.type_name(),
                self.property.property_type_name()
            );
        }
    }

    fn get_value_boxed(&self) -> Option<BoxedValue> {
        Some(self.get_value())
    }

    fn unsubscribe(&self) {}
}
