use crate::data::{BindingBase, BindingExpressionBase};
use crate::{FerroObject, FerroProperty, Ref, StyledElement};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const CLASS_PROPERTY_PREFIX: &str = "__FerroReserved::Classes::";

thread_local! {
    static REGISTERED_PROPERTIES: RefCell<HashMap<String, &'static FerroProperty>> = RefCell::new(HashMap::new());
}

/// Implements bindings to individual style classes of an element
/// (`Classes.name="{Binding ...}"`): each class gets an intermediate boolean
/// property whose value adds or removes the class.
pub struct ClassBindingManager;

impl ClassBindingManager {
    /// Binds the presence of the class `class_name` on `target` to `source`.
    pub fn bind(
        target: &StyledElement,
        class_name: &str,
        source: &dyn BindingBase,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        let property = Self::get_class_property(class_name);
        target.bind_binding_with_anchor(property, source, anchor)
    }

    fn register_class_proxy_property(class_name: &str) -> &'static FerroProperty {
        let property =
            FerroProperty::register::<StyledElement, bool>(&format!("{CLASS_PROPERTY_PREFIX}{class_name}"), false);
        let class_name = class_name.to_string();
        property.changed().subscribe(move |args| {
            if let Some(element) = args.sender().downcast_ref::<StyledElement>() {
                let value = args.get_new_value::<bool>();
                element.classes().set(&class_name, value);
            }
        });
        property
    }

    /// The intermediate property for the class `class_name`.
    pub fn get_class_property(class_name: &str) -> &'static FerroProperty {
        let prefixed = format!("{CLASS_PROPERTY_PREFIX}{class_name}");
        let existing = REGISTERED_PROPERTIES.with(|p| p.borrow().get(&prefixed).copied());
        match existing {
            Some(property) => property,
            None => {
                let property = Self::register_class_proxy_property(class_name);
                REGISTERED_PROPERTIES.with(|p| p.borrow_mut().insert(prefixed, property));
                property
            }
        }
    }

    /// Whether `property` is the intermediate property of a class binding;
    /// if so, returns the class name.
    pub fn is_classes_binding_property(property: &FerroProperty) -> Option<&str> {
        let name = property.name();
        let prefix_len = CLASS_PROPERTY_PREFIX.len();
        if name.len() >= prefix_len && name[..prefix_len].eq_ignore_ascii_case(CLASS_PROPERTY_PREFIX) {
            // As in the managed implementation, the reported name starts one
            // character past the prefix.
            Some(name.get(prefix_len + 1..).unwrap_or(""))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_class_property_should_return_same_instance_for_same_class() {
        let property1 = ClassBindingManager::get_class_property("Foo");
        let property2 = ClassBindingManager::get_class_property("Foo");
        assert!(std::ptr::eq(property1, property2));
    }

    #[test]
    fn get_class_property_should_return_different_instances_for_different_classes() {
        let property1 = ClassBindingManager::get_class_property("Foo");
        let property2 = ClassBindingManager::get_class_property("Bar");
        assert!(!std::ptr::eq(property1, property2));
    }
}
