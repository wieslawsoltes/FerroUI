use super::{IPropertyAccessor, IPropertyAccessorPlugin};
use crate::data::core::{IPropertyInfo, WeakValue};
use crate::AnyValue;
use std::rc::Rc;

/// Creates the accessor for a property description and a source object.
pub type PropertyAccessorFactory = Rc<dyn Fn(&WeakValue, Rc<dyn IPropertyInfo>) -> Rc<dyn IPropertyAccessor>>;

/// The plugin of a compiled binding path element: a property description
/// together with the factory of its accessor.
pub struct PropertyInfoAccessorPlugin {
    property_info: Rc<dyn IPropertyInfo>,
    accessor_factory: PropertyAccessorFactory,
}

impl PropertyInfoAccessorPlugin {
    pub fn new(property_info: Rc<dyn IPropertyInfo>, accessor_factory: PropertyAccessorFactory) -> Self {
        Self { property_info, accessor_factory }
    }
}

impl IPropertyAccessorPlugin for PropertyInfoAccessorPlugin {
    fn match_(&self, _obj: &dyn AnyValue, _property_name: &str) -> bool {
        panic!("The PropertyInfoAccessorPlugin does not support dynamic matching");
    }

    fn start(&self, reference: &WeakValue, property_name: &str) -> Option<Rc<dyn IPropertyAccessor>> {
        debug_assert!(property_name == self.property_info.name());
        Some((self.accessor_factory)(reference, self.property_info.clone()))
    }
}
