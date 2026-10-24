//! AST level tests of the transformers, run on the fake type system. They cover the behavior
//! the upstream suite verifies by running the generated code (type and property resolution,
//! intrinsics, converters, content properties, diagnostics).

use std::rc::Rc;

use crate::ast::*;
use crate::diagnostics::{throw_exception_if_any_error, XamlDiagnosticSeverity};
use crate::exceptions::XamlError;

use super::helpers::dump;
use super::test_xaml_language::{TestHost, TestHostOptions};
use super::whitespace_tests::{assignments, property_values};

const X: &str = "xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

fn root(xaml: &str) -> Rc<dyn IXamlAstNode> {
    TestHost::new().transform_root(xaml)
}

fn root_type_name(node: &Rc<dyn IXamlAstNode>) -> String {
    node.cast::<dyn IXamlAstValueNode>()
        .expect("value node")
        .type_()
        .get_clr_type()
        .expect("clr type")
        .full_name()
}

/// The dump of the single value assigned to `property`.
fn assigned(node: &Rc<dyn IXamlAstNode>, property: &str) -> String {
    let values = property_values(node, property);
    assert_eq!(values.len(), 1, "{}", dump(node));
    dump(&values[0].as_node())
}

fn intrinsics(attributes: &str) -> Rc<dyn IXamlAstNode> {
    root(&format!(
        "<IntrinsicsTestsClass xmlns='test' {X} {attributes}/>"
    ))
}

#[test]
fn types_are_resolved_through_xmlns_definitions_and_clr_namespaces() {
    for xmlns in [
        "test",
        "clr-namespace:XamlParserTests",
        "clr-namespace:XamlParserTests;assembly=XamlParserTests",
        "using:XamlParserTests",
    ] {
        let node = root(&format!("<Control xmlns='{xmlns}'/>"));
        assert_eq!(
            dump(&node),
            "(Constructable clr!!XamlParserTests.Control args=[] children=[])"
        );
    }

    // The System namespace is reachable through the XAML namespace
    let node = root(&format!(
        "<ContentControl xmlns='test' {X}><x:String>abc</x:String></ContentControl>"
    ));
    assert_eq!(
        assigned(&node, "Content"),
        "(Text \"abc\" preserve=false type=clr!!System.String)"
    );
}

#[test]
fn unresolved_types_are_reported() {
    let host = TestHost::new();
    let doc = host
        .transform("<ContentControl xmlns='test'>\n  <Missing/>\n</ContentControl>")
        .expect("non-fatal");
    let errors = host.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].title,
        "Unable to resolve type Missing from namespace test Line 2, position 4."
    );
    assert_eq!(
        (errors[0].line_number, errors[0].line_position),
        (Some(2), Some(4))
    );
    assert_eq!(errors[0].severity, XamlDiagnosticSeverity::Error);
    // The failed type reference is replaced with the unknown pseudo type; a value of that type is
    // not assigned (and not reported again), and compilation goes on.
    assert_eq!(
        dump(&doc.root().expect("root")),
        "(Constructable clr!!XamlParserTests.ContentControl args=[] children=[(Group [])])"
    );

    let fatal = TestHost::with_options(TestHostOptions {
        errors_are_fatal: true,
        custom_value_converter: None,
    });
    match fatal.transform("<ContentControl xmlns='test'>\n  <Missing/>\n</ContentControl>") {
        Err(XamlError::Transform(e)) => {
            assert_eq!(
                e.title,
                "Unable to resolve type Missing from namespace test"
            );
            assert_eq!((e.line_number, e.line_position), (2, 4));
        }
        other => panic!(
            "Expected a transform exception, got {:?}",
            other.map(|_| ())
        ),
    }
}

#[test]
fn generic_types_are_resolved_from_type_arguments() {
    let node = root(&format!(
        "<GenericClass xmlns='test' {X} x:TypeArguments='x:Int32' Value='5'/>"
    ));
    assert_eq!(
        root_type_name(&node),
        "XamlParserTests.GenericClass`1[System.Int32]"
    );
    assert_eq!(assigned(&node, "Value"), "(Const clr!!System.Int32 5i32)");

    let node = root(&format!(
        "<GenericClass xmlns='test' {X} xmlns:scg='clr-namespace:System.Collections.Generic;assembly=System.Runtime' \
         x:TypeArguments='scg:List(Control)'/>"
    ));
    assert_eq!(
        root_type_name(&node),
        "XamlParserTests.GenericClass`1[System.Collections.Generic.List`1[XamlParserTests.Control]]"
    );
}

#[test]
fn attribute_values_are_converted_to_the_property_type() {
    let node = intrinsics(
        "IntProperty='5' NullableIntProperty='7' DoubleProperty='1.5' BoolProperty='True' EnumProperty='B' \
         FlagsProperty='One, Four' ObjectProperty='text' TypeProperty='Control'",
    );
    assert_eq!(
        assigned(&node, "IntProperty"),
        "(Const clr!!System.Int32 5i32)"
    );
    assert_eq!(
        assigned(&node, "NullableIntProperty"),
        "(Const clr!!System.Int32 7i32)"
    );
    assert_eq!(
        assigned(&node, "DoubleProperty"),
        "(Const clr!!System.Double 1.5f64)"
    );
    assert_eq!(
        assigned(&node, "BoolProperty"),
        "(Const clr!!System.Boolean true)"
    );
    assert_eq!(
        assigned(&node, "EnumProperty"),
        "(Const clr!!XamlParserTests.TestEnum 2i32)"
    );
    assert_eq!(
        assigned(&node, "FlagsProperty"),
        "(Const clr!!XamlParserTests.FlagsEnum 5i32)"
    );
    assert_eq!(
        assigned(&node, "ObjectProperty"),
        "(Text \"text\" preserve=true type=clr!!System.String)"
    );
    assert_eq!(
        assigned(&node, "TypeProperty"),
        "(TypeExt clr!!XamlParserTests.Control)"
    );
}

#[test]
fn parse_methods_and_type_converters_are_used() {
    let node =
        intrinsics("ParseableProperty='p' CultureParseableProperty='c' ConvertedProperty='v'");
    assert_eq!(
        assigned(&node, "ParseableProperty"),
        "(XamlStaticOrTargetedReturnMethodCallNode ParseableStruct.Parse [(Text \"p\" preserve=true type=clr!!System.String)])"
    );
    assert_eq!(
        assigned(&node, "CultureParseableProperty"),
        "(XamlStaticOrTargetedReturnMethodCallNode CultureParseableStruct.Parse [(Text \"c\" preserve=true type=clr!!System.String), \
         (XamlStaticOrTargetedReturnMethodCallNode CultureInfo.get_InvariantCulture [])])"
    );
    assert_eq!(
        assigned(&node, "ConvertedProperty"),
        "(XamlAstNeedsParentStackValueNode (Cast clr!!XamlParserTests.ConvertedClass \
         (XamlStaticOrTargetedReturnMethodCallNode TypeConverter.ConvertFrom [\
         (New clr!!XamlParserTests.ConvertedClassConverter args=[]), \
         (ContextLocal clr!!System.ComponentModel.ITypeDescriptorContext), \
         (XamlStaticOrTargetedReturnMethodCallNode CultureInfo.get_InvariantCulture []), \
         (Text \"v\" preserve=true type=clr!!System.String)])))"
    );
    let value = property_values(&node, "ConvertedProperty").remove(0);
    assert!(value
        .as_needs_parent_stack()
        .is_some_and(|n| n.needs_parent_stack()));
}

#[test]
fn conversion_failures_are_reported_with_line_info() {
    let host = TestHost::new();
    host.transform("<IntrinsicsTestsClass xmlns='test'\n   IntProperty='abc'/>")
        .expect("non-fatal");
    let errors = host.errors();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert_eq!(
        errors[0].title,
        "The input string 'abc' was not in a correct format. Line 2, position 4."
    );
    assert!(matches!(errors[0].to_exception(), XamlError::Parse(_)));

    let host = TestHost::new();
    host.transform("<IntrinsicsTestsClass xmlns='test'><IntrinsicsTestsClass.IntProperty><Control/></IntrinsicsTestsClass.IntProperty></IntrinsicsTestsClass>")
        .expect("non-fatal");
    let errors = host.errors();
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert!(
        errors[0].title.starts_with(
            "Unable to find suitable setter or adder for property IntProperty of type XamlParserTests:XamlParserTests.IntrinsicsTestsClass \
             for argument XamlParserTests:XamlParserTests.Control, available setter parameter lists are:\nSystem.Int32"
        ),
        "{}",
        errors[0].title
    );
    assert!(matches!(errors[0].to_exception(), XamlError::Load(_)));
}

#[test]
fn custom_value_converter_is_consulted_first() {
    let host = TestHost::with_options(TestHostOptions {
        errors_are_fatal: false,
        custom_value_converter: Some(Rc::new(|_context, node, _attributes, type_| {
            if type_.name() != "Int32" {
                return Ok(None);
            }
            let Some(text) = node.cast::<XamlAstTextNode>() else {
                return Ok(None);
            };
            let value = text.text().len() as i32;
            Ok(Some(XamlConstantNode::new(
                &**node,
                type_.clone(),
                crate::type_system::XamlValue::Int32(value),
            )?))
        })),
    });
    let node = host.transform_root(
        "<IntrinsicsTestsClass xmlns='test' IntProperty='four' DoubleProperty='2'/>",
    );
    assert_eq!(
        assigned(&node, "IntProperty"),
        "(Const clr!!System.Int32 4i32)"
    );
    assert_eq!(
        assigned(&node, "DoubleProperty"),
        "(Const clr!!System.Double 2f64)"
    );
}

#[test]
fn null_and_boolean_intrinsics() {
    let node = intrinsics("ObjectProperty='{x:Null}' BoolProperty='{x:True}'");
    assert_eq!(assigned(&node, "ObjectProperty"), "(Null)");
    assert_eq!(
        assigned(&node, "BoolProperty"),
        "(Const clr!!System.Boolean true)"
    );

    let node = root(&format!(
        "<IntrinsicsTestsClass xmlns='test' {X}><IntrinsicsTestsClass.ObjectProperty><x:False/></IntrinsicsTestsClass.ObjectProperty></IntrinsicsTestsClass>"
    ));
    assert_eq!(
        assigned(&node, "ObjectProperty"),
        "(Const clr!!System.Boolean false)"
    );

    // x:Null can't be assigned to a value type
    let host = TestHost::new();
    host.transform(&format!(
        "<IntrinsicsTestsClass xmlns='test' {X} IntProperty='{{x:Null}}'/>"
    ))
    .expect("non-fatal");
    assert_eq!(host.errors().len(), 1);
    assert!(host.errors()[0]
        .title
        .contains("Unable to find suitable setter or adder for property IntProperty"));
}

#[test]
fn type_extension_resolves_types() {
    for (expected, type_ext) in [
        ("XamlParserTests.Control", "{x:Type Control}"),
        ("XamlParserTests.Control", "{x:Type TypeName=Control}"),
        ("System.String", "{x:Type x:String}"),
        (
            "System.Collections.Generic.List`1[System.Int32]",
            "{x:Type scg:List, x:TypeArguments=x:Int32}",
        ),
    ] {
        let node = intrinsics(&format!(
            "xmlns:scg='clr-namespace:System.Collections.Generic;assembly=System.Runtime' TypeProperty='{type_ext}'"
        ));
        assert_eq!(
            assigned(&node, "TypeProperty"),
            format!("(TypeExt clr!!{expected})"),
            "{type_ext}"
        );
    }
}

#[test]
fn static_extension_resolves_members() {
    let node = intrinsics("ObjectProperty='{x:Static IntrinsicsTestsClass.StaticProp}'");
    assert_eq!(
        assigned(&node, "ObjectProperty"),
        "(Static clr!!XamlParserTests.IntrinsicsTestsClass.StaticProp)"
    );
    let value = property_values(&node, "ObjectProperty").remove(0);
    assert_eq!(
        value.type_().get_clr_type().expect("clr type").full_name(),
        "System.String"
    );

    let node = intrinsics("IntProperty='{x:Static Member=IntrinsicsTestsClass.ConstField}' EnumProperty='{x:Static TestEnum.B}'");
    let value = property_values(&node, "IntProperty").remove(0);
    assert_eq!(
        value.type_().get_clr_type().expect("clr type").full_name(),
        "System.Int32"
    );
    let value = property_values(&node, "EnumProperty").remove(0);
    let static_node = value
        .cast::<XamlStaticExtensionNode>()
        .expect("static extension");
    match static_node.resolve_member(true).expect("resolved") {
        Some(XamlStaticMember::Field(field)) => {
            assert_eq!(
                field.get_literal_value().expect("literal"),
                crate::type_system::XamlValue::Int32(2)
            )
        }
        _ => panic!("Expected a field"),
    }
}

#[test]
fn static_extension_reports_errors() {
    let host = TestHost::new();
    host.transform(&format!(
        "
<IntrinsicsTestsClass 
    xmlns='test' 
    {X}
    xmlns:scg='clr-namespace:System.Collections.Generic'
>
    <IntrinsicsTestsClass.ObjectProperty><x:Static Member='IntrinsicsTestsClass.StaticPropDoesntExist1'/></IntrinsicsTestsClass.ObjectProperty>
    <IntrinsicsTestsClass.BoolProperty><x:Static Member='IntrinsicsTestsClass.StaticPropDoesntExist2'/></IntrinsicsTestsClass.BoolProperty>
</IntrinsicsTestsClass>"
    ))
    .expect("errors are collected");

    let result = throw_exception_if_any_error(&host.diagnostics.borrow());
    match result {
        Err(XamlError::Aggregate(inner)) => {
            assert_eq!(inner.len(), 2);
            assert!(
                inner[0].message().contains("StaticPropDoesntExist1"),
                "{}",
                inner[0]
            );
            assert!(
                inner[1].message().contains("StaticPropDoesntExist2"),
                "{}",
                inner[1]
            );
            assert!(inner.iter().all(|e| matches!(e, XamlError::Transform(_))));
        }
        other => panic!("Expected an aggregate exception, got {other:?}"),
    }
}

#[test]
fn markup_extensions_are_wrapped() {
    let node = intrinsics("ObjectProperty='{Test 5, Prop=x}'");
    assert_eq!(
        assigned(&node, "ObjectProperty"),
        "(MarkupExt ProvideValue(1) (Constructable clr!!XamlParserTests.TestExtension{ME} \
         args=[(Text \"5\" preserve=true type=clr!!System.String)] \
         children=[(Assign TestExtension.Prop [setter(Object)] [(Text \"x\" preserve=true type=clr!!System.String)])]))"
    );
    let value = property_values(&node, "ObjectProperty").remove(0);
    assert!(value
        .as_needs_parent_stack()
        .is_some_and(|n| n.needs_parent_stack()));

    // The typed, parameterless ProvideValue is preferred
    let node = intrinsics("IntProperty='{Typed}'");
    assert_eq!(
        assigned(&node, "IntProperty"),
        "(MarkupExt ProvideValue(0) (Constructable clr!!XamlParserTests.TypedExtension{ME} args=[] children=[]))"
    );
    let value = property_values(&node, "IntProperty").remove(0);
    assert!(value
        .as_needs_parent_stack()
        .is_some_and(|n| !n.needs_parent_stack()));

    let host = TestHost::new();
    host.transform("<IntrinsicsTestsClass xmlns='test' ObjectProperty='{Broken}'/>")
        .expect("non-fatal");
    assert_eq!(
        host.errors()[0].title,
        "XamlParserTests:XamlParserTests.BrokenExtension was resolved as markup extension, \
         but doesn't have a matching ProvideValue/ProvideTypedValue method Line 1, position 36."
    );
}

#[test]
fn attached_properties_and_events_are_resolved() {
    let node = root("<Control xmlns='test' AttachedProps.Foo='x' StrProp='y'/>");
    assert_eq!(
        dump(&node),
        "(Constructable clr!!XamlParserTests.Control args=[] children=[\
         (Assign AttachedProps.Foo [setter(String)] [(Text \"x\" preserve=true type=clr!!System.String)]), \
         (Assign Control.StrProp [setter(String)] [(Text \"y\" preserve=true type=clr!!System.String)])])"
    );
    let attached = &assignments(&node)[0];
    assert!(attached
        .property
        .getter()
        .is_some_and(|g| g.name() == "GetFoo"));
    assert_eq!(
        attached.possible_setters.borrow()[0]
            .target_type()
            .full_name(),
        "XamlParserTests.Control"
    );

    let node = intrinsics("Click='OnClick'");
    assert_eq!(
        dump(&node),
        "(Constructable clr!!XamlParserTests.IntrinsicsTestsClass args=[] children=[\
         (Assign IntrinsicsTestsClass.Click [setter(Action)] [(LoadMethodDelegate OnClick (XamlRootObjectNode))])])"
    );

    let host = TestHost::new();
    host.transform("<Control xmlns='test' Nope='1'/>")
        .expect("non-fatal");
    assert_eq!(
        host.errors()[0].title,
        "Unable to resolve suitable regular or attached property Nope on type XamlParserTests:XamlParserTests.Control \
         Line 1, position 2."
    );
}

#[test]
fn compiler_should_support_content_attribute() {
    let node = root(&format!(
        "<SimpleClassWithContentAttribute xmlns='test'  {X}>123</SimpleClassWithContentAttribute>"
    ));
    assert_eq!(
        assigned(&node, "Text"),
        "(Text \"123\" preserve=false type=clr!!System.String)"
    );
}

#[test]
fn compiler_should_support_content_attribute_override() {
    let node =
        root(&format!("<SubClassWithContentAttributeOverride xmlns='test'  {X}>123</SubClassWithContentAttributeOverride>"));
    assert!(property_values(&node, "Text").is_empty());
    assert_eq!(
        assigned(&node, "OtherText"),
        "(Text \"123\" preserve=false type=clr!!System.String)"
    );
}

#[test]
fn compiler_should_fail_to_build_xaml_when_multiple_content_attributes_defined() {
    let host = TestHost::new();
    host.transform("\n<SimpleClassWithTwoContentAttributes xmlns='test'>123</SimpleClassWithTwoContentAttributes>")
        .expect("errors are collected");
    let result = throw_exception_if_any_error(&host.diagnostics.borrow());
    match result {
        Err(XamlError::Transform(e)) => {
            assert_eq!(
                e.title,
                "Internal compiler error: Content attribute is declared on multiple properties of \
                 XamlParserTests:XamlParserTests.SimpleClassWithTwoContentAttributes (ResolveContentPropertyTransformer)"
            );
            assert_eq!((e.line_number, e.line_position), (2, 2));
            assert!(matches!(
                e.inner_exception.as_deref(),
                Some(XamlError::TypeSystem(_))
            ));
        }
        other => panic!("Expected a transform exception, got {other:?}"),
    }
}

#[test]
fn collections_get_add_method_setters() {
    let node = root("<ListHost xmlns='test'><Control/><Control/></ListHost>");
    let list = assignments(&node);
    assert_eq!(list.len(), 2);
    assert_eq!(
        dump(&list[0].as_node()),
        "(Assign ListHost.Content [setter(Control) setter(Control)] [(Constructable clr!!XamlParserTests.Control args=[] children=[])])"
    );
    assert!(list[0]
        .possible_setters
        .borrow()
        .iter()
        .all(|s| s.binder_parameters().allow_multiple.get()));

    // IAddChild<T> and IAddChild
    let node = root("<AddChildHost xmlns='test'><Control/></AddChildHost>");
    assert_eq!(
        dump(&assignments(&node)[0].as_node()),
        "(Assign AddChildHost.Content [setter(Control) setter(Object)] [(Constructable clr!!XamlParserTests.Control args=[] children=[])])"
    );

    let host = TestHost::new();
    host.transform("<Control xmlns='test'><Control/></Control>")
        .expect("non-fatal");
    assert_eq!(
        host.errors()[0].title,
        "No Content property or any Add methods found for type XamlParserTests:XamlParserTests.Control Line 1, position 24."
    );
}

#[test]
fn dictionary_content_uses_x_key() {
    let node = root(&format!("<DictionaryHost xmlns='test' {X}><Control x:Key='a'/><x:String x:Key='b'>text</x:String></DictionaryHost>"));
    let items = assignments(&node);
    assert_eq!(items.len(), 2);
    assert_eq!(
        dump(&items[0].as_node()),
        "(Assign DictionaryHost.Items [adder(String,Object)] [(Text \"a\" preserve=true type=clr!!System.String), \
         (Constructable clr!!XamlParserTests.Control args=[] children=[])])"
    );
    assert_eq!(
        dump(&items[1].as_node()),
        "(Assign DictionaryHost.Items [adder(String,Object)] [(Text \"b\" preserve=true type=clr!!System.String), \
         (XamlValueWithManipulationNode (Text \"text\" preserve=false type=clr!!System.String) (Group []))])"
    );

    // A value without a key can't be added to a dictionary
    let host = TestHost::new();
    host.transform("<DictionaryHost xmlns='test'><Control/></DictionaryHost>")
        .expect("non-fatal");
    assert!(host.errors()[0]
        .title
        .starts_with("Unable to find suitable setter or adder for property Items"));
}

#[test]
fn constructor_arguments_are_converted() {
    let node = root(&format!(
        "<ContentControl xmlns='test' {X}><CtorClass><x:Arguments><x:Int32>5</x:Int32> <x:String>abc</x:String></x:Arguments></CtorClass></ContentControl>"
    ));
    assert_eq!(
        assigned(&node, "Content"),
        "(Constructable clr!!XamlParserTests.CtorClass args=[(Const clr!!System.Int32 5i32), \
         (Text \"abc\" preserve=false type=clr!!System.String)] children=[])"
    );

    // String arguments are converted to the parameter types
    let node = root(&format!(
        "<ContentControl xmlns='test' {X}><CtorClass><x:Arguments><x:String>7</x:String><x:String>abc</x:String></x:Arguments></CtorClass></ContentControl>"
    ));
    assert!(
        assigned(&node, "Content").contains("args=[(Const clr!!System.Int32 7i32), (Text \"abc\"")
    );

    // A nested object needs a matching constructor
    let host = TestHost::new();
    host.transform("<ContentControl xmlns='test'><CtorClass/></ContentControl>")
        .expect("non-fatal");
    assert_eq!(
        host.errors()[0].title,
        "Unable to find public constructor for type XamlParserTests:XamlParserTests.CtorClass() Line 1, position 31."
    );
    assert!(matches!(
        host.errors()[0].to_exception(),
        XamlError::Load(_)
    ));

    // The root object doesn't: it is usually populated rather than constructed
    let node = root("<CtorClass xmlns='test'/>");
    assert_eq!(
        dump(&node),
        "(Constructable clr!!XamlParserTests.CtorClass args=[] children=[])"
    );

    let host = TestHost::new();
    host.transform(&format!(
        "<CtorClass xmlns='test' {X}><x:Arguments><x:Int32>1</x:Int32></x:Arguments><x:Arguments><x:Int32>2</x:Int32></x:Arguments></CtorClass>"
    ))
    .expect("non-fatal");
    assert!(host.errors().iter().any(|e| e
        .title
        .starts_with("x:Arguments directive is specified more than once")));
}

#[test]
fn known_directives_become_directive_nodes() {
    let host = TestHost::new();
    host.configuration
        .known_directives
        .borrow_mut()
        .push(("test".to_string(), "MyDirective".to_string()));
    let node = host.transform_root("<ContentControl xmlns='test'><MyDirective/></ContentControl>");
    assert_eq!(
        dump(&node),
        "(Constructable clr!!XamlParserTests.ContentControl args=[] children=[(Directive test:MyDirective [])])"
    );
}

fn verify_diagnostic(host: &TestHost, title: &str, code: &str, severity: XamlDiagnosticSeverity) {
    let diagnostics = host.diagnostics.borrow();
    assert!(
        diagnostics
            .iter()
            .any(|d| d.title == title && d.code == code && d.severity == severity),
        "Missing diagnostic '{title}' in {:#?}",
        diagnostics
            .iter()
            .map(|d| (&d.title, &d.code, d.severity))
            .collect::<Vec<_>>()
    );
}

#[test]
fn obsolete_is_reported() {
    let host = TestHost::new();
    host.transform(&format!(
        "
<ObsoleteClass 
    xmlns='test' 
    {X}
>
    <ObsoleteClass.ObjectProperty><x:Static Member='ObsoleteClass.StaticProp'/></ObsoleteClass.ObjectProperty>
</ObsoleteClass>"
    ))
    .expect("transform");

    verify_diagnostic(
        &host,
        "'ObsoleteClass' is obsolete: ObsoleteClass is obsolete",
        "Obsolete",
        XamlDiagnosticSeverity::Warning,
    );
    verify_diagnostic(
        &host,
        "'ObsoleteClass.ObjectProperty' is obsolete: ObjectProperty is obsolete",
        "Obsolete",
        XamlDiagnosticSeverity::Warning,
    );
    verify_diagnostic(
        &host,
        "'ObsoleteClass.StaticProp' is obsolete: StaticProp is obsolete",
        "Obsolete",
        XamlDiagnosticSeverity::Error,
    );
    let diagnostics = host.diagnostics.borrow();
    assert_eq!(diagnostics.len(), 3);
    assert_eq!(
        (diagnostics[0].line_number, diagnostics[0].line_position),
        (Some(2), Some(2))
    );
}

#[test]
fn experimental_is_reported() {
    let host = TestHost::new();
    host.transform(&format!(
        "
<ExperimentalClass 
    xmlns='test' 
    {X}
>
    <ExperimentalClass.ObjectProperty><x:Static Member='ExperimentalClass.StaticProp'/></ExperimentalClass.ObjectProperty>
</ExperimentalClass>"
    ))
    .expect("transform");

    for member in [
        "ExperimentalClass",
        "ExperimentalClass.ObjectProperty",
        "ExperimentalClass.StaticProp",
    ] {
        verify_diagnostic(
            &host,
            &format!("'{member}' is for evaluation purposes only and is subject to change or removal in future updates."),
            "FOO123",
            XamlDiagnosticSeverity::Warning,
        );
    }
}

#[test]
fn experimental_with_message_is_reported() {
    let host = TestHost::new();
    host.transform(&format!(
        "
<ExperimentalWithMessageClass 
    xmlns='test' 
    {X}
>
    <ExperimentalWithMessageClass.ObjectProperty><x:Static Member='ExperimentalWithMessageClass.StaticProp'/></ExperimentalWithMessageClass.ObjectProperty>
</ExperimentalWithMessageClass>"
    ))
    .expect("transform");

    for (member, message) in [
        (
            "ExperimentalWithMessageClass",
            "ExperimentalClass is experimental",
        ),
        (
            "ExperimentalWithMessageClass.ObjectProperty",
            "ObjectProperty is experimental",
        ),
        (
            "ExperimentalWithMessageClass.StaticProp",
            "StaticProp is experimental",
        ),
    ] {
        verify_diagnostic(
            &host,
            &format!("'{member}' is for evaluation purposes only and is subject to change or removal in future updates: '{message}'."),
            "FOO123",
            XamlDiagnosticSeverity::Warning,
        );
    }
}

#[test]
fn failed_nodes_are_replaced_with_skip_nodes() {
    let host = TestHost::new();
    let doc = host
        .transform("<ContentControl xmlns='test'>\n <ContentControl.Content>\n  <Control Nope='1' StrProp='ok'/>\n </ContentControl.Content>\n</ContentControl>")
        .expect("non-fatal");
    assert_eq!(host.errors().len(), 1);
    // The unresolved property becomes a property of the unknown pseudo type and its assignment is dropped,
    // the rest of the document is transformed as usual.
    let root = doc.root().expect("root");
    assert_eq!(
        dump(&root),
        "(Constructable clr!!XamlParserTests.ContentControl args=[] children=[\
         (Assign ContentControl.Content [setter(Object)] [(Constructable clr!!XamlParserTests.Control args=[] children=[\
         (Group []), (Assign Control.StrProp [setter(String)] [(Text \"ok\" preserve=true type=clr!!System.String)])])]), \
         (Group [])])"
    );

    // An internal failure replaces the node with a skip node
    let host = TestHost::new();
    let doc = host
        .transform("<ContentControl xmlns='test'><SimpleClassWithTwoContentAttributes>1</SimpleClassWithTwoContentAttributes></ContentControl>")
        .expect("non-fatal");
    assert_eq!(host.errors().len(), 1);
    let root = doc.root().expect("root");
    let skipped = property_values(&root, "Content");
    assert!(skipped.is_empty(), "{}", dump(&root));
    assert!(dump(&root).contains("Group []"), "{}", dump(&root));
}

#[test]
fn imperative_transformers_build_initialization_nodes() {
    let host = TestHost::new();
    let doc = host
        .transform_imperative(
            "<ContentControl xmlns='test' StrProp='s'><Control/></ContentControl>",
        )
        .expect("transform");
    host.assert_no_errors();
    assert_eq!(
        dump(&doc.root().expect("root")),
        "(XamlValueWithManipulationNode (New clr!!XamlParserTests.ContentControl args=[]) \
         (Init ContentControl skipBeginInit=false (Group [\
         (Assign Control.StrProp [setter(String)] [(Text \"s\" preserve=true type=clr!!System.String)]), \
         (Assign ContentControl.Content [setter(Object)] [\
         (XamlValueWithManipulationNode (New clr!!XamlParserTests.Control args=[]) (Init Control skipBeginInit=false (Group [])))])])))"
    );
}

#[test]
fn top_down_initialization_defers_the_initializer() {
    let host = TestHost::new();
    let doc = host
        .transform_imperative("<TopDownParent xmlns='test'><TopDownParent.Child><TopDownChild StrProp='x'/></TopDownParent.Child></TopDownParent>")
        .expect("transform");
    host.assert_no_errors();
    assert_eq!(
        dump(&doc.root().expect("root")),
        "(XamlValueWithManipulationNode (New clr!!XamlParserTests.TopDownParent args=[]) \
         (Init TopDownParent skipBeginInit=false (Group [\
         (Assign TopDownParent.Child [setter(TopDownChild)] [(XamlValueNodeWithBeginInit (LocalInit (New clr!!XamlParserTests.TopDownChild args=[])))]), \
         (ManipulationImperative (ImperativeValueManipulation (Local TopDownChild) \
         (Init TopDownChild skipBeginInit=true \
         (Assign TopDownChild.StrProp [setter(String)] [(Text \"x\" preserve=true type=clr!!System.String)]))))])))"
    );
}

#[test]
fn deferred_content_is_wrapped() {
    let host = TestHost::new();
    let doc = host
        .transform_imperative("<DeferredHost xmlns='test'><DeferredHost.Template><Control/></DeferredHost.Template></DeferredHost>")
        .expect("transform");
    host.assert_no_errors();
    let root = doc.root().expect("root");
    assert_eq!(
        dump(&root),
        "(XamlValueWithManipulationNode (New clr!!XamlParserTests.DeferredHost args=[]) \
         (Init DeferredHost skipBeginInit=false \
         (Assign DeferredHost.Template [setter(Object)] [(Deferred (XamlValueWithManipulationNode \
         (DeferredRoot (New clr!!XamlParserTests.Control args=[])) (Init Control skipBeginInit=false (Group []))))])))"
    );

    struct FindDeferred(Option<Rc<XamlDeferredContentNode>>);
    impl IXamlAstVisitor for FindDeferred {
        fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> crate::XamlResult<Rc<dyn IXamlAstNode>> {
            if let Some(deferred) = node.cast::<XamlDeferredContentNode>() {
                self.0 = Some(deferred);
            }
            Ok(node)
        }
        fn push(&mut self, _node: Rc<dyn IXamlAstNode>) {}
        fn pop(&mut self) {}
    }
    let mut finder = FindDeferred(None);
    visit_node(&root, &mut finder).expect("visit");
    let deferred = finder.0.expect("deferred content node");
    assert_eq!(
        deferred
            .type_()
            .get_clr_type()
            .expect("clr type")
            .full_name(),
        "System.Func`2[System.IServiceProvider,System.Object]"
    );
    assert!(deferred
        .func_type()
        .equals(&*deferred.type_().get_clr_type().expect("clr type")));
}

#[test]
fn visitors_replace_children_in_place_and_track_parents() {
    // A node shared between two parents is visited (and replaced) through each parent.
    let li = XamlLineInfo::new(1, 1);
    let shared: Rc<dyn IXamlAstValueNode> = XamlAstTextNode::new(&li, "shared", false);
    let type_ = XamlAstXmlTypeReference::new(&li, Some("ns"), "T");
    let parent = XamlAstObjectNode::new(&li, type_.clone());
    parent.arguments.borrow_mut().push(shared.clone());
    parent.children.borrow_mut().push(shared.as_node());

    struct Replacer {
        depth: usize,
        max_depth: usize,
        seen: Vec<String>,
    }
    impl IXamlAstVisitor for Replacer {
        fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> crate::XamlResult<Rc<dyn IXamlAstNode>> {
            self.seen.push(node.type_name().to_string());
            if let Some(text) = node.cast::<XamlAstTextNode>() {
                if text.text() == "shared" {
                    return Ok(XamlAstTextNode::new(&*text, "replaced", false));
                }
            }
            Ok(node)
        }
        fn push(&mut self, _node: Rc<dyn IXamlAstNode>) {
            self.depth += 1;
            self.max_depth = self.max_depth.max(self.depth);
        }
        fn pop(&mut self) {
            self.depth -= 1;
        }
    }

    let mut replacer = Replacer {
        depth: 0,
        max_depth: 0,
        seen: Vec::new(),
    };
    let visited = visit_node(&parent.as_node(), &mut replacer).expect("visit");
    assert!(visited.same_node(&parent));
    assert_eq!(replacer.depth, 0);
    assert_eq!(replacer.max_depth, 3);
    assert_eq!(
        replacer.seen,
        vec![
            "XamlAstObjectNode",
            "XamlAstXmlTypeReference",
            "XamlAstTextNode",
            "XamlAstXmlTypeReference",
            "XamlAstTextNode",
            "XamlAstXmlTypeReference"
        ]
    );
    let argument = parent.arguments.borrow()[0].clone();
    let child = parent.children.borrow()[0].clone();
    assert!(
        !argument.same_node(&shared) && !child.same_node(&shared) && !argument.same_node(&child)
    );
    assert_eq!(
        argument.cast::<XamlAstTextNode>().expect("text").text(),
        "replaced"
    );
    assert_eq!(
        shared.cast::<XamlAstTextNode>().expect("text").text(),
        "shared"
    );

    // A visitor returning a node of the wrong kind is an invalid cast, not a panic.
    struct WrongKind;
    impl IXamlAstVisitor for WrongKind {
        fn visit(&mut self, node: Rc<dyn IXamlAstNode>) -> crate::XamlResult<Rc<dyn IXamlAstNode>> {
            if node
                .cast::<XamlAstXmlTypeReference>()
                .is_some_and(|t| t.name() == "T")
            {
                return Ok(XamlAstTextNode::new(&*node, "not a type", false));
            }
            Ok(node)
        }
        fn push(&mut self, _node: Rc<dyn IXamlAstNode>) {}
        fn pop(&mut self) {}
    }
    match visit_node(&parent.as_node(), &mut WrongKind) {
        Err(XamlError::Internal(e)) => {
            assert_eq!(e.type_name, "InvalidCastException");
            assert_eq!(
                e.message,
                "Unable to cast object of type 'XamlAstTextNode' to type 'IXamlAstTypeReference'."
            );
        }
        other => panic!("Expected an invalid cast, got {:?}", other.map(|_| ())),
    }
}
