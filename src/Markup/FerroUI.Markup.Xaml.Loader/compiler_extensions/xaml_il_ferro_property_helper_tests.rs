//! Tests of `xaml_il_ferro_property_helper.rs`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstTypeReference, IXamlAstValueNode, IXamlPropertySetter,
    XamlAstClrProperty, XamlAstClrTypeReference, XamlAstExtensions, XamlAstNamePropertyReference,
    XamlAstNodeExtensions, XamlDirectCallPropertySetter, XamlLineInfo,
};
use xamlx::exceptions::XamlError;
use xamlx::testing::FakeCustomAttribute;
use xamlx::transform::transformers::PropertyReferenceResolver;
use xamlx::transform::{AstTransformationContext, IXamlAstTransformer};
use xamlx::type_system::{IXamlField, IXamlType, XamlPseudoType};

use super::*;
use crate::testing::{create_test_framework, TestFramework};

fn li() -> XamlLineInfo {
    XamlLineInfo::new(4, 12)
}

fn type_reference(fw: &TestFramework, name: &str) -> Rc<dyn IXamlAstTypeReference> {
    XamlAstClrTypeReference::new(&li(), fw.t(name), false)
}

/// Resolves `name` on `type_name` the way the property reference resolver does.
fn clr_property(
    fw: &TestFramework,
    context: &AstTransformationContext,
    type_name: &str,
    name: &str,
) -> Rc<XamlAstClrProperty> {
    let reference = XamlAstNamePropertyReference::new(
        &li(),
        type_reference(fw, type_name),
        name,
        type_reference(fw, type_name),
    );
    PropertyReferenceResolver
        .transform(context, reference)
        .expect("resolved")
        .cast::<XamlAstClrProperty>()
        .expect("clr property")
}

fn field(fw: &TestFramework, type_name: &str, name: &str) -> Rc<dyn IXamlField> {
    fw.t(type_name)
        .get_all_fields()
        .into_iter()
        .find(|f| f.name() == name)
        .unwrap_or_else(|| panic!("{type_name}.{name} is not declared"))
}

fn setter_names(property: &XamlAstClrProperty) -> Vec<&'static str> {
    property.setters().iter().map(|s| s.type_name()).collect()
}

fn parameter_names(setter: &Rc<dyn IXamlPropertySetter>) -> Vec<String> {
    setter.parameters().iter().map(|p| p.full_name()).collect()
}

#[test]
fn resolves_the_registered_property_field_of_a_clr_property() {
    let fw = create_test_framework();
    let context = fw.create_context();

    let background = clr_property(&fw, &context, "FerroUI.Controls.Border", "Background");
    let found = XamlIlFerroPropertyHelper::try_get_ferro_property_field(&background)
        .expect("Border.BackgroundProperty");
    assert_eq!(found.name(), "BackgroundProperty");
    assert_eq!(found.declaring_type().full_name(), "FerroUI.Controls.Border");
    assert_eq!(
        found.field_type().full_name(),
        "FerroUI.StyledProperty`1[FerroUI.Media.IBrush]"
    );

    // The field is looked up on the declaring type only: Child is declared by Decorator.
    let child = clr_property(&fw, &context, "FerroUI.Controls.Border", "Child");
    assert_eq!(child.declaring_type().name(), "Decorator");
    assert_eq!(
        XamlIlFerroPropertyHelper::try_get_ferro_property_field(&child)
            .expect("Decorator.ChildProperty")
            .declaring_type()
            .name(),
        "Decorator"
    );

    // A plain CLR property has no field.
    let children = clr_property(&fw, &context, "FerroUI.Controls.Panel", "Children");
    assert!(XamlIlFerroPropertyHelper::try_get_ferro_property_field(&children).is_none());

    // The overload taking a type system property.
    let text = fw
        .t("FerroUI.Controls.TextBlock")
        .properties()
        .into_iter()
        .find(|p| p.name() == "Text")
        .expect("Text");
    assert_eq!(
        XamlIlFerroPropertyHelper::try_get_ferro_property_field_for_property(&*text)
            .expect("TextProperty")
            .name(),
        "TextProperty"
    );
    let classes = fw.types.styled_element_classes_property.clone();
    assert!(XamlIlFerroPropertyHelper::try_get_ferro_property_field_for_property(&*classes).is_none());
}

#[test]
fn provide_value_target_is_the_registered_property_or_the_clr_property() {
    let fw = create_test_framework();
    let context = fw.create_context();

    let background = clr_property(&fw, &context, "FerroUI.Controls.Border", "Background");
    match XamlIlFerroPropertyHelper::try_get_provide_value_target(&background) {
        Some(XamlIlProvideValueTargetProperty::FerroProperty(f)) => {
            assert_eq!(f.name(), "BackgroundProperty")
        }
        _ => panic!("expected the registered property field"),
    }

    let children = clr_property(&fw, &context, "FerroUI.Controls.Panel", "Children");
    match XamlIlFerroPropertyHelper::try_get_provide_value_target(&children) {
        Some(XamlIlProvideValueTargetProperty::ClrProperty(p)) => assert_eq!(p.name(), "Children"),
        _ => panic!("expected the CLR property"),
    }

    // Attached property accessors are no CLR property of the declaring type, but the field is found.
    let row = clr_property(&fw, &context, "FerroUI.Controls.Grid", "Row");
    assert!(matches!(
        XamlIlFerroPropertyHelper::try_get_provide_value_target(&row),
        Some(XamlIlProvideValueTargetProperty::FerroProperty(_))
    ));

    // An event has neither.
    let click = clr_property(&fw, &context, "FerroUI.Controls.Button", "Click");
    assert!(XamlIlFerroPropertyHelper::try_get_provide_value_target(&click).is_none());
}

#[test]
fn create_node_resolves_a_property_of_the_selector_type() {
    let fw = create_test_framework();
    let context = fw.create_context();
    let node = XamlIlFerroPropertyHelper::create_node(
        &context,
        "Background",
        type_reference(&fw, "FerroUI.Controls.Border"),
        &li(),
    )
    .expect("node");
    assert!(!node.is_xaml_il_ferro_class_property_node());
    assert_eq!(node.ferro_property_type().full_name(), "FerroUI.Media.IBrush");
    assert_eq!(
        node.type_().get_clr_type().expect("clr type").full_name(),
        "FerroUI.FerroProperty"
    );
    assert_eq!((node.line(), node.position()), (4, 12));

    let as_node: Rc<dyn IXamlAstNode> = node.clone();
    let property_node = as_node.cast::<XamlIlFerroPropertyNode>().expect("property node");
    assert_eq!(property_node.property.name(), "Background");
    assert_eq!(
        property_node
            .resolve_ferro_property_field()
            .expect("field")
            .name(),
        "BackgroundProperty"
    );
    // The interface cast used by the transformers.
    assert!(as_node.cast::<dyn IXamlIlFerroPropertyNode>().is_some());
    let value: Rc<dyn IXamlAstValueNode> = node;
    assert!(value.is::<dyn IXamlIlFerroPropertyNode>());
}

#[test]
fn create_node_uses_the_setter_parameter_when_there_is_no_getter() {
    let fw = create_test_framework();
    let write_only = fw.controls.define_class("Tests", "WriteOnly");
    write_only.set_base_type(fw.t("FerroUI.Controls.Control"));
    write_only.add_property_with("Secret", fw.t("System.Int32"), false, true, false);
    let context = fw.create_context();
    let node = XamlIlFerroPropertyHelper::create_node(
        &context,
        "Secret",
        type_reference(&fw, "Tests.WriteOnly"),
        &li(),
    )
    .expect("node");
    assert_eq!(node.ferro_property_type().full_name(), "System.Int32");
    // There is no SecretProperty field: emitting the node fails.
    let as_node: Rc<dyn IXamlAstNode> = node;
    let error = as_node
        .cast::<XamlIlFerroPropertyNode>()
        .expect("property node")
        .resolve_ferro_property_field()
        .err()
        .expect("no field");
    assert!(matches!(error, XamlError::Load(_)));
    assert_eq!(error.message(), "Secret is not a FerroProperty Line 4, position 12.");
}

#[test]
fn create_node_returns_a_stub_for_an_unknown_property() {
    let fw = create_test_framework();
    let context = fw.create_context();
    let node = XamlIlFerroPropertyHelper::create_node(
        &context,
        "Nope",
        type_reference(&fw, "FerroUI.Controls.Border"),
        &li(),
    )
    .expect("the error is reported as a diagnostic");
    assert!(XamlPseudoType::is_unknown(&*node.ferro_property_type()));
    let diagnostics = fw.reported_diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "FRN2000");
    assert!(
        diagnostics[0]
            .title
            .starts_with("Unable to resolve suitable regular or attached property Nope on type"),
        "{}",
        diagnostics[0].title
    );
}

#[test]
fn create_node_resolves_an_owner_qualified_property_through_its_field() {
    let fw = create_test_framework();
    let context = fw.create_context();
    let selector = type_reference(&fw, "FerroUI.Controls.Border");

    for name in ["Grid.Row", "(Grid.Row)"] {
        let node = XamlIlFerroPropertyHelper::create_node(&context, name, selector.clone(), &li())
            .expect("node");
        assert_eq!(node.ferro_property_type().full_name(), "System.Int32");
        assert_eq!(
            node.type_().get_clr_type().expect("clr type").full_name(),
            "FerroUI.AttachedProperty`1[System.Int32]"
        );
        let as_node: Rc<dyn IXamlAstNode> = node;
        let field_node = as_node
            .cast::<XamlIlFerroPropertyFieldNode>()
            .expect("field node");
        assert_eq!(field_node.field().name(), "RowProperty");
    }

    // Inherited fields are found, with or without an xmlns prefix.
    let context = fw.create_context();
    context
        .namespace_aliases
        .borrow_mut()
        .insert("c".to_string(), crate::compiler_extensions::transformers::FERRO_XML_NAMESPACE.to_string());
    let node =
        XamlIlFerroPropertyHelper::create_node(&context, "c:Border.Padding", selector.clone(), &li())
            .expect("node");
    assert_eq!(node.ferro_property_type().full_name(), "FerroUI.Thickness");

    // Errors.
    let error = XamlIlFerroPropertyHelper::create_node(&context, "Border.Nope", selector.clone(), &li())
        .err()
        .expect("no such field");
    assert!(matches!(error, XamlError::Transform(_)));
    assert_eq!(
        error.message(),
        "Unable to find NopeProperty field on type FerroUI.Controls.Border,FerroUI.Controls Line 4, position 12."
    );
    assert!(
        XamlIlFerroPropertyHelper::create_node(&context, "Nope.Row", selector.clone(), &li()).is_err()
    );
    let error = XamlIlFerroPropertyHelper::create_node(&context, "(Border", selector, &li())
        .err()
        .expect("malformed");
    assert_eq!(error.type_name(), "ExpressionParseException");
}

#[test]
fn create_node_resolves_a_style_class() {
    let fw = create_test_framework();
    let context = fw.create_context();
    let selector = type_reference(&fw, "FerroUI.Controls.Border");
    for name in ["Classes.active", "(classes.active)"] {
        let node = XamlIlFerroPropertyHelper::create_node(&context, name, selector.clone(), &li())
            .expect("node");
        assert!(node.is_xaml_il_ferro_class_property_node());
        assert_eq!(node.ferro_property_type().full_name(), "System.Boolean");
        assert_eq!(
            node.type_().get_clr_type().expect("clr type").full_name(),
            "FerroUI.FerroProperty`1[System.Boolean]"
        );
        let as_node: Rc<dyn IXamlAstNode> = node;
        let class_property = as_node
            .cast::<XamlIlFerroClassProperty>()
            .expect("class property");
        assert_eq!(class_property.class_name(), "active");
        assert_eq!(class_property.method().name(), "GetClassProperty");
        assert!(class_property.return_type().equals(
            &*class_property
                .type_()
                .get_clr_type()
                .expect("clr type")
        ));
        assert_eq!(
            class_property
                .parameters()
                .iter()
                .map(|p| p.full_name())
                .collect::<Vec<_>>(),
            ["System.String"]
        );
        // The property node base: named after the class, declared by Classes, no accessors.
        assert_eq!(class_property.base.name(), "active");
        assert_eq!(class_property.base.declaring_type().name(), "Classes");
        assert!(class_property.base.getter().is_none());
        assert!(class_property.base.setters().is_empty());
        assert!(as_node.clone().into_property_reference().is_some());
        assert!(as_node.into_value_node().is_some());
    }
}

#[test]
fn ferro_property_type_is_the_argument_of_the_typed_property_base() {
    let fw = create_test_framework();
    let types = &fw.types;
    for (type_name, field_name, expected) in [
        ("FerroUI.Controls.Border", "BackgroundProperty", "FerroUI.Media.IBrush"),
        ("FerroUI.Controls.Grid", "RowProperty", "System.Int32"),
        ("FerroUI.StyledElement", "NameProperty", "System.String"),
    ] {
        let found = field(&fw, type_name, field_name);
        assert_eq!(
            XamlIlFerroPropertyHelper::get_ferro_property_type(&*found, types, &li())
                .expect("typed property")
                .full_name(),
            expected
        );
    }

    // Not a typed registered property.
    let click_event = field(&fw, "FerroUI.Controls.Button", "ClickEvent");
    let error = XamlIlFerroPropertyHelper::get_ferro_property_type(&*click_event, types, &li())
        .err()
        .expect("not a property");
    assert!(matches!(error, XamlError::Transform(_)));
    assert!(error.message().starts_with(
        "ClickEvent's type FerroUI.Interactivity.RoutedEvent`1[FerroUI.Interactivity.RoutedEventArgs] doesn't inherit from  FerroProperty<T>"
    ));
    assert!(XamlIlFerroPropertyFieldNode::new(types, &li(), click_event).is_err());
}

#[test]
fn ferro_property_of_a_styled_property_gets_all_setters() {
    let fw = create_test_framework();
    let context = fw.create_context();
    let original = clr_property(&fw, &context, "FerroUI.Controls.Border", "Background");
    assert_eq!(setter_names(&original), ["XamlDirectCallPropertySetter"]);
    let background_field = field(&fw, "FerroUI.Controls.Border", "BackgroundProperty");

    let property = XamlIlFerroProperty::new(&original, background_field.clone(), &fw.types)
        .expect("registered property node");
    assert!(!Rc::ptr_eq(&property, &original));
    assert_eq!(property.name(), "Background");
    assert_eq!(property.declaring_type().name(), "Border");
    assert!(property.getter().is_some());
    assert_eq!((property.line(), property.position()), (4, 12));

    assert_eq!(
        setter_names(&property),
        [
            "UnsetValueSetter",
            "SetValueWithPrioritySetter",
            "BindingWithPrioritySetter",
            "BindingSetter",
            "XamlDirectCallPropertySetter"
        ]
    );
    let setters = property.setters();
    assert_eq!(parameter_names(&setters[0]), ["FerroUI.UnsetValueType"]);
    assert_eq!(
        parameter_names(&setters[1]),
        ["FerroUI.Data.BindingPriority", "FerroUI.Media.IBrush"]
    );
    assert_eq!(
        parameter_names(&setters[2]),
        ["FerroUI.Data.BindingPriority", "FerroUI.Data.BindingBase"]
    );
    assert_eq!(parameter_names(&setters[3]), ["FerroUI.Data.BindingBase"]);
    assert_eq!(parameter_names(&setters[4]), ["FerroUI.Media.IBrush"]);
    for setter in &setters[..4] {
        assert_eq!(setter.target_type().name(), "Border");
        assert!(setter.custom_attributes().is_empty());
        assert!(!setter.binder_parameters().allow_multiple.get());
        assert!(setter.binder_parameters().allow_attribute_syntax.get());
    }
    // Null: only the value setter of a reference-typed property accepts it.
    let allows_null: Vec<(bool, bool)> = setters[..4]
        .iter()
        .map(|s| {
            let b = s.binder_parameters();
            (b.allow_x_null.get(), b.allow_runtime_null.get())
        })
        .collect();
    assert_eq!(
        allows_null,
        [(false, false), (true, true), (false, false), (false, false)]
    );

    // The data the back ends need.
    let unset = setters[0]
        .as_any()
        .downcast_ref::<UnsetValueSetter>()
        .expect("unset value setter");
    assert!(unset.ferro_property().equals(&*background_field));
    assert_eq!(unset.unset_value_field().expect("UnsetValue").name(), "UnsetValue");
    assert!(Rc::ptr_eq(&unset.base().types, &fw.types));
    let set_value = setters[1]
        .as_any()
        .downcast_ref::<SetValueWithPrioritySetter>()
        .expect("set value setter");
    let method = set_value
        .set_styled_property_value_method()
        .expect("SetValue<IBrush>");
    assert_eq!(
        method
            .parameters()
            .iter()
            .map(|p| p.full_name())
            .collect::<Vec<_>>(),
        [
            "FerroUI.StyledProperty`1[FerroUI.Media.IBrush]",
            "FerroUI.Media.IBrush",
            "FerroUI.Data.BindingPriority"
        ]
    );

    // The node is recognisable as a registered property.
    let extension = XamlIlFerroProperty::from_clr_property(&property).expect("XamlIlFerroProperty");
    assert!(extension.ferro_property().equals(&*background_field));
    assert!(as_xaml_il_ferro_property(&property)
        .expect("IXamlIlFerroProperty")
        .ferro_property()
        .equals(&*background_field));
    assert!(XamlIlFerroProperty::from_clr_property(&original).is_none());
    assert!(as_xaml_il_ferro_property(&original).is_none());
    assert!(XamlIlFerroPropertyHelper::try_get_ferro_property_field(&property)
        .expect("field")
        .equals(&*background_field));
}

#[test]
fn ferro_property_of_a_value_typed_styled_property_rejects_null() {
    let fw = create_test_framework();
    let context = fw.create_context();
    let original = clr_property(&fw, &context, "FerroUI.Controls.Border", "BorderThickness");
    let property = XamlIlFerroProperty::new(
        &original,
        field(&fw, "FerroUI.Controls.Border", "BorderThicknessProperty"),
        &fw.types,
    )
    .expect("registered property node");
    let setters = property.setters();
    assert_eq!(setters[1].type_name(), "SetValueWithPrioritySetter");
    assert!(!setters[1].binder_parameters().allow_x_null.get());
    assert!(!setters[1].binder_parameters().allow_runtime_null.get());
}

#[test]
fn ferro_property_of_a_direct_property_has_no_priority_setters() {
    let fw = create_test_framework();
    let context = fw.create_context();
    let original = clr_property(&fw, &context, "FerroUI.Controls.Border", "Name");
    let property = XamlIlFerroProperty::new(
        &original,
        field(&fw, "FerroUI.StyledElement", "NameProperty"),
        &fw.types,
    )
    .expect("registered property node");
    assert_eq!(
        setter_names(&property),
        ["UnsetValueSetter", "BindingSetter", "XamlDirectCallPropertySetter"]
    );
}

#[test]
fn ferro_property_of_an_attached_property_gets_the_priority_setters() {
    let fw = create_test_framework();
    let context = fw.create_context();
    let original = clr_property(&fw, &context, "FerroUI.Controls.Grid", "Row");
    let property = XamlIlFerroProperty::new(
        &original,
        field(&fw, "FerroUI.Controls.Grid", "RowProperty"),
        &fw.types,
    )
    .expect("registered property node");
    assert_eq!(
        setter_names(&property),
        [
            "UnsetValueSetter",
            "SetValueWithPrioritySetter",
            "BindingWithPrioritySetter",
            "BindingSetter",
            "XamlDirectCallPropertySetter"
        ]
    );
    assert_eq!(
        parameter_names(&property.setters()[1]),
        ["FerroUI.Data.BindingPriority", "System.Int32"]
    );
}

#[test]
fn assign_binding_properties_get_no_binding_setters() {
    let fw = create_test_framework();
    // A styled property whose bindings are assigned, not applied.
    let host = fw.fake_type("FerroUI.Controls.Border");
    host.add_property("Source", fw.t("FerroUI.Data.BindingBase"))
        .add_attribute(FakeCustomAttribute::new(
            fw.types.assign_binding_attribute.clone(),
            vec![],
        ));
    let styled = fw
        .t("FerroUI.StyledProperty`1")
        .make_generic_type(&[fw.t("FerroUI.Data.BindingBase")])
        .expect("StyledProperty<BindingBase>");
    let source_field: Rc<dyn IXamlField> = host.add_field("SourceProperty", styled, true, None);

    let context = fw.create_context();
    let original = clr_property(&fw, &context, "FerroUI.Controls.Border", "Source");
    let property =
        XamlIlFerroProperty::new(&original, source_field, &fw.types).expect("registered property node");
    assert_eq!(
        setter_names(&property),
        ["UnsetValueSetter", "SetValueWithPrioritySetter", "XamlDirectCallPropertySetter"]
    );
    assert_eq!(property.custom_attributes().len(), 1);
}

#[test]
fn custom_setters_compare_by_class_and_property() {
    let fw = create_test_framework();
    let border = fw.t("FerroUI.Controls.Border");
    let background = field(&fw, "FerroUI.Controls.Border", "BackgroundProperty");
    let border_brush = field(&fw, "FerroUI.Controls.Border", "BorderBrushProperty");

    let a: Rc<dyn IXamlPropertySetter> =
        BindingSetter::new(&fw.types, border.clone(), background.clone());
    let same: Rc<dyn IXamlPropertySetter> =
        BindingSetter::new(&fw.types, border.clone(), background.clone());
    let other_property: Rc<dyn IXamlPropertySetter> =
        BindingSetter::new(&fw.types, border.clone(), border_brush);
    let other_class: Rc<dyn IXamlPropertySetter> =
        BindingWithPrioritySetter::new(&fw.types, border.clone(), background.clone());
    let unset: Rc<dyn IXamlPropertySetter> =
        UnsetValueSetter::new(&fw.types, border, background);

    assert!(a.equals(&*a));
    assert!(a.equals(&*same));
    assert!(!a.equals(&*other_property));
    assert!(!a.equals(&*other_class));
    assert!(!other_class.equals(&*a));
    assert!(!a.equals(&*unset));

    let direct: Rc<dyn IXamlPropertySetter> = XamlDirectCallPropertySetter::new(
        fw.t("FerroUI.Controls.Border")
            .find_method(|m| m.name() == "set_Background")
            .expect("set_Background"),
    )
    .expect("direct setter");
    assert!(!a.equals(&*direct));
}

#[test]
fn property_node_requires_an_accessor() {
    let fw = create_test_framework();
    let property = XamlAstClrProperty::new(&li(), "Ghost", fw.t("FerroUI.Controls.Border"), None);
    let error = XamlIlFerroPropertyNode::new(&li(), fw.types.ferro_property.clone(), property)
        .err()
        .expect("no accessor");
    assert_eq!(
        error.message(),
        "Unable to resolve \"Border.Ghost\" property type. There is no setter or getter."
    );
}

#[test]
fn class_names_compare_ignoring_case() {
    assert!(ordinal_ignore_case_equals("classes", "Classes"));
    assert!(ordinal_ignore_case_equals("CLASSES", "Classes"));
    assert!(!ordinal_ignore_case_equals("Class", "Classes"));
    assert!(is_null_or_white_space(None));
    assert!(is_null_or_white_space(Some(" \t")));
    assert!(!is_null_or_white_space(Some("x")));
}

fn _type_helper(_: &Rc<dyn IXamlType>) {}
