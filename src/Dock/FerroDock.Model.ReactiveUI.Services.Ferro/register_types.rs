//! Native class and namespace discovery for derived markup views.
use crate::controls::DockReactiveUserControlBase;
use ferroui_base::metadata::MarkupAssembly;
use ferroui_base::{StaticType, TypeInfo};
const NAMESPACES: &[(&str, &str)] = &[(
    "ferrodock_model_reactiveui_services_ferro::controls",
    "FerroDock.Model.ReactiveUI.Services.Ferro.Controls",
)];
/// Source library identity for native markup and dependent metadata export.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Model.ReactiveUI.Services.Ferro",
    crate_name: "ferrodock_model_reactiveui_services_ferro",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {
    classes: [crate::controls::DockReactiveUserControlBase],
    types: [], contracts: [], generics: [], generic_contracts: [],
}

/// Registers the shared framework and this library's immutable type descriptor.
/// Class metadata intentionally exposes no default constructor for the abstract base.
pub fn register_types() {
    ferroui_reactiveui_controls::register_types();
    TypeInfo::register_namespaces(NAMESPACES);
    TypeInfo::register_all(&[<DockReactiveUserControlBase as StaticType>::TYPE]);
    register_rust_paths();
    MarkupAssembly::register(&ASSEMBLY);
}
