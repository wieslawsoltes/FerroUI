//! Port of the upstream `ParserTests.cs`, plus tests of the parser behavior the upstream suite
//! only covers through compiled output (line info, errors, entities, xml:space).

use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::exceptions::{XamlError, XamlResult};
use crate::parsers::system_xaml_markup_extension_parser::{
    MeScannerError, SystemXamlMarkupExtensionParser,
};
use crate::parsers::{CommaSeparatedParenthesesTreeParser, XDocumentXamlParser};

use super::helpers::{dump, struct_diff};

const X: &str = "http://schemas.microsoft.com/winfx/2006/xaml";

fn ni() -> XamlLineInfo {
    XamlLineInfo::new(1, 1)
}

fn xml_type(ns: &str, name: &str) -> Rc<XamlAstXmlTypeReference> {
    XamlAstXmlTypeReference::new(&ni(), Some(ns), name)
}

fn generic_xml_type(
    ns: &str,
    name: &str,
    args: Vec<Rc<XamlAstXmlTypeReference>>,
) -> Rc<XamlAstXmlTypeReference> {
    XamlAstXmlTypeReference::with_generic_arguments(&ni(), Some(ns), name, args)
}

fn text(text: &str, preserve: bool) -> Rc<dyn IXamlAstValueNode> {
    XamlAstTextNode::new(&ni(), text, preserve)
}

fn object(
    type_: &Rc<XamlAstXmlTypeReference>,
    arguments: Vec<Rc<dyn IXamlAstValueNode>>,
    children: Vec<Rc<dyn IXamlAstNode>>,
) -> Rc<XamlAstObjectNode> {
    let rv = XamlAstObjectNode::new(&ni(), type_.clone());
    *rv.arguments.borrow_mut() = arguments;
    *rv.children.borrow_mut() = children;
    rv
}

fn prop(
    declaring: &Rc<XamlAstXmlTypeReference>,
    name: &str,
    target: &Rc<XamlAstXmlTypeReference>,
    values: Vec<Rc<dyn IXamlAstValueNode>>,
    attribute_syntax: bool,
) -> Rc<dyn IXamlAstNode> {
    XamlAstXamlPropertyValueNode::with_values(
        &ni(),
        XamlAstNamePropertyReference::new(&ni(), declaring.clone(), name, target.clone()),
        values,
        attribute_syntax,
    )
}

fn directive(ns: &str, name: &str, values: Vec<Rc<dyn IXamlAstValueNode>>) -> Rc<dyn IXamlAstNode> {
    XamlAstXmlDirective::new(&ni(), Some(ns), name, values)
}

fn parse(xaml: &str) -> XamlDocument {
    XDocumentXamlParser::parse(xaml, None).expect("the markup parses")
}

fn root_object(doc: &XamlDocument) -> Rc<XamlAstObjectNode> {
    doc.root()
        .expect("root")
        .cast::<XamlAstObjectNode>()
        .expect("the root is an object node")
}

fn parse_error(xaml: &str) -> XamlError {
    match XDocumentXamlParser::parse(xaml, None) {
        Ok(_) => panic!("Expected a parse error"),
        Err(e) => e,
    }
}

#[test]
fn parser_should_be_able_to_parse_a_simple_tree() {
    let root = parse(
        r#"
<Root xmlns='rootns' xmlns:t='testns' xmlns:d='directive' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>
    <Child Ext='{Extension 123, 321, Prop=test, Prop2=test2, Prop3={Extension}, Prop4=test3}'
        Other.Prop='{}Not extension'
        Prop1='123'
        Root.AttachedProp='AttachedValue'
        t:Namespaced.AttachedProp='AttachedValue'
        d:Directive='DirectiveValue'
        d:DirectiveExt='{Extension 123}'>
        <t:SubChild Prop='321' Root.AttachedProp='AttachedValue'/>
        <Child.DottedProp>DottedValue</Child.DottedProp>
        <Root.AttachedDottedProp>AttachedValue</Root.AttachedDottedProp>
        <Child.NodeListProp>
            <SubChild/>
            <SubChild/>
        </Child.NodeListProp>
    </Child>
    <GenericType x:TypeArguments='Child,t:NamespacedGeneric(Child,GenericType ( Child, t:Namespaced) )'/>

</Root>"#,
    );
    let root_type = xml_type("rootns", "Root");
    let child_type = xml_type("rootns", "Child");
    let sub_child_type = xml_type("rootns", "SubChild");
    let ns_sub_child_type = xml_type("testns", "SubChild");
    let namespaced_type = xml_type("testns", "Namespaced");
    let extension_type = xml_type("rootns", "Extension");
    extension_type.is_markup_extension.set(true);

    let expected_aliases: HashMap<String, String> = [
        ("", "rootns"),
        ("t", "testns"),
        ("d", "directive"),
        ("x", X),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    assert_eq!(root.namespace_aliases, expected_aliases);

    let expected: Rc<dyn IXamlAstNode> = object(
        &root_type,
        vec![],
        vec![
            // Whitespace preceeding first child
            text("\n    ", false),
            // <Child
            object(
                &child_type,
                vec![],
                vec![
                    // Ext='{Extension 123, 321, Prop=test, Prop2=test2}'
                    prop(
                        &child_type,
                        "Ext",
                        &child_type,
                        vec![object(
                            &extension_type,
                            vec![text("123", true), text("321", true)],
                            vec![
                                prop(
                                    &extension_type,
                                    "Prop",
                                    &extension_type,
                                    vec![text("test", true)],
                                    true,
                                ),
                                prop(
                                    &extension_type,
                                    "Prop2",
                                    &extension_type,
                                    vec![text("test2", true)],
                                    true,
                                ),
                                prop(
                                    &extension_type,
                                    "Prop3",
                                    &extension_type,
                                    vec![object(&extension_type, vec![], vec![])],
                                    true,
                                ),
                                prop(
                                    &extension_type,
                                    "Prop4",
                                    &extension_type,
                                    vec![text("test3", true)],
                                    true,
                                ),
                            ],
                        )],
                        true,
                    ),
                    // Other.Prop='{}Not extension'
                    prop(
                        &xml_type("rootns", "Other"),
                        "Prop",
                        &child_type,
                        vec![text("Not extension", true)],
                        true,
                    ),
                    // Prop1='123'
                    prop(
                        &child_type,
                        "Prop1",
                        &child_type,
                        vec![text("123", true)],
                        true,
                    ),
                    // Root.AttachedProp='AttachedValue'
                    prop(
                        &root_type,
                        "AttachedProp",
                        &child_type,
                        vec![text("AttachedValue", true)],
                        true,
                    ),
                    // t:Namespaced.AttachedProp='AttachedValue'
                    prop(
                        &namespaced_type,
                        "AttachedProp",
                        &child_type,
                        vec![text("AttachedValue", true)],
                        true,
                    ),
                    // d:Directive='DirectiveValue'>
                    directive("directive", "Directive", vec![text("DirectiveValue", true)]),
                    // d:DirectiveExt='{Extension 123}'>
                    directive(
                        "directive",
                        "DirectiveExt",
                        vec![object(&extension_type, vec![text("123", true)], vec![])],
                    ),
                    text("\n        ", false),
                    // <t:SubChild Prop='321' Root.AttachedProp='AttachedValue'/>
                    object(
                        &ns_sub_child_type,
                        vec![],
                        vec![
                            prop(
                                &ns_sub_child_type,
                                "Prop",
                                &ns_sub_child_type,
                                vec![text("321", true)],
                                true,
                            ),
                            // Root.AttachedProp='AttachedValue'
                            prop(
                                &root_type,
                                "AttachedProp",
                                &ns_sub_child_type,
                                vec![text("AttachedValue", true)],
                                true,
                            ),
                        ],
                    ),
                    text("\n        ", false),
                    // <Child.DottedProp>DottedValue</Child.DottedProp>
                    prop(
                        &child_type,
                        "DottedProp",
                        &child_type,
                        vec![text("DottedValue", false)],
                        false,
                    ),
                    text("\n        ", false),
                    // <Root.AttachedDottedProp>AttachedValue</Root.AttachedDottedProp>
                    prop(
                        &root_type,
                        "AttachedDottedProp",
                        &child_type,
                        vec![text("AttachedValue", false)],
                        false,
                    ),
                    text("\n        ", false),
                    // <Child.NodeListProp>
                    prop(
                        &child_type,
                        "NodeListProp",
                        &child_type,
                        vec![
                            text("\n            ", false),
                            // <SubChild/>
                            object(&sub_child_type, vec![], vec![]),
                            text("\n            ", false),
                            // <SubChild/>
                            object(&sub_child_type, vec![], vec![]),
                            text("\n        ", false),
                        ],
                        false,
                    ),
                    text("\n    ", false),
                ],
            ),
            // Whitespace between child and generic type
            text("\n    ", false),
            // <GenericType x:TypeArguments='Child,t:NamespacedGeneric(Child,GenericType(Child, t:Namespaced))'/>
            object(
                &generic_xml_type(
                    "rootns",
                    "GenericType",
                    vec![
                        child_type.clone(),
                        generic_xml_type(
                            "testns",
                            "NamespacedGeneric",
                            vec![
                                child_type.clone(),
                                generic_xml_type(
                                    "rootns",
                                    "GenericType",
                                    vec![child_type.clone(), namespaced_type.clone()],
                                ),
                            ],
                        ),
                    ],
                ),
                vec![],
                vec![],
            ),
            // Whitespace after generic type
            text("\n\n", false),
        ],
    );
    struct_diff(&root.root().expect("root"), &expected);
}

fn parser_should_handle_ignorable_content(map: bool) {
    let mappings: HashMap<String, String> = [("test".to_string(), "mapped".to_string())]
        .into_iter()
        .collect();
    let root = XDocumentXamlParser::parse(
        r#"
<Root xmlns='rootns' xmlns:mc='http://schemas.openxmlformats.org/markup-compatibility/2006'
    mc:Ignorable='d d2' xmlns:d='test' xmlns:d2='test2'
    d:DataContext='123' d2:Lalala='321'>
    <d:DesignWidth>test</d:DesignWidth>
</Root>
 "#,
        if map { Some(&mappings) } else { None },
    )
    .expect("the markup parses");
    let root_type = xml_type("rootns", "Root");

    let expected: Rc<dyn IXamlAstNode> = if map {
        object(
            &root_type,
            vec![],
            vec![
                directive("mapped", "DataContext", vec![text("123", true)]),
                text("\n    ", false),
                object(
                    &xml_type("mapped", "DesignWidth"),
                    vec![],
                    vec![text("test", false)],
                ),
                text("\n", false),
            ],
        )
    } else {
        // The whitespace following the skipped element is consumed together with it.
        object(&root_type, vec![], vec![text("\n    ", false)])
    };
    struct_diff(&root.root().expect("root"), &expected);

    assert_eq!(
        root.namespace_aliases.get("d").map(String::as_str),
        Some(if map { "mapped" } else { "test" })
    );
    assert_eq!(
        root.namespace_aliases.get("d2").map(String::as_str),
        Some("test2")
    );
}

#[test]
fn parser_should_handle_ignorable_content_without_mapping() {
    parser_should_handle_ignorable_content(false);
}

#[test]
fn parser_should_handle_ignorable_content_with_mapping() {
    parser_should_handle_ignorable_content(true);
}

fn parse_extension(ext: &str) -> Result<Rc<dyn IXamlAstValueNode>, MeScannerError> {
    let resolver = |n: &str| -> XamlResult<Rc<XamlAstXmlTypeReference>> {
        Ok(XamlAstXmlTypeReference::new(&ni(), Some(""), n))
    };
    SystemXamlMarkupExtensionParser::parse(&ni(), ext, &resolver)
}

fn extension_dump(ext: &str) -> String {
    dump(
        &parse_extension(ext)
            .expect("the extension parses")
            .as_node(),
    )
}

#[test]
fn empty_extension_with_space_should_be_parsed() {
    let parsed = parse_extension("{Binding }").expect("the extension parses");
    let binding = xml_type("", "Binding");
    binding.is_markup_extension.set(true);
    let expected: Rc<dyn IXamlAstNode> = object(&binding, vec![], vec![]);
    struct_diff(&parsed.as_node(), &expected);
}

#[test]
fn markup_extension_positional_and_named_arguments() {
    assert_eq!(
        extension_dump("{Ext 1, two , Named=3}"),
        "(Object xml!!:Ext{ME} args=[(Text \"1\" preserve=true type=xml!!http://schemas.microsoft.com/winfx/2006/xaml:String), \
         (Text \"two\" preserve=true type=xml!!http://schemas.microsoft.com/winfx/2006/xaml:String)] \
         children=[(Prop (NameProp xml!!:Ext{ME}.Named target=xml!!:Ext{ME}) attr=true \
         [(Text \"3\" preserve=true type=xml!!http://schemas.microsoft.com/winfx/2006/xaml:String)])])"
    );
}

fn extension_texts(ext: &str) -> Vec<String> {
    let parsed = parse_extension(ext).expect("the extension parses");
    let object = parsed.cast::<XamlAstObjectNode>().expect("object node");
    let arguments = object.arguments.borrow();
    arguments
        .iter()
        .map(|a| a.cast::<XamlAstTextNode>().expect("text argument").text())
        .collect()
}

#[test]
fn markup_extension_quotes_and_escapes() {
    assert_eq!(
        extension_texts("{Ext 'a, b', \"c=d\"}"),
        vec!["a, b", "c=d"]
    );
    assert_eq!(extension_texts(r"{Ext a\,b, \{c\}}"), vec!["a,b", "{c}"]);
    assert_eq!(extension_texts("{Ext {}{literal}}"), vec!["{literal}"]);
    assert_eq!(extension_texts("{Ext   padded   }"), vec!["padded"]);
    assert_eq!(extension_texts("{Ext '  quoted  '}"), vec!["  quoted  "]);
    assert_eq!(extension_texts("{Ext a{b}c}"), vec!["a{b}c"]);
}

#[test]
fn markup_extension_nested_and_quoted_extensions() {
    let nested = parse_extension("{Outer {Inner 1}, P='{Inner 2}'}").expect("the extension parses");
    assert_eq!(
        dump(&nested.as_node()).replace("http://schemas.microsoft.com/winfx/2006/xaml", "x"),
        "(Object xml!!:Outer{ME} args=[(Object xml!!:Inner{ME} args=[(Text \"1\" preserve=true type=xml!!x:String)] children=[])] \
         children=[(Prop (NameProp xml!!:Outer{ME}.P target=xml!!:Outer{ME}) attr=true \
         [(Object xml!!:Inner{ME} args=[(Text \"2\" preserve=true type=xml!!x:String)] children=[])])])"
    );
}

#[test]
fn markup_extension_attached_property_name() {
    let parsed = parse_extension("{Ext Other.Prop=1}").expect("the extension parses");
    let object = parsed.cast::<XamlAstObjectNode>().expect("object node");
    let children = object.children.borrow();
    let prop = children[0]
        .cast::<XamlAstXamlPropertyValueNode>()
        .expect("property node");
    let reference = prop
        .property()
        .cast::<XamlAstNamePropertyReference>()
        .expect("name reference");
    assert_eq!(reference.name(), "Prop");
    assert_eq!(
        dump(&reference.declaring_type.borrow().as_node()),
        "xml!!:Other"
    );
    assert_eq!(
        dump(&reference.target_type.borrow().as_node()),
        "xml!!:Ext{ME}"
    );
}

fn extension_error(ext: &str) -> String {
    match parse_extension(ext) {
        Ok(_) => panic!("Expected {ext} to fail"),
        Err(MeScannerError::Parse(e)) => e.message,
        Err(MeScannerError::Xaml(e)) => panic!("Unexpected error {e}"),
    }
}

#[test]
fn markup_extension_errors() {
    assert_eq!(extension_error("{Ext 'abc}"), "Unclosed quote");
    assert_eq!(
        extension_error("{Ext a'b'}"),
        "Quote characters out of place"
    );
    assert_eq!(extension_error("{Ext"), "Unexpected token None");
    assert_eq!(
        extension_error("{Ext P=1, 2}"),
        "Unexpected token after property list String"
    );
    assert_eq!(extension_error("{Ext {a}"), "Unexpected token None");
    assert_eq!(extension_error("}"), "Unexpected token Close");
    // "{}" is the escape sequence for a literal string, here an empty one.
    let empty = parse_extension("{}").expect("the escape parses");
    assert_eq!(
        empty.cast::<XamlAstTextNode>().expect("text node").text(),
        ""
    );
}

#[test]
fn markup_extension_errors_become_parse_exceptions_with_line_info() {
    let error = parse_error("<Root xmlns='rootns'>\n  <Child Prop='{Ext &apos;abc}'/>\n</Root>");
    match &error {
        XamlError::Parse(e) => {
            assert_eq!(e.title, "Unclosed quote");
            assert_eq!((e.line_number, e.line_position), (2, 10));
        }
        other => panic!("Unexpected error {other}"),
    }
    assert_eq!(error.message(), "Unclosed quote Line 2, position 10.");
}

#[test]
fn markup_extension_type_arguments_and_directives() {
    let doc = parse(&format!(
        "<Root xmlns='rootns' xmlns:x='{X}' xmlns:t='testns' Prop='{{Ext x:TypeArguments=t:A, x:Key=key, Inner={{Ext2 x:Name=n}}}}'/>"
    ));
    let root = root_object(&doc);
    let children = root.children.borrow();
    let prop = children[0]
        .cast::<XamlAstXamlPropertyValueNode>()
        .expect("property node");
    let value = prop.values.borrow()[0].as_node();
    assert_eq!(
        dump(&value).replace(X, "x"),
        "(Object xml!!rootns:Ext[xml!!testns:A]{ME} args=[] children=[\
         (Prop (NameProp xml!!rootns:Ext[xml!!testns:A]{ME}.Inner target=xml!!rootns:Ext[xml!!testns:A]{ME}) attr=true \
         [(Object xml!!rootns:Ext2{ME} args=[] children=[(Directive x:Name [(Text \"n\" preserve=true type=xml!!x:String)])])]), \
         (Directive x:Key [(Text \"key\" preserve=true type=xml!!x:String)])])"
    );
}

#[test]
fn line_info_points_at_names_and_text() {
    let doc = parse("<Root xmlns='rootns'\n      Prop='1'>\n  <Child Other='2'/>text\n</Root>");
    let root = root_object(&doc);
    assert_eq!((root.line(), root.position()), (1, 2));
    let children = root.children.borrow();

    // Attribute property nodes are positioned at the element, their values at the attribute name.
    let prop = children[0]
        .cast::<XamlAstXamlPropertyValueNode>()
        .expect("property node");
    assert_eq!((prop.line(), prop.position()), (1, 2));
    let value = prop.values.borrow()[0].clone();
    assert_eq!((value.line(), value.position()), (2, 7));

    let whitespace = children[1].cast::<XamlAstTextNode>().expect("text node");
    assert_eq!((whitespace.line(), whitespace.position()), (2, 16));

    let child = children[2]
        .cast::<XamlAstObjectNode>()
        .expect("object node");
    assert_eq!((child.line(), child.position()), (3, 4));
    let child_children = child.children.borrow();
    let child_prop = child_children[0]
        .cast::<XamlAstXamlPropertyValueNode>()
        .expect("property node");
    let child_value = child_prop.values.borrow()[0].clone();
    assert_eq!((child_value.line(), child_value.position()), (3, 10));

    let text = children[3].cast::<XamlAstTextNode>().expect("text node");
    assert_eq!(text.text(), "text\n");
    assert_eq!((text.line(), text.position()), (3, 21));
}

#[test]
fn line_info_counts_utf16_units_and_all_line_endings() {
    let doc = parse("<Root xmlns='rootns'>\r\n\u{1F600}<A/>\r<B/></Root>");
    let root = root_object(&doc);
    let children = root.children.borrow();
    let a = children[1]
        .cast::<XamlAstObjectNode>()
        .expect("object node");
    // The emoji takes two UTF-16 code units.
    assert_eq!((a.line(), a.position()), (2, 4));
    let b = children[3]
        .cast::<XamlAstObjectNode>()
        .expect("object node");
    assert_eq!((b.line(), b.position()), (3, 2));
}

#[test]
fn xmlns_declarations_are_only_allowed_on_the_root_element() {
    let error = parse_error("<Root xmlns='rootns'>\n  <Child a='1' xmlns:t='testns'/>\n</Root>");
    match error {
        XamlError::Parse(e) => {
            assert_eq!(
                e.title,
                "xmlns declarations are only allowed on the root element to preserve memory"
            );
            assert_eq!((e.line_number, e.line_position), (2, 16));
        }
        other => panic!("Unexpected error {other}"),
    }

    // Redeclaring an identical namespace is rejected as well.
    assert!(matches!(
        parse_error("<Root xmlns='rootns'><Child xmlns='rootns'/></Root>"),
        XamlError::Parse(_)
    ));
}

#[test]
fn dots_are_not_allowed_in_type_names() {
    match parse_error("<Root.Dotted xmlns='rootns'/>") {
        XamlError::Parse(e) => {
            assert_eq!(e.title, "Dots aren't allowed in type names");
            assert_eq!((e.line_number, e.line_position), (1, 2));
        }
        other => panic!("Unexpected error {other}"),
    }
}

#[test]
fn attributes_are_not_allowed_on_element_properties() {
    match parse_error("<Root xmlns='rootns'><Root.Prop a='1'>x</Root.Prop></Root>") {
        XamlError::Parse(e) => {
            assert_eq!(e.title, "Attributes aren't allowed on element properties");
            assert_eq!((e.line_number, e.line_position), (1, 23));
        }
        other => panic!("Unexpected error {other}"),
    }
}

#[test]
fn malformed_xml_is_reported_as_xml_exception_with_position() {
    let error = parse_error("<Root xmlns='rootns'>\n  <Child>\n</Root>");
    assert!(matches!(error, XamlError::Xml(_)), "{error}");
    assert!(error.is_xml_exception());
    assert!(!error.is_xaml_parse_exception());
    assert!(error.line_number().is_some_and(|l| l > 0));

    assert!(matches!(parse_error(""), XamlError::Xml(_)));
    assert!(matches!(parse_error("not xml"), XamlError::Xml(_)));
    assert!(matches!(
        parse_error("<Root xmlns='rootns' a='1' a='2'/>"),
        XamlError::Xml(_)
    ));
    assert!(matches!(parse_error("<t:Root/>"), XamlError::Xml(_)));
}

#[test]
fn invalid_type_arguments_are_reported() {
    let error = parse_error(&format!(
        "<Root xmlns='rootns' xmlns:x='{X}' x:TypeArguments='A)'/>"
    ));
    match error {
        XamlError::Parse(e) => {
            assert_eq!(e.title, "Unmatched ')' at position 1");
            assert_eq!((e.line_number, e.line_position), (1, 77));
        }
        other => panic!("Unexpected error {other}"),
    }
}

#[test]
fn xml_space_marks_text_nodes_and_is_inherited() {
    let doc = parse(
        "<Root xmlns='rootns'><A xml:space='preserve'> a <B> b </B><C xml:space='default'> c </C></A><D> d </D></Root>",
    );
    let root = root_object(&doc);
    assert_eq!(
        dump(&root.as_node()).replace(X, "x"),
        "(Object xml!!rootns:Root args=[] children=[\
         (Object xml!!rootns:A args=[] children=[(Text \" a \" preserve=true type=xml!!x:String), \
         (Object xml!!rootns:B args=[] children=[(Text \" b \" preserve=true type=xml!!x:String)]), \
         (Object xml!!rootns:C args=[] children=[(Text \" c \" preserve=false type=xml!!x:String)])]), \
         (Object xml!!rootns:D args=[] children=[(Text \" d \" preserve=false type=xml!!x:String)])])"
    );
}

fn single_text(xaml: &str) -> String {
    let doc = parse(xaml);
    let root = root_object(&doc);
    let children = root.children.borrow();
    assert_eq!(children.len(), 1, "{}", dump(&root.as_node()));
    children[0]
        .cast::<XamlAstTextNode>()
        .expect("text node")
        .text()
}

#[test]
fn entities_cdata_and_line_endings() {
    assert_eq!(
        single_text("<Root xmlns='rootns'>a &lt;&amp;&gt; &#x41;&#66;</Root>"),
        "a <&> AB"
    );
    assert_eq!(
        single_text("<Root xmlns='rootns'>a\r\nb\rc</Root>"),
        "a\nb\nc"
    );
    assert_eq!(single_text("<Root xmlns='rootns'>a&#xD;b</Root>"), "a\rb");
    assert_eq!(
        single_text("<Root xmlns='rootns'><![CDATA[<raw> & text]]></Root>"),
        "<raw> & text"
    );
}

#[test]
fn document_type_declarations_are_ignored() {
    // As with DtdProcessing.Ignore, the declaration is skipped and its entities are unknown.
    let xaml = "<!DOCTYPE Root [\n<!ENTITY e 'ent>ity'>\n<!-- ] > -->\n]>\n<Root xmlns='rootns'>text</Root>";
    let doc = parse(xaml);
    let root = root_object(&doc);
    assert_eq!((root.line(), root.position()), (5, 2));
    assert_eq!(single_text(xaml), "text");
    assert!(matches!(
        parse_error("<!DOCTYPE Root [<!ENTITY e 'entity'>]><Root xmlns='rootns'>an &e;</Root>"),
        XamlError::Xml(_)
    ));
}

#[test]
fn comments_and_processing_instructions_split_text_nodes() {
    let doc = parse("<Root xmlns='rootns'>a<!-- c -->b<?pi x?>c</Root>");
    let root = root_object(&doc);
    let texts: Vec<String> = root
        .children
        .borrow()
        .iter()
        .map(|c| c.cast::<XamlAstTextNode>().expect("text node").text())
        .collect();
    assert_eq!(texts, vec!["a", "b", "c"]);
}

#[test]
fn attribute_values_are_normalized_by_the_xml_parser() {
    let doc = parse("<Root xmlns='rootns' A=' \n\tX&#x9;&#xA; ' B='&quot;q&quot;'/>");
    let root = root_object(&doc);
    let values: Vec<String> = root
        .children
        .borrow()
        .iter()
        .map(|c| {
            let prop = c
                .cast::<XamlAstXamlPropertyValueNode>()
                .expect("property node");
            let value = prop.values.borrow()[0].clone();
            value.cast::<XamlAstTextNode>().expect("text node").text()
        })
        .collect();
    assert_eq!(values, vec!["   X\t\n ", "\"q\""]);
}

#[test]
fn xml_prefixed_attributes_are_ignored_and_order_is_preserved() {
    let doc = parse(&format!(
        "<Root xmlns='rootns' xmlns:x='{X}' xml:lang='en' C='3' x:Name='n' A='1' xml:space='default' B='2'/>"
    ));
    let root = root_object(&doc);
    let names: Vec<String> = root
        .children
        .borrow()
        .iter()
        .map(|c| {
            if let Some(d) = c.cast::<XamlAstXmlDirective>() {
                return format!("directive:{}", d.name.borrow());
            }
            let prop = c
                .cast::<XamlAstXamlPropertyValueNode>()
                .expect("property node");
            prop.property()
                .cast::<XamlAstNamePropertyReference>()
                .expect("name reference")
                .name()
        })
        .collect();
    assert_eq!(names, vec!["C", "directive:Name", "A", "B"]);
}

#[test]
fn elements_without_namespace_and_unknown_prefixes() {
    let doc = parse("<Root Prop='{t:Ext}' Other.P='1'/>");
    assert!(doc.namespace_aliases.is_empty());
    let root = root_object(&doc);
    // As upstream, an undeclared prefix inside a markup extension resolves to the empty namespace.
    assert_eq!(
        dump(&root.as_node()).replace(X, "x"),
        "(Object xml!!:Root args=[] children=[\
         (Prop (NameProp xml!!:Root.Prop target=xml!!:Root) attr=true [(Object xml!!:Ext{ME} args=[] children=[])]), \
         (Prop (NameProp xml!!:Other.P target=xml!!:Root) attr=true [(Text \"1\" preserve=true type=xml!!x:String)])])"
    );
}

#[test]
fn ignorable_scopes_are_nested_and_restored() {
    let doc = parse(
        "<Root xmlns='rootns' xmlns:mc='http://schemas.openxmlformats.org/markup-compatibility/2006' xmlns:d='design' xmlns:e='extra'>\
         <A mc:Ignorable='d'><d:Skipped><B/></d:Skipped><e:Kept/><C mc:Ignorable='e' e:Attr='1'><e:Skipped/></C><e:KeptAgain/></A>\
         <d:Kept d:Attr='1'/></Root>",
    );
    let root = root_object(&doc);
    assert_eq!(
        dump(&root.as_node()).replace(X, "x"),
        "(Object xml!!rootns:Root args=[] children=[\
         (Object xml!!rootns:A args=[] children=[(Object xml!!extra:Kept args=[] children=[]), \
         (Object xml!!rootns:C args=[] children=[]), (Object xml!!extra:KeptAgain args=[] children=[])]), \
         (Object xml!!design:Kept args=[] children=[(Directive design:Attr [(Text \"1\" preserve=true type=xml!!x:String)])])])"
    );
}

#[test]
fn deeply_nested_markup_is_rejected_instead_of_overflowing_the_stack() {
    let depth = 5000;
    let xaml = format!(
        "<Root xmlns='rootns'>{}{}</Root>",
        "<A>".repeat(depth),
        "</A>".repeat(depth)
    );
    assert!(XDocumentXamlParser::parse(&xaml, None).is_err());

    let extension = format!("{}{}", "{A ".repeat(depth), "}".repeat(depth));
    assert!(parse_extension(&extension).is_err());
}

#[test]
fn comma_separated_parentheses_tree_parser() {
    fn show(nodes: &[crate::parsers::Node]) -> String {
        nodes
            .iter()
            .map(|n| {
                let value = n.value.clone().unwrap_or_else(|| "<null>".to_string());
                if n.children.is_empty() {
                    value
                } else {
                    format!("{value}({})", show(&n.children))
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }
    let parse = |s: &str| {
        CommaSeparatedParenthesesTreeParser::parse(s)
            .map(|n| show(&n))
            .map_err(|e| e.message)
    };

    assert_eq!(parse("A"), Ok("A".to_string()));
    assert_eq!(parse("A,B(C,D(E)) ,F"), Ok("A,B(C,D(E)),F".to_string()));
    assert_eq!(parse(""), Ok("<null>".to_string()));
    assert_eq!(parse("A(B"), Ok("B".to_string()));
    assert_eq!(parse("A)"), Err("Unmatched ')' at position 1".to_string()));
    assert_eq!(
        parse("A(B)C"),
        Err("Invalid character after ')' at position 4".to_string())
    );
}
