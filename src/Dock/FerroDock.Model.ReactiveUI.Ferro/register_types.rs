//! Original model assembly identity and UI-only metadata composition.
use ferroui_base::metadata::MarkupAssembly;
/// The original model assembly identity; engine/model code remains UI independent.
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroDock.Model.ReactiveUI",
    crate_name: "ferrodock_model_reactiveui_ferro",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};
ferroui_base::ferro_rust_paths! {
    classes: [],
    types: [crate::reactive_metadata::ReactiveObjectReference,crate::reactive_metadata::ReactiveBaseReference,crate::reactive_metadata::DockableBaseReference,crate::reactive_metadata::DockBaseReference,crate::reactive_metadata::DocumentReference,crate::reactive_metadata::ToolReference,crate::reactive_metadata::DocumentDockReference,crate::reactive_metadata::RootDockReference,crate::reactive_metadata::ToolDockReference,crate::reactive_metadata::ProportionalDockReference,crate::reactive_metadata::StackDockReference,crate::reactive_metadata::GridDockReference,crate::reactive_metadata::DockDockReference,crate::reactive_metadata::WrapDockReference,crate::reactive_metadata::UniformGridDockReference,crate::reactive_metadata::SplitViewDockReference,crate::reactive_metadata::ProportionalDockSplitterReference,crate::reactive_metadata::GridDockSplitterReference,crate::reactive_metadata::DockWindowReference,crate::reactive_metadata::FactoryReference],
    contracts: [], generics: [], generic_contracts: [],
}
/// Registers original model class metadata; applications register controls separately.
pub fn register_types() {
    ferrodock_model_ferro::register_types();
    register_rust_paths();
    crate::reactive_metadata::register_reactive_model_classes();
    MarkupAssembly::register(&ASSEMBLY);
}
