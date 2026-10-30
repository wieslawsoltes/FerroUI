//! Core-owned capability policy handles and their original identity-preserving casts.
use crate::{
    DockCapabilityOverridesMetadata, DockCapabilityPolicyMetadata, IDockCapabilityPolicyMetadata,
};
use ferrodock_model::core::{
    DockCapabilityOverrides, DockCapabilityPolicy, DockCapabilityPolicyImpl, IDockCapabilityPolicy,
};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use std::rc::Rc;
pub(crate) fn register() {
    MarkupType::register_all(&[
        <dyn IDockCapabilityPolicyMetadata>::MARKUP,
        <dyn DockCapabilityPolicyMetadata>::MARKUP,
        <dyn DockCapabilityOverridesMetadata>::MARKUP,
    ]);
    ValueTypes::register_nullable::<Rc<dyn IDockCapabilityPolicy>>();
    ValueTypes::register_nullable::<Rc<dyn DockCapabilityPolicyImpl>>();
    ValueTypes::register_reference::<DockCapabilityPolicy>();
    ValueTypes::register_reference::<DockCapabilityOverrides>();
    ValueTypes::register_reference_identity::<Rc<dyn DockCapabilityPolicyImpl>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_reference_identity::<Rc<dyn IDockCapabilityPolicy>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_cast::<Rc<DockCapabilityPolicy>, Rc<dyn DockCapabilityPolicyImpl>>(
        |value| value.clone(),
    );
    ValueTypes::register_cast::<Rc<DockCapabilityPolicy>, Rc<dyn IDockCapabilityPolicy>>(|value| {
        value.clone()
    });
    ValueTypes::register_cast::<Rc<DockCapabilityOverrides>, Rc<dyn DockCapabilityPolicyImpl>>(
        |value| value.clone(),
    );
    ValueTypes::register_cast::<Rc<DockCapabilityOverrides>, Rc<dyn IDockCapabilityPolicy>>(
        |value| value.clone(),
    );
    ValueTypes::register_cast::<Rc<dyn DockCapabilityPolicyImpl>, Rc<dyn IDockCapabilityPolicy>>(
        |value| value.clone(),
    );
    ferroui_base::__ferro_compiler_metadata! {
        ferroui_base::metadata::register_type_rust_paths(&[
            (||std::any::TypeId::of::<Rc<dyn DockCapabilityPolicyImpl>>(),"std::rc::Rc<dyn ferrodock_model::core::DockCapabilityPolicyImpl>"),
            (||std::any::TypeId::of::<Rc<DockCapabilityOverrides>>(),"std::rc::Rc<ferrodock_model::core::DockCapabilityOverrides>"),
        ]);
    }
}
