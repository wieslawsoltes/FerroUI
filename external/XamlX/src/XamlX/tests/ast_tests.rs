//! Not from upstream, which has no tests of the text of AST nodes: the `ToString()` overrides
//! of the type and property references, ported as `IXamlAstNode::to_node_string`.

use crate::ast::*;
use crate::testing::FakeTypeSystem;

#[test]
fn clr_type_reference_to_string_is_fqn_of_type() {
    let ts = FakeTypeSystem::new();
    let li = XamlLineInfo::new(1, 1);
    let reference = XamlAstClrTypeReference::new(&li, ts.get("System.String"), false);

    assert_eq!("System.Runtime:System.String", reference.to_node_string());
}

#[test]
fn clr_property_to_string_is_fqn_of_declaring_type_and_name() {
    let ts = FakeTypeSystem::new();
    let li = XamlLineInfo::new(1, 1);
    let property = XamlAstClrProperty::with_setters(&li, "Length", ts.get("System.String"), None, None, None);

    assert_eq!("System.Runtime:System.String.Length", property.to_node_string());
}

#[test]
fn xml_type_reference_to_string_is_namespace_and_name() {
    let li = XamlLineInfo::new(1, 1);

    assert_eq!("xml!!ns:T", XamlAstXmlTypeReference::new(&li, Some("ns"), "T").to_node_string());
    // A null namespace is concatenated as nothing.
    assert_eq!("xml!!:T", XamlAstXmlTypeReference::new(&li, None, "T").to_node_string());
}
