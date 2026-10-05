//! Tests of `FerroXamlIlSetterTransformer` and `FerroXamlIlSetterTargetTypeMetadataTransformer`.
//! The XAML shapes come from the upstream `SetterTests` and `StyleTests`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, IXamlPropertySetter, XamlAstClrProperty, XamlAstExtensions,
    XamlAstNodeExtensions, XamlAstObjectNode, XamlAstPropertyReferenceExtensions, XamlAstTextNode,
    XamlAstXamlPropertyValueNode, XamlAstXmlDirective,
};
use xamlx::transform::IXamlAstTransformer;

use super::*;
use crate::compiler_extensions::{
    FerroXamlDiagnosticCodes, IXamlIlFerroPropertyNode, XamlIlFerroClassProperty,
    XamlIlFerroPropertyFieldNode, XamlIlFerroPropertyNode,
};
use crate::testing::styles::{create_styles_test_framework, find_nodes, run_transformers, xmlns};
use crate::testing::TestFramework;

fn run(fw: &TestFramework, xaml: &str) -> Rc<dyn IXamlAstNode> {
    let pipeline: Vec<Box<dyn IXamlAstTransformer>> = vec![
        Box::new(FerroXamlIlSelectorTransformer),
        Box::new(FerroXamlIlSetterTargetTypeMetadataTransformer),
        Box::new(FerroXamlIlSetterTransformer),
        Box::new(FerroXamlIlSetterTransformer),
    ];
    run_transformers(fw, xaml, pipeline).expect("errors are not fatal in the fixture")
}

fn styled(fw: &TestFramework, selector: &str, setter: &str) -> Rc<dyn IXamlAstNode> {
    run(fw, &format!("<Style {} Selector='{selector}'>{setter}</Style>", xmlns()))
}

fn setter_node(root: &Rc<dyn IXamlAstNode>) -> Rc<XamlAstObjectNode> {
    find_nodes::<XamlAstObjectNode>(root)
        .into_iter()
        .find(|o| o.type_().get_clr_type().expect("type").name() == "Setter")
        .expect("a setter")
}

fn property_value(setter: &XamlAstObjectNode, name: &str) -> Rc<XamlAstXamlPropertyValueNode> {
    setter
        .children
        .borrow()
        .iter()
        .filter_map(|c| c.cast::<XamlAstXamlPropertyValueNode>())
        .find(|p| p.property().get_clr_property().expect("clr").name() == name)
        .unwrap_or_else(|| panic!("no {name} property"))
}

fn setter_parameters(property: &XamlAstClrProperty) -> Vec<(String, bool)> {
    property
        .setters()
        .iter()
        .map(|s| {
            assert!(s.as_any().is::<XamlIlDirectCallPropertySetter>());
            assert_eq!(s.target_type().name(), "Setter");
            assert_eq!(
                s.binder_parameters().allow_x_null.get(),
                s.binder_parameters().allow_runtime_null.get()
            );
            (s.parameters()[0].name(), s.binder_parameters().allow_x_null.get())
        })
        .collect()
}

fn check_error(xaml: &str, code: &str, message: &str) {
    let fw = create_styles_test_framework();
    check_error_in(&fw, xaml, code, message);
}

fn check_error_in(fw: &TestFramework, xaml: &str, code: &str, message: &str) {
    let _ = run(fw, xaml);
    let diagnostics = fw.reported_diagnostics();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, code, "{diagnostics:?}");
    assert!(diagnostics[0].title.starts_with(message), "{:?}", diagnostics[0].title);
}

#[test]
fn property_is_resolved_against_the_style_target_type_and_value_is_retyped() {
    let fw = create_styles_test_framework();
    let root = styled(&fw, "Button", "<Setter Property='Width' Value='100'/>");
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    let setter = setter_node(&root);

    let property = property_value(&setter, "Property");
    let values = property.values.borrow().clone();
    assert_eq!(values.len(), 1);
    let ferro_property = values[0].cast::<XamlIlFerroPropertyNode>().expect("property node");
    assert_eq!(ferro_property.property.name(), "Width");
    assert_eq!(ferro_property.property.declaring_type().name(), "Layoutable");
    assert_eq!(ferro_property.ferro_property_type().full_name(), "System.Double");

    let value = property_value(&setter, "Value");
    let value_property = value.property().get_clr_property().expect("clr");
    assert!(SetterValueProperty::from_clr_property(&value_property).is_some());
    assert_eq!(value_property.declaring_type().name(), "Setter");
    assert_eq!(value_property.getter().expect("getter").name(), "get_Value");
    assert_eq!(
        setter_parameters(&value_property),
        [
            ("BindingBase".to_string(), false),
            ("UnsetValueType".to_string(), false),
            ("Double".to_string(), false)
        ]
    );
    // The value itself is left for the standard conversion
    assert_eq!(value.values.borrow()[0].cast::<XamlAstTextNode>().expect("text").text(), "100");

    // Setters are equal when method and type are equal
    let setters = value_property.setters();
    let direct = |s: &Rc<dyn IXamlPropertySetter>| {
        let s = s.as_any().downcast_ref::<XamlIlDirectCallPropertySetter>().expect("setter");
        XamlIlDirectCallPropertySetter::new(s.method.clone(), s.type_.clone(), false).expect("copy")
    };
    assert!(setters[0].equals(&*direct(&setters[0])));
    assert!(!setters[0].equals(&*setters[1]));
    assert_eq!(setters[2].custom_attributes().len(), 0);
}

#[test]
fn reference_typed_property_allows_null() {
    let fw = create_styles_test_framework();
    let root = styled(&fw, "TextBlock", "<Setter Property='Text' Value='abc'/>");
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    let value = property_value(&setter_node(&root), "Value");
    let value_property = value.property().get_clr_property().expect("clr");
    assert_eq!(setter_parameters(&value_property)[2], ("String".to_string(), true));
}

#[test]
fn text_content_becomes_the_value_property() {
    let fw = create_styles_test_framework();
    let root = styled(&fw, "Button", "<Setter Property='Width'>100</Setter>");
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    let setter = setter_node(&root);
    assert!(setter.children.borrow().iter().all(|c| c.is::<XamlAstXamlPropertyValueNode>()));
    let value = property_value(&setter, "Value");
    assert!(!value.is_attribute_syntax);
    let value_property = value.property().get_clr_property().expect("clr");
    assert!(SetterValueProperty::from_clr_property(&value_property).is_some());
    assert_eq!(value.values.borrow().len(), 1);
}

#[test]
fn object_values_keep_the_plain_value_property() {
    let fw = create_styles_test_framework();
    let root = styled(
        &fw,
        "Button",
        "<Setter Property='Tag'><Setter.Value><Border/></Setter.Value></Setter>",
    );
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    let value = property_value(&setter_node(&root), "Value");
    let value_property = value.property().get_clr_property().expect("clr");
    assert!(SetterValueProperty::from_clr_property(&value_property).is_none());
}

#[test]
fn attached_and_class_properties() {
    let fw = create_styles_test_framework();
    let root = styled(&fw, "Button", "<Setter Property='Grid.Row' Value='1'/>");
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    let property = property_value(&setter_node(&root), "Property");
    let field = property.values.borrow()[0].cast::<XamlIlFerroPropertyFieldNode>().expect("field node");
    assert_eq!(field.field().name(), "RowProperty");
    assert_eq!(field.ferro_property_type().name(), "Int32");

    // A style class may be set from a style with a plain type selector...
    let fw = create_styles_test_framework();
    let root = styled(&fw, "Button", "<Setter Property='(Classes.foo)' Value='true'/>");
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    let property = property_value(&setter_node(&root), "Property");
    let class = property.values.borrow()[0].cast::<XamlIlFerroClassProperty>().expect("class node");
    assert_eq!(class.class_name(), "foo");

    // ...but not from a style with any other activator
    check_error(
        &format!(
            "<Style {} Selector='Button.bar'><Setter Property='(Classes.foo)' Value='true'/></Style>",
            xmlns()
        ),
        FerroXamlDiagnosticCodes::STYLE_TRANSFORM_ERROR,
        "Cannot set Classes Binding property '(Classes.foo)' because the style has an activator.",
    );
}

#[test]
fn setter_errors() {
    let style = |selector: &str, setter: &str| {
        format!("<Style {} Selector='{selector}'>{setter}</Style>", xmlns())
    };
    check_error(
        &format!("<Setter {} Property='Width' Value='1'/>", xmlns()),
        FerroXamlDiagnosticCodes::STYLE_TRANSFORM_ERROR,
        "Could not determine target type of Setter",
    );
    check_error(
        &style(".foo", "<Setter Property='Width' Value='1'/>"),
        FerroXamlDiagnosticCodes::STYLE_TRANSFORM_ERROR,
        "Can not find parent Style Selector or ControlTemplate TargetType. If setter is not part of the style, you can set x:SetterTargetType directive on its parent.",
    );
    check_error(
        &style("Button", "<Setter Value='1'/>"),
        FerroXamlDiagnosticCodes::STYLE_TRANSFORM_ERROR,
        "Setter without a property or property path is not valid",
    );
    check_error(
        &style("Button", "<Setter><Setter.Property><Button/></Setter.Property></Setter>"),
        FerroXamlDiagnosticCodes::STYLE_TRANSFORM_ERROR,
        "Setter.Property must be a string.",
    );
    check_error(
        &style("Button", "<Setter Property='Template' Value='abc'/>"),
        FerroXamlDiagnosticCodes::STYLE_TRANSFORM_ERROR,
        "Unable to convert property value to FerroUI.Controls:FerroUI.Controls.Templates.IControlTemplate",
    );

    let fw = create_styles_test_framework();
    fw.fake_type("FerroUI.Styling.Setter")
        .add_property("PropertyPath", fw.t("System.String"));
    check_error_in(
        &fw,
        &style("Button", "<Setter PropertyPath='Width' Value='1'/>"),
        FerroXamlDiagnosticCodes::STYLE_TRANSFORM_ERROR,
        "Unable to get the property path property type",
    );
}

#[test]
fn unconvertible_text_content_is_left_alone() {
    // Only an explicit `Value` property with unparsable text is an error
    let fw = create_styles_test_framework();
    let root = styled(&fw, "Button", "<Setter Property='Template'>abc</Setter>");
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    let setter = setter_node(&root);
    assert!(setter.children.borrow().iter().any(|c| c.is::<XamlAstTextNode>()));
}

#[test]
fn template_content_gets_a_control_template_scope() {
    let fw = create_styles_test_framework();
    let root = styled(&fw, "Button", "<Setter Property='Tag'><Template><Border/></Template></Setter>");
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    let setter = setter_node(&root);
    let scope = setter
        .children
        .borrow()
        .iter()
        .find_map(|c| c.cast::<FerroXamlIlTargetTypeMetadataNode>())
        .expect("scope");
    assert_eq!(scope.scope_type, ScopeTypes::ControlTemplate);
    assert_eq!(scope.target_type().get_clr_type().expect("type").name(), "Button");
    assert_eq!(scope.value().type_().get_clr_type().expect("type").name(), "Template");

    // Not when the property itself is a template of a control
    let fw = create_styles_test_framework();
    let template_of_control = fw
        .t("FerroUI.Controls.ITemplate`1")
        .make_generic_type(&[fw.t("FerroUI.Controls.Control")])
        .expect("ITemplate<Control>");
    fw.fake_type("FerroUI.Input.InputElement")
        .add_property("FocusAdorner", template_of_control);
    let root = styled(
        &fw,
        "Button",
        "<Setter Property='FocusAdorner'><Template><Border/></Template></Setter>",
    );
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    assert!(setter_node(&root)
        .children
        .borrow()
        .iter()
        .all(|c| !c.is::<FerroXamlIlTargetTypeMetadataNode>()));

    // Nor for other objects
    let fw = create_styles_test_framework();
    let root = styled(&fw, "Button", "<Setter Property='Tag'><Border/></Setter>");
    assert!(setter_node(&root).children.borrow().iter().any(|c| c.is::<XamlAstObjectNode>()));
}

#[test]
fn setter_target_type_directive_creates_a_style_scope() {
    for target in ["Button", "{x:Type Button}"] {
        let fw = create_styles_test_framework();
        let root = run(
            &fw,
            &format!(
                "<Border {} x:SetterTargetType='{target}'><Setter Property='Content' Value='x'/></Border>",
                xmlns()
            ),
        );
        assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
        let scope = root.cast::<FerroXamlIlTargetTypeMetadataNode>().expect("scope");
        assert_eq!(scope.scope_type, ScopeTypes::Style);
        assert_eq!(scope.target_type().get_clr_type().expect("type").name(), "Button");
        // The directive is removed and the setter resolved against the directive's type
        assert!(find_nodes::<XamlAstXmlDirective>(&root).is_empty());
        let property = property_value(&setter_node(&root), "Property");
        let node = property.values.borrow()[0].cast::<XamlIlFerroPropertyNode>().expect("node");
        assert_eq!(node.property.declaring_type().name(), "ContentControl");
    }

    check_error(
        &format!("<Border {} x:SetterTargetType='Missing'/>", xmlns()),
        FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
        "Unable to resolve type Missing from namespace",
    );

    // Objects without the directive are not touched
    let fw = create_styles_test_framework();
    let root = run(&fw, &format!("<Border {} x:Name='a'/>", xmlns()));
    assert!(root.is::<XamlAstObjectNode>());
    let value: Option<Rc<dyn IXamlAstValueNode>> = root.cast::<dyn IXamlAstValueNode>();
    assert!(value.is_some());
}

#[test]
fn style_exception_is_a_tagged_transform_exception() {
    let line = xamlx::ast::XamlLineInfo::new(2, 3);
    let e = XamlStyleTransformException::new("bad", &line, None);
    assert!(XamlStyleTransformException::is(&e));
    assert!(!XamlSelectorsTransformException::is(&e));
    assert_eq!(e.type_name(), "XamlStyleTransformException");
    let e = XamlSelectorsTransformException::new("bad", &line, Some(e));
    assert!(XamlSelectorsTransformException::is(&e));
    assert_eq!(e.message(), "bad Line 2, position 3.");
    assert!(e.inner_exception().is_some());
}
