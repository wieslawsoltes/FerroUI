//! UI-independent native metadata over the original recycling model interfaces.
mod contract_metadata;
#[doc(hidden)]
pub use contract_metadata::{IControlRecyclingIdProviderMetadata, IControlRecyclingMetadata};
pub use ferrodock_controls_recycling_model::{IControlRecycling, IControlRecyclingIdProvider};
use ferroui_base::{
    data::core::ValueTypes,
    metadata::{MarkupAssembly, MarkupType, MarkupTyped},
};
use std::rc::Rc;
/// Original logical recycling-model assembly identity.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Controls.Recycling.Model",
    crate_name: "ferrodock_controls_recycling_model_ferro",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {classes:[],types:[],contracts:[crate::IControlRecyclingMetadata,crate::IControlRecyclingIdProviderMetadata],generics:[],generic_contracts:[]}
/// Registers source contract metadata and identities without importing UI controls.
pub fn register_types() {
    ferroui_base::register_types();
    register_rust_paths();
    MarkupAssembly::register(&ASSEMBLY);
    MarkupType::register_all(&[
        <dyn IControlRecyclingMetadata>::MARKUP,
        <dyn IControlRecyclingIdProviderMetadata>::MARKUP,
    ]);
    ValueTypes::register_nullable::<Rc<dyn IControlRecycling>>();
    ValueTypes::register_nullable::<Rc<dyn IControlRecyclingIdProvider>>();
    ValueTypes::register_reference_identity::<Rc<dyn IControlRecycling>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
    ValueTypes::register_reference_identity::<Rc<dyn IControlRecyclingIdProvider>>(|value| {
        Rc::as_ptr(value).cast::<()>()
    });
}

#[cfg(test)]
mod metadata_tests;
