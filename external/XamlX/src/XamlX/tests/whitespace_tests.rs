//! Port of the upstream `WhitespaceTests.cs`.
//!
//! Upstream compiles the markup and inspects the created objects; here the same expectations are
//! asserted on the transformed AST: the values assigned to the content property.
//!
//! Most of these test cases are derived from these sources:
//! https://docs.microsoft.com/en-us/dotnet/desktop/xaml-services/white-space-processing
//! https://www.w3.org/TR/2006/REC-xml11-20060816/

use std::rc::Rc;

use crate::ast::*;

use super::test_xaml_language::TestHost;

const NS: &str = "xmlns='test'";

// As per the XAML docs, these three characters are considered whitespace,
// we also add \r to simulate character entities bypassing the XML parser
const ALL_WHITESPACE: &str = " \n\t";

// CharacterEntity representation of AllWhitespace
const CHARACTER_ENTITIES: &str = "&#x20;&#xA;&#x9;";

/// A value assigned to a property: either a string or an object of the named type.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Text(String),
    Object(String),
}

fn text(s: &str) -> Item {
    Item::Text(s.to_string())
}

fn object(s: &str) -> Item {
    Item::Object(s.to_string())
}

fn collect_assignments(node: &Rc<dyn IXamlAstNode>, rv: &mut Vec<Rc<XamlPropertyAssignmentNode>>) {
    if let Some(assignment) = node.cast::<XamlPropertyAssignmentNode>() {
        rv.push(assignment);
    } else if let Some(group) = node.cast::<XamlManipulationGroupNode>() {
        for child in group.children.borrow().iter() {
            collect_assignments(&child.as_node(), rv);
        }
    }
}

/// The property assignments of a transformed object node.
pub fn assignments(object: &Rc<dyn IXamlAstNode>) -> Vec<Rc<XamlPropertyAssignmentNode>> {
    let object = object
        .cast::<XamlAstConstructableObjectNode>()
        .expect("a constructable object node");
    let mut rv = Vec::new();
    for child in object.children.borrow().iter() {
        collect_assignments(child, &mut rv);
    }
    rv
}

/// The value nodes assigned to (or added to) the given property of a transformed object node.
pub fn property_values(
    object: &Rc<dyn IXamlAstNode>,
    property: &str,
) -> Vec<Rc<dyn IXamlAstValueNode>> {
    assignments(object)
        .iter()
        .filter(|a| a.property.name() == property)
        .map(|a| {
            a.values
                .borrow()
                .last()
                .cloned()
                .expect("an assignment has a value")
        })
        .collect()
}

fn describe(value: &Rc<dyn IXamlAstValueNode>) -> Item {
    if let Some(text_node) = value.cast::<XamlAstTextNode>() {
        return Item::Text(text_node.text());
    }
    if let Some(object_node) = value.cast::<XamlAstConstructableObjectNode>() {
        return Item::Object(object_node.type_().get_clr_type().expect("clr type").name());
    }
    Item::Object(value.type_name().to_string())
}

fn property_items(object: &Rc<dyn IXamlAstNode>, property: &str) -> Vec<Item> {
    property_values(object, property)
        .iter()
        .map(describe)
        .collect()
}

fn read_xaml(xaml: &str) -> Rc<dyn IXamlAstNode> {
    TestHost::new().transform_root(xaml)
}

/// The value assigned to `ContentControl.Content`, `None` when nothing is assigned.
fn test_content_control_content(
    raw_content: &str,
    xml_preserve: bool,
) -> Option<Rc<dyn IXamlAstValueNode>> {
    let xml_space_attr = if xml_preserve {
        " xml:space='preserve'"
    } else {
        ""
    };
    let control = read_xaml(&format!(
        "<ContentControl {NS}{xml_space_attr}>{raw_content}</ContentControl>"
    ));
    let mut values = property_values(&control, "Content");
    assert!(values.len() <= 1);
    values.pop()
}

fn content_text(raw_content: &str, xml_preserve: bool) -> Option<String> {
    test_content_control_content(raw_content, xml_preserve).map(|value| match describe(&value) {
        Item::Text(text) => text,
        other => panic!("Expected text content, got {other:?}"),
    })
}

fn content_object(raw_content: &str, xml_preserve: bool) -> Rc<dyn IXamlAstNode> {
    test_content_control_content(raw_content, xml_preserve)
        .expect("content")
        .as_node()
}

fn test_mixed_content(
    raw_content: &str,
    preserve_space: bool,
    white_space_opt_in: bool,
) -> Vec<Item> {
    let control_name = if white_space_opt_in {
        "WhitespaceOptInControl"
    } else {
        "MixedContentControl"
    };
    let xml_space_attr = if preserve_space {
        " xml:space='preserve'"
    } else {
        ""
    };
    let xaml = format!("<{control_name} {NS}{xml_space_attr}>{raw_content}</{control_name}>");
    property_items(&read_xaml(&xaml), "Content")
}

#[test]
fn empty_tag_results_in_null_content() {
    assert_eq!(content_text("", false), None);
}

#[test]
fn self_closing_tag_results_in_null_content() {
    let control = read_xaml(&format!("<ContentControl {NS}/>"));
    assert!(property_values(&control, "Content").is_empty());
}

#[test]
fn white_space_only_content_results_in_null_content_without_opt_in() {
    assert_eq!(content_text(ALL_WHITESPACE, false), None);
}

#[test]
fn leading_and_trailing_white_space_is_trimmed_without_opt_in() {
    assert_eq!(
        content_text(&format!("{ALL_WHITESPACE}ABC{ALL_WHITESPACE}"), false).as_deref(),
        Some("ABC")
    );
}

#[test]
fn inner_whitespace_is_preserved_but_collapsed() {
    let content = content_text(
        &format!("{ALL_WHITESPACE}A{ALL_WHITESPACE}C{ALL_WHITESPACE}"),
        false,
    );
    assert_eq!(content.as_deref(), Some("A C"));
}

#[test]
fn whitespace_other_than_space_is_converted_to_space() {
    assert_eq!(
        content_text("A B\rC\nD\tE\r\nF", false).as_deref(),
        Some("A B C D E F")
    );
}

#[test]
fn character_entities_are_recognized_as_whitespace() {
    // We add a carriage return here since it can be used to bypass the parsers normalization
    // and does behave differently between \r and &#xD; (\r always gets converted to \n, while &#xD; does not).
    let entities = format!("{CHARACTER_ENTITIES}&#xD;");
    assert_eq!(
        content_text(&format!("{entities}A{entities}B{entities}"), false).as_deref(),
        Some("A B")
    );
}

#[test]
fn comment_nodes_are_ignored_for_whitespace_normalization() {
    assert_eq!(content_text(" <!-- X --> ", false), None);
}

// This is per XML specification https://www.w3.org/TR/2006/REC-xml11-20060816/#sec-line-ends
#[test]
fn carriage_return_is_translated_to_new_line_even_with_xml_space_preserve() {
    assert_eq!(content_text("A\rB\r\nC", true).as_deref(), Some("A\nB\nC"));
}

// As an exception to the aforementioned spec, character entities bypass the XML parser's normalization
// and will result in \r being passed through when xml:space=preserve is being used.
#[test]
fn carriage_return_character_entity_is_maintained_with_xml_space_preserve() {
    assert_eq!(
        content_text("A&#x0D;B&#x0D;\nC", true).as_deref(),
        Some("A\rB\r\nC")
    );
}

// This behavior differs from WPF, where a string property maintains the white-space, while an
// object property does not.
#[test]
fn white_space_only_text_nodes_are_stripped_for_controls_not_opting_in_even_with_xml_space_preserve(
) {
    assert_eq!(
        content_text(ALL_WHITESPACE, true).as_deref(),
        Some(ALL_WHITESPACE)
    );
}

#[test]
fn string_properties_will_receive_whitespace_only_with_xml_space_preserve() {
    let content = content_object(
        &format!("<Control><Control.StrProp>{ALL_WHITESPACE}</Control.StrProp></Control>"),
        true,
    );
    assert_eq!(
        property_items(&content, "StrProp"),
        vec![text(ALL_WHITESPACE)]
    );
}

#[test]
fn leading_and_trailing_white_space_is_not_trimmed_with_xml_space_preserve() {
    let raw = format!("{ALL_WHITESPACE}X{ALL_WHITESPACE}");
    assert_eq!(content_text(&raw, true), Some(raw));
}

#[test]
fn inner_whitespace_is_preserved_and_not_collapsed_with_xml_space_preserve() {
    let raw = format!("{ALL_WHITESPACE}A{ALL_WHITESPACE}C{ALL_WHITESPACE}");
    assert_eq!(content_text(&raw, true), Some(raw));
}

#[test]
fn xml_space_preserve_is_inherited_from_parent() {
    let content = content_object(
        &format!("<ContentControl>{ALL_WHITESPACE}A{ALL_WHITESPACE}</ContentControl>"),
        true,
    );
    assert_eq!(
        property_items(&content, "Content"),
        vec![text(&format!("{ALL_WHITESPACE}A{ALL_WHITESPACE}"))]
    );
}

#[test]
fn white_space_before_between_and_after_property_setters_is_trimmed() {
    let content = content_text(
        &format!("{ALL_WHITESPACE}<Control.StrProp>Red</Control.StrProp>{ALL_WHITESPACE}<Control.BoolProp>false</Control.BoolProp> CONTENT"),
        false,
    );
    assert_eq!(content.as_deref(), Some("CONTENT"));
}

#[test]
fn white_space_before_and_between_and_property_setters_is_trimmed_with_xml_space_preserve() {
    let content = content_text(
        &format!("{ALL_WHITESPACE}<Control.StrProp>Red</Control.StrProp>{ALL_WHITESPACE}<Control.BoolProp>false</Control.BoolProp> CONTENT"),
        true,
    );
    assert_eq!(content.as_deref(), Some(" CONTENT"));
}

#[test]
fn xml_space_preserve_does_not_affect_attribute_value_normalization() {
    for xml_preserve in [false, true] {
        let xaml = format!("<ContentControl Content=\"{ALL_WHITESPACE}X{ALL_WHITESPACE}\" />");
        let content = content_object(&xaml, xml_preserve);
        // This normalization is due to XML spec 3.3.3 Attribute-Value Normalization
        assert_eq!(property_items(&content, "Content"), vec![text("   X   ")]);
    }
}

// See XML spec 3.3.3 Attribute-Value Normalization
#[test]
fn character_entities_in_attributes_are_not_subject_to_attribute_value_normalization() {
    let xaml = format!("<ContentControl Content=\"{CHARACTER_ENTITIES}\" />");
    let content = content_object(&xaml, false);
    assert_eq!(
        property_items(&content, "Content"),
        vec![text(ALL_WHITESPACE)]
    );
}

#[test]
fn xml_space_default_applies_property_setter_element_normalization() {
    // Whitespace normalization is applied normally
    let content = content_text(
        &format!("<ContentControl.Content>{ALL_WHITESPACE}</ContentControl.Content>"),
        false,
    );
    assert_eq!(content, None);
}

// xml:space=preserve can be used to disable whitespace normalization for property setters too,
// even though the schema does not allow the attribute to be set on the property-setter itself,
// it's value is inherited from the parent.
#[test]
fn xml_space_preserve_does_not_apply_property_setter_element_normalization() {
    // Whitespace normalization isn't applied, because the parent has xml:space="preserve"
    let content = content_text(
        &format!("<ContentControl.Content>{ALL_WHITESPACE}</ContentControl.Content>"),
        true,
    );
    assert_eq!(content.as_deref(), Some(ALL_WHITESPACE));
}

#[test]
fn white_space_around_nested_control_is_trimmed_without_opt_in() {
    let content = test_mixed_content(
        &format!("{ALL_WHITESPACE}<Control/>{ALL_WHITESPACE}<Control/>{ALL_WHITESPACE}"),
        false,
        false,
    );
    assert_eq!(content, vec![object("Control"), object("Control")]);
}

#[test]
fn white_space_around_nested_control_is_trimmed_without_opt_in_even_with_xml_space_preserve() {
    let content = test_mixed_content(
        &format!("{ALL_WHITESPACE}<Control/>{ALL_WHITESPACE}<Control/>{ALL_WHITESPACE}"),
        true,
        false,
    );
    assert_eq!(content, vec![object("Control"), object("Control")]);
}

#[test]
fn text_across_comments_is_merged() {
    let content = test_mixed_content(
        &format!("{ALL_WHITESPACE}<!-- X -->{ALL_WHITESPACE}"),
        true,
        true,
    );
    assert_eq!(
        content,
        vec![text(&format!("{ALL_WHITESPACE}{ALL_WHITESPACE}"))]
    );
}

// Due to the following normalization rules from the XAML documentation:
// - A space immediately following the start tag is deleted.
// - A space immediately before the end tag is deleted.
#[test]
fn white_space_at_start_and_end_is_removed_even_with_whitespace_opt_in() {
    let content = test_mixed_content(
        &format!("{ALL_WHITESPACE}<Control/>{ALL_WHITESPACE}"),
        false,
        true,
    );
    assert_eq!(content, vec![object("Control")]);
}

#[test]
fn white_space_at_start_and_end_is_preserved_with_both_opt_in_and_xml_space_preserve() {
    let content = test_mixed_content(
        &format!("{ALL_WHITESPACE}<Control />{ALL_WHITESPACE}"),
        true,
        true,
    );
    assert_eq!(
        content,
        vec![
            text(ALL_WHITESPACE),
            object("Control"),
            text(ALL_WHITESPACE)
        ]
    );
}

// This is important for TextBlock for space between Spans/Runs and normal text
#[test]
fn white_space_in_nodes_around_normal_control_is_preserved_when_opting_in() {
    let content = test_mixed_content("A <Control/> B", false, true);
    assert_eq!(content, vec![text("A "), object("Control"), text(" B")]);
}

// This is important for TextBlock for space between Spans/Runs.
#[test]
fn whitespace_between_non_text_nodes_is_preserved_when_opting_in() {
    let content = test_mixed_content("<Control/> <Control/>", false, true);
    assert_eq!(
        content,
        vec![object("Control"), text(" "), object("Control")]
    );
}

#[test]
fn whitespace_in_text_nodes_around_trim_around_control_is_trimmed_when_opting_in() {
    let content = test_mixed_content("A <TrimControl/> B", false, true);
    assert_eq!(content, vec![text("A"), object("TrimControl"), text("B")]);
}

#[test]
fn trim_surrounding_whitespace_is_disabled_by_xml_space_preserve() {
    let content = test_mixed_content("A <TrimControl/> B", true, true);
    assert_eq!(content, vec![text("A "), object("TrimControl"), text(" B")]);
}

#[test]
fn whitespace_surrounding_trim_control_is_trimmed_when_opting_in() {
    // The comments in the string ensure that this happens AFTER merging text nodes OR the algorithm
    // is capable of trimming multiple whitespace nodes.
    let content = test_mixed_content(
        "<Control/> <!-- --> <TrimControl/> <!-- --> <Control/>",
        false,
        true,
    );
    assert_eq!(
        content,
        vec![object("Control"), object("TrimControl"), object("Control")]
    );
}

#[test]
fn whitespace_should_be_trimmed_for_plain_ienumerable_content_property() {
    let content = read_xaml(&format!(
        "<MixedEnumerableContentControl xmlns='test'>{ALL_WHITESPACE}<Control/>{ALL_WHITESPACE}<Control/>{ALL_WHITESPACE}</MixedEnumerableContentControl>"
    ));
    assert_eq!(
        property_items(&content, "Items"),
        vec![object("Control"), object("Control")]
    );
}

#[test]
fn whitespace_should_not_be_trimmed_for_whitespace_opt_in_collection() {
    let xaml = "
<ControlWithInlines  xmlns='test'>
   with <InlineWithInlines>several</InlineWithInlines>
    <InlineWithInlines>Span</InlineWithInlines>
  </ControlWithInlines>
";
    let content = read_xaml(xaml);
    let inlines = property_values(&content, "Inlines");
    let items: Vec<Item> = inlines.iter().map(describe).collect();
    assert_eq!(
        items,
        vec![
            text("with "),
            object("InlineWithInlines"),
            text(" "),
            object("InlineWithInlines")
        ]
    );
    assert_eq!(
        property_items(&inlines[1].as_node(), "Inlines"),
        vec![text("several")]
    );
    assert_eq!(
        property_items(&inlines[3].as_node(), "Inlines"),
        vec![text("Span")]
    );
}
