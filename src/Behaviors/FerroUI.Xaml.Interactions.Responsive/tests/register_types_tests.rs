//! Not from the reference: checks the type table of the crate (the `XmlnsDefinition` of
//! `Properties/AssemblyInfo.cs`) and the markup metadata of the lists of setters.

use super::support::start_app;
use crate::register_types::TYPES;
use crate::{register_types, AdaptiveClassSetters, AspectRatioClassSetters, ASSEMBLY};
use ferroui_base::metadata::{MarkupAssembly, MarkupTyped, FERRO_XML_NAMESPACE};
use ferroui_base::TypeInfo;

#[test]
fn every_type_of_this_crate_has_the_namespace_of_the_crate() {
    register_types();

    for type_ in TYPES {
        assert_eq!("FerroUI.Xaml.Interactions.Responsive", type_.namespace(), "{type_} has another namespace");
    }
    assert!(TypeInfo::find("FerroUI.Xaml.Interactions.Responsive", "AdaptiveBehavior").is_some());
    assert!(TypeInfo::find("FerroUI.Xaml.Interactions.Responsive", "AspectRatioClassSetter").is_some());
}

#[test]
fn the_behaviors_and_setters_can_be_created_by_markup() {
    let _app = start_app();

    for name in ["AdaptiveBehavior", "AdaptiveClassSetter", "AspectRatioBehavior", "AspectRatioClassSetter"] {
        let type_ = TypeInfo::find("FerroUI.Xaml.Interactions.Responsive", name).expect("the type is registered");
        assert!(type_.create_instance().is_some(), "{name} has a default constructor");
    }
}

#[test]
fn the_lists_of_setters_have_an_add_method_for_markup() {
    register_types();

    assert_eq!(1, <AdaptiveClassSetters as MarkupTyped>::MARKUP.find_methods("Add").count());
    assert_eq!(1, <AspectRatioClassSetters as MarkupTyped>::MARKUP.find_methods("Add").count());
}

#[test]
fn the_assembly_maps_the_namespace_into_the_xml_namespace_of_the_framework() {
    register_types();

    let assembly = MarkupAssembly::find("FerroUI.Xaml.Interactions.Responsive").expect("the assembly is registered");
    assert!(std::ptr::eq(assembly, &ASSEMBLY));
    assert!(assembly
        .xmlns_definitions
        .iter()
        .all(|definition| definition.xml_namespace == FERRO_XML_NAMESPACE
            && definition.namespace == "FerroUI.Xaml.Interactions.Responsive"));
}
