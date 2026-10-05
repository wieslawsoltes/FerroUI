//! Port of `MarkupExtensions/ResolveByNameExtension.cs`.

use crate::object_casts::{box_object, TargetProperty};
use crate::{IProvideValueTarget, ServiceProviderExtensions};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::{ferro_markup_type, BoxedValue, FerroObject, FerroProperty, Ref};
use std::rc::Rc;

/// Resolves a named control of the name scope: what text assigned to a
/// property marked as resolved by name stands for.
pub struct ResolveByNameExtension {
    name: String,
}

impl ResolveByNameExtension {
    pub fn new(name: &str) -> Rc<Self> {
        Rc::new(Self { name: name.to_string() })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// The named control; null (`None`) without a name scope; the unset
    /// value marker if the name is not registered yet, in which case the
    /// target property is set once it is.
    pub fn provide_value(&self, service_provider: &Rc<dyn IServiceProvider>) -> Option<BoxedValue> {
        Self::provide_value_for(service_provider, &self.name)
    }

    /// The resolved element as the value of the property it is resolved for.
    /// The managed runtime assigns the object to a property of any type the
    /// object is an instance of; a registered property of this port holds a
    /// value of exactly its type (a handle of the declared class, or a
    /// reference to an element it does not own), so the element is brought
    /// to that type where it is one. For a property of any object, or a
    /// plain property, it is the handle of the element's own class.
    fn element_value(element: Ref<FerroObject>, stored_as: Option<ValueType>) -> BoxedValue {
        let boxed = box_object(element);
        match stored_as {
            Some(type_) if !type_.is_object() => match ValueTypes::try_convert(Some(&boxed), type_) {
                Some(Some(stored)) => stored,
                _ => boxed,
            },
            _ => boxed,
        }
    }

    fn provide_value_for(service_provider: &Rc<dyn IServiceProvider>, name: &str) -> Option<BoxedValue> {
        let name_scope = service_provider.get_name_scope()?;

        let value = name_scope.find_async(name);

        let provide_value_target = service_provider.get_service_of::<Rc<dyn IProvideValueTarget>>();
        let target_property =
            provide_value_target.as_ref().and_then(|target| TargetProperty::of(target.target_property().as_ref()));
        // The type the element is stored as by the property it is resolved for.
        let stored_as = match &target_property {
            Some(TargetProperty::Ferro(property)) => {
                Some(ValueType::new(property.property_type(), property.property_type_name()))
            }
            _ => None,
        };

        if value.is_completed() {
            return value.get_result().map(|element| Self::element_value(element, stored_as));
        }

        if let (Some(provide_value_target), Some(property)) = (provide_value_target, target_property) {
            let property = property.into_property_info();
            let target = provide_value_target.target_object();
            let result = value.clone();
            value.on_completed(move || {
                if let Some(target) = &target {
                    let element = result.get_result().map(|element| Self::element_value(element, stored_as));
                    // A failing setter has no caller to report to here.
                    let _ = property.set(&**target, element.as_ref());
                }
            });
        }

        Some(FerroProperty::unset_value())
    }
}

crate::identity_eq!(ResolveByNameExtension);

ferro_markup_type!(class ResolveByNameExtension {
    this: Rc<ResolveByNameExtension>,
    handles: [ResolveByNameExtension, Rc<ResolveByNameExtension>, Option<Rc<ResolveByNameExtension>>],
    constructors: [(String) => |name: String| ResolveByNameExtension::new(&name)],
    properties: [
        Name: String { get: |this: &Rc<ResolveByNameExtension>| this.name().to_string() },
    ],
    methods: [
        fn ProvideValue(Rc<dyn IServiceProvider>) -> Option<BoxedValue> =>
            |this: &Rc<ResolveByNameExtension>, service_provider: Rc<dyn IServiceProvider>| {
                this.provide_value(&service_provider)
            },
    ],
});
