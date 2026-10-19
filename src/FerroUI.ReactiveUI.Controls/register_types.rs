//! The type table of this crate: its namespace, its classes and what it states about itself for
//! markup (`Attributes.cs`: the `XmlnsDefinition` of the assembly).
//!
//! The classes are the non-generic ones: markup creates [`ReactiveUserControlBase`] and
//! [`ReactiveWindowBase`] (a document whose root is a reactive user control or window), and the
//! two view hosts. Deviation (DEVIATIONS.md, ReactiveUI controls): the generic
//! `ReactiveUserControl<TViewModel>` and `ReactiveWindow<TViewModel>` are typed handles of Rust
//! ([`ReactiveView`](crate::ReactiveView)), not classes, and markup does not name them; a document
//! names the base class, and the code behind it types the instance with
//! `ReactiveUserControl::<VM>::from_control`.

use crate::{ReactiveUserControlBase, ReactiveWindowBase, RoutedViewHost, ViewModelViewHost};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition};
use ferroui_base::{StaticType, TypeInfo};
use ferroui_reactiveui::RoutingState;

/// The XML namespace of ReactiveUI (`http://reactiveui.net`).
pub const REACTIVEUI_XML_NAMESPACE: &str = "http://reactiveui.net";

/// The dotted namespace of the modules of this crate (upstream's namespace, with the name of the
/// framework).
const NAMESPACES: &[(&str, &str)] = &[("ferroui_reactiveui_controls", "ReactiveUI.FerroUI")];

/// What this crate states about itself for markup: its assembly name and the XML namespace of
/// ReactiveUI mapped to its namespace (`[assembly: XmlnsDefinition("http://reactiveui.net", ..)]`).
pub static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "FerroUI.ReactiveUI.Controls",
    crate_name: "ferroui_reactiveui_controls",
    xmlns_definitions: &[XmlnsDefinition { xml_namespace: REACTIVEUI_XML_NAMESPACE, namespace: "ReactiveUI.FerroUI" }],
    xmlns_prefixes: &[],
    metadata: &[],
};

const TYPES: &[&TypeInfo] = &[
    <ReactiveUserControlBase as StaticType>::TYPE,
    <ReactiveWindowBase as StaticType>::TYPE,
    <RoutedViewHost as StaticType>::TYPE,
    <ViewModelViewHost as StaticType>::TYPE,
];

/// Registers the namespace, the types and the assembly of this crate (and of the crates it is
/// built on). Cheap and idempotent.
pub fn register_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        ferroui_controls::register_types();
        TypeInfo::register_namespaces(NAMESPACES);
        TypeInfo::register_all(TYPES);
        crate::rust_paths::register_rust_paths();
        MarkupAssembly::register(&ASSEMBLY);
        ValueTypes::register_global(register_value_types);
    });
}

/// What the untyped value conversions must know about the types of this crate: a router that a
/// view model exposes binds to the `Router` of a [`RoutedViewHost`].
fn register_value_types() {
    ValueTypes::register_reference::<RoutingState>();
}
