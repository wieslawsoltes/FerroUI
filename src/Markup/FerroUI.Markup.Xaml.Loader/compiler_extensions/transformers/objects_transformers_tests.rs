//! Tests of the object, property and resource transformers: documents are parsed with the
//! xamlx parser and run through the XamlX standard transformers plus the transformers of this
//! area (`testing::objects::objects_pipeline`), cut short where a test needs an intermediate
//! tree.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, XamlAstClrProperty, XamlAstExtensions,
    XamlAstNamePropertyReference, XamlAstNewClrObjectNode, XamlAstNodeExtensions,
    XamlAstObjectNode, XamlAstTextNode, XamlAstXamlPropertyValueNode, XamlAstXmlDirective,
    XamlConstantNode, XamlDeferredContentNode, XamlLoadMethodDelegateNode,
    XamlManipulationGroupNode, XamlMarkupExtensionNode, XamlPropertyAssignmentNode,
    XamlValueWithManipulationNode,
};
use xamlx::diagnostics::XamlDiagnosticSeverity;
use xamlx::extensions::query_node_interface;
use xamlx::transform::IXamlAstTransformer;
use xamlx::type_system::{IXamlMember, IXamlMethod, IXamlType, XamlValue};

use super::*;
use crate::compiler_extensions::{
    as_xaml_il_ferro_property, FerroXamlDiagnosticCodes, FerroXamlIlCompiler,
    IOptionsMarkupExtensionNode, XamlIlFerroProperty,
};
use crate::testing::objects::*;
use crate::testing::TestFramework;

fn transform(fw: &TestFramework, body: &str) -> Rc<dyn IXamlAstNode> {
    match transform_objects(fw, body) {
        Ok(root) => root,
        Err(e) => panic!("The transformation failed: {e}"),
    }
}

/// The objects pipeline up to and including the transformer named `last`.
fn pipeline_until(fw: &TestFramework, last: &str) -> FerroXamlIlCompiler {
    let mut compiler = objects_pipeline(fw);
    let index = compiler
        .transformers
        .iter()
        .position(|t| t.transformer_name() == last)
        .unwrap_or_else(|| panic!("{last} is not part of the pipeline"));
    compiler.transformers.truncate(index + 1);
    compiler.simplification_transformers.clear();
    compiler
}

fn transform_until(fw: &TestFramework, last: &str, xaml: &str) -> Rc<dyn IXamlAstNode> {
    match transform_with(&pipeline_until(fw, last), xaml) {
        Ok(root) => root,
        Err(e) => panic!("The transformation failed: {e}"),
    }
}

fn text_of(value: &Rc<dyn IXamlAstValueNode>) -> String {
    value
        .cast::<XamlAstTextNode>()
        .unwrap_or_else(|| panic!("{} is not a text node", value.type_name()))
        .text()
}

fn setter_names(assignment: &XamlPropertyAssignmentNode) -> Vec<&'static str> {
    assignment
        .possible_setters
        .borrow()
        .iter()
        .map(|s| s.type_name())
        .collect()
}

fn no_errors(fw: &TestFramework) {
    let errors: Vec<String> = fw
        .reported_diagnostics()
        .iter()
        .filter(|d| d.severity >= XamlDiagnosticSeverity::Error)
        .map(|d| d.title.clone())
        .collect();
    assert!(errors.is_empty(), "{errors:?}");
}

fn error_titles(fw: &TestFramework) -> Vec<String> {
    fw.reported_diagnostics()
        .iter()
        .filter(|d| d.severity >= XamlDiagnosticSeverity::Error)
        .map(|d| d.title.clone())
        .collect()
}

#[test]
fn fixture_namespace_matches() {
    assert_xmlns_matches();
}

// XNameTransformer, IgnoredDirectivesTransformer

#[test]
fn x_name_becomes_a_name_property_value() {
    let fw = create_objects_test_framework();
    let root = transform_until(
        &fw,
        "XNameTransformer",
        &format!("<Border {XMLNS} x:Name='b' x:Key='k'><TextBlock x:Name='t'/></Border>"),
    );
    let object = root.cast::<XamlAstObjectNode>().expect("object");
    let children = object.children.borrow().clone();
    let name = children[0]
        .cast::<XamlAstXamlPropertyValueNode>()
        .expect("x:Name was replaced");
    assert!(name.is_attribute_syntax);
    let reference = name
        .property()
        .cast::<XamlAstNamePropertyReference>()
        .expect("name reference");
    assert_eq!(reference.name(), "Name");
    let object_type = object.type_.borrow().clone();
    assert!(reference.declaring_type.borrow().same_node(&object_type));
    assert!(reference.target_type.borrow().same_node(&object_type));
    assert_eq!(text_of(&name.values.borrow()[0]), "b");
    // Other directives are left alone, nested objects are handled too.
    assert!(children[1].is::<XamlAstXmlDirective>());
    let nested = children[2].cast::<XamlAstObjectNode>().expect("nested");
    assert!(nested.children.borrow()[0].is::<XamlAstXamlPropertyValueNode>());
}

#[test]
fn ignored_directives_are_removed() {
    let fw = create_objects_test_framework();
    let root = transform_until(
        &fw,
        "IgnoredDirectivesTransformer",
        &format!(
            "<Border {XMLNS} x:Class='A.B' x:Precompile='False' x:FieldModifier='public' x:ClassModifier='internal' x:Key='k' d:DesignWidth='1'/>"
        ),
    );
    let directives: Vec<String> = collect::<XamlAstXmlDirective>(&root)
        .iter()
        .map(|d| d.name.borrow().clone())
        .collect();
    assert_eq!(directives, ["Key", "DesignWidth"]);
}

// FerroXamlIlDesignPropertiesTransformer

const DESIGN_DOCUMENT: &str =
    "d:DesignWidth='10' d:DesignHeight='20' d:DataContext='ctx' d:Other='x' Design.Width='5' Tag='t'";

#[test]
fn design_markup_is_removed_outside_of_design_mode() {
    let fw = create_objects_test_framework();
    let compiler = pipeline_until(&fw, "FerroXamlIlDesignPropertiesTransformer");
    assert!(!compiler.is_design_mode());
    let root = transform_with(&compiler, &format!("<Border {XMLNS} {DESIGN_DOCUMENT}/>"))
        .expect("transformed");
    let object = root.cast::<XamlAstObjectNode>().expect("object");
    let children = object.children.borrow().clone();
    // The unknown d: directive and the ordinary property stay.
    assert_eq!(children.len(), 2);
    assert_eq!(
        *children[0].cast::<XamlAstXmlDirective>().expect("directive").name.borrow(),
        "Other"
    );
    assert!(children[1].is::<XamlAstXamlPropertyValueNode>());
}

#[test]
fn design_directives_map_to_design_properties_in_design_mode() {
    let fw = create_objects_test_framework();
    let compiler = pipeline_until(&fw, "FerroXamlIlDesignPropertiesTransformer");
    compiler.set_is_design_mode(true);
    assert!(compiler.is_design_mode());
    let root = transform_with(&compiler, &format!("<Border {XMLNS} {DESIGN_DOCUMENT}/>"))
        .expect("transformed");
    let object = root.cast::<XamlAstObjectNode>().expect("object");
    let mut mapped = Vec::new();
    for child in object.children.borrow().iter() {
        if let Some(value) = child.cast::<XamlAstXamlPropertyValueNode>() {
            let reference = value
                .property()
                .cast::<XamlAstNamePropertyReference>()
                .expect("reference");
            mapped.push((
                reference.declaring_type.borrow().to_node_string(),
                reference.name(),
                value.is_attribute_syntax,
            ));
        }
    }
    let design = format!("xml!!{FERRO_XML_NAMESPACE}:Design");
    assert_eq!(
        mapped,
        [
            (design.clone(), "Width".to_string(), true),
            (design.clone(), "Height".to_string(), true),
            (design.clone(), "DataContext".to_string(), true),
            (design, "Width".to_string(), true),
            (format!("xml!!{FERRO_XML_NAMESPACE}:Border"), "Tag".to_string(), true),
        ]
    );

    // The whole pipeline assigns them through the attached property accessors.
    let compiler = objects_pipeline(&fw);
    compiler.set_is_design_mode(true);
    let root = transform_with(
        &compiler,
        &format!("<Border {XMLNS} d:DesignWidth='10' Design.Height='4'/>"),
    )
    .expect("transformed");
    no_errors(&fw);
    for name in ["Width", "Height"] {
        let assignment = &assignments(&root, name)[0];
        assert_eq!(assignment.property.declaring_type().name(), "Design");
    }
}

// FerroXamlIlResolveClassesPropertiesTransformer, FerroXamlIlReorderClassesPropertiesTransformer,
// FerroXamlIlClassesTransformer

#[test]
fn classes_properties_are_resolved() {
    let fw = create_objects_test_framework();
    let root = transform(
        &fw,
        &format!("<Border {XMLNS} Classes.accent='True' Classes='a b'/>"),
    );
    no_errors(&fw);

    let class = &assignments(&root, "class:accent")[0];
    assert!(class.property.declaring_type().equals(&*fw.types.classes));
    assert!(class.property.getter().is_none());
    assert_eq!(setter_names(class), ["ClassValueSetter"]);
    let all_setters = class.property.setters();
    assert_eq!(all_setters.len(), 2);
    let value_setter = all_setters[0]
        .as_any()
        .downcast_ref::<ClassValueSetter>()
        .expect("value setter");
    assert_eq!(value_setter.class_name, "accent");
    assert!(all_setters[0].target_type().equals(&*fw.types.styled_element));
    assert_eq!(all_setters[0].parameters()[0].name(), "Boolean");
    assert!(!all_setters[0].binder_parameters().allow_x_null.get());
    assert!(all_setters[0].custom_attributes().is_empty());
    assert_eq!(value_setter.classes_set_method().expect("Set").name(), "Set");
    let binding_setter = all_setters[1]
        .as_any()
        .downcast_ref::<ClassBindingSetter>()
        .expect("binding setter");
    assert_eq!(binding_setter.class_name, "accent");
    assert!(all_setters[1].parameters()[0].equals(&*fw.types.binding_base));
    assert!(all_setters[1].target_type().equals(&*fw.types.styled_element));
    assert!(!all_setters[1].binder_parameters().allow_x_null.get());
    // Setters compare by identity.
    assert!(all_setters[0].equals(&*all_setters[0]));
    assert!(!all_setters[0].equals(&*all_setters[1]));

    // `Classes="a b"` became two adds, moved in front of the single class.
    let classes = assignments(&root, "Classes");
    assert_eq!(classes.len(), 2);
    assert_eq!(text_of(&classes[0].values.borrow()[0]), "a");
    assert_eq!(text_of(&classes[1].values.borrow()[0]), "b");
    let order: Vec<String> = collect::<XamlPropertyAssignmentNode>(&root)
        .iter()
        .map(|a| a.property.name())
        .collect();
    assert_eq!(order, ["Classes", "Classes", "class:accent"]);
}

#[test]
fn classes_properties_need_a_styled_element_and_the_classes_type() {
    let fw = create_objects_test_framework();
    let root = transform_until(
        &fw,
        "FerroXamlIlResolveClassesPropertiesTransformer",
        &format!(
            "<Border {XMLNS} Classes.accent='True' Grid.Row='1'><Border.Resources><SolidColorBrush x:Key='b' Classes.x='True'/></Border.Resources></Border>"
        ),
    );
    let resolved: Vec<String> = collect::<XamlAstClrProperty>(&root)
        .iter()
        .map(|p| p.name())
        .collect();
    assert_eq!(resolved, ["class:accent"]);
    // `Grid.Row` and the class of an object that is not a styled element stay unresolved.
    let unresolved: Vec<String> = collect::<XamlAstNamePropertyReference>(&root)
        .iter()
        .map(|p| p.name())
        .collect();
    assert_eq!(unresolved, ["Row", "Resources", "x"]);
}

#[test]
fn classes_are_not_reordered_without_a_single_class() {
    let fw = create_objects_test_framework();
    let root = transform_until(
        &fw,
        "FerroXamlIlReorderClassesPropertiesTransformer",
        &format!("<Border {XMLNS} Tag='t' Classes='a'/>"),
    );
    let object = root.cast::<XamlAstObjectNode>().expect("object");
    let order: Vec<String> = object
        .children
        .borrow()
        .iter()
        .filter_map(|c| c.cast::<XamlAstXamlPropertyValueNode>())
        .map(|v| v.property().cast::<XamlAstClrProperty>().expect("clr").name())
        .collect();
    assert_eq!(order, ["Tag", "Classes"]);

    let root = transform_until(
        &fw,
        "FerroXamlIlReorderClassesPropertiesTransformer",
        &format!("<Border {XMLNS} Tag='t' Classes.x='True' Classes.y='True' Classes='a'/>"),
    );
    let object = root.cast::<XamlAstObjectNode>().expect("object");
    let order: Vec<String> = object
        .children
        .borrow()
        .iter()
        .filter_map(|c| c.cast::<XamlAstXamlPropertyValueNode>())
        .map(|v| v.property().cast::<XamlAstClrProperty>().expect("clr").name())
        .collect();
    assert_eq!(order, ["Tag", "Classes", "class:x", "class:y"]);
}

#[test]
fn classes_text_is_split_only_in_attribute_syntax() {
    let fw = create_objects_test_framework();
    let last = "FerroXamlIlClassesTransformer";
    let root = transform_until(&fw, last, &format!("<Border {XMLNS} Classes='a  b'/>"));
    let value = &collect::<XamlAstXamlPropertyValueNode>(&root)[0];
    assert!(!value.is_attribute_syntax);
    let texts: Vec<String> = value.values.borrow().iter().map(text_of).collect();
    // `string.Split(' ')` keeps empty entries.
    assert_eq!(texts, ["a", "", "b"]);
    let first = value.values.borrow()[0].clone();
    assert!(first
        .type_()
        .get_clr_type()
        .expect("typed")
        .equals(&*fw.configuration.well_known_types().string));

    // Element syntax and other properties are not touched.
    let root = transform_until(
        &fw,
        last,
        &format!("<Border {XMLNS} Tag='a b'><Border.Classes>a b</Border.Classes></Border>"),
    );
    for value in collect::<XamlAstXamlPropertyValueNode>(&root) {
        assert_eq!(value.values.borrow().len(), 1);
        assert_eq!(text_of(&value.values.borrow()[0]), "a b");
    }
}

// FerroXamlIlTransformInstanceAttachedProperties

#[test]
fn instance_properties_of_unrelated_objects_become_attached_instance_properties() {
    let fw = create_objects_test_framework();
    let root = transform_until(
        &fw,
        "FerroXamlIlTransformInstanceAttachedProperties",
        &format!("<Border {XMLNS} TextBlock.FontSize='12' Grid.Row='1' Decorator.Padding='1'/>"),
    );
    let properties = collect::<XamlAstClrProperty>(&root);
    assert_eq!(properties.len(), 1);
    let property = &properties[0];
    assert_eq!(property.name(), "FontSize");
    assert_eq!(property.declaring_type().name(), "TextBlock");
    let instance = FerroAttachedInstanceProperty::from_clr_property(property).expect("instance");
    assert_eq!(instance.property_type.name(), "Double");
    assert_eq!(instance.field.name(), "FontSizeProperty");
    // The generic registered property type is replaced with its non-generic base.
    assert_eq!(
        instance.ferro_property_type.as_ref().expect("type").name(),
        "FerroProperty"
    );
    assert!(instance.ferro_object.equals(&*fw.types.ferro_object));
    assert_eq!(
        as_xaml_il_ferro_property(property)
            .expect("IXamlIlFerroProperty")
            .ferro_property()
            .name(),
        "FontSizeProperty"
    );
    assert_eq!(
        property.extension().expect("extension").type_name(),
        "FerroAttachedInstanceProperty"
    );

    let setters = property.setters();
    assert_eq!(setters.len(), 1);
    assert_eq!(setters[0].type_name(), "SetterMethod");
    assert_eq!(setters[0].target_type().name(), "TextBlock");
    let parameters: Vec<String> = setters[0].parameters().iter().map(|p| p.name()).collect();
    assert_eq!(parameters, ["FerroObject", "Double"]);
    assert!(setters[0].custom_attributes().is_empty());
    assert!(setters[0].binder_parameters().allow_x_null.get());
    assert_eq!(instance.set_value_method().expect("SetValue").name(), "SetValue");

    let getter = property.getter().expect("getter");
    assert_eq!(getter.name(), "FerroObject:GetValue_FontSize");
    assert!(getter.is_static() && getter.is_public() && !getter.is_private());
    assert_eq!(getter.return_type().name(), "Double");
    assert_eq!(getter.parameters()[0].name(), "FerroObject");
    assert_eq!(getter.declaring_type().name(), "TextBlock");
    assert!(getter.equals(&*getter));
    assert!(getter.make_generic_method(&[]).is_err());
    assert_eq!(
        getter.get_parameter_info(0).expect("info").parameter_type().name(),
        "FerroObject"
    );
    assert!(getter.get_parameter_info(1).is_err());
    assert!(getter.generic_arguments().is_empty() && getter.generic_parameters().is_empty());
    match instance.get_value_method() {
        Ok(method) => assert_eq!(method.name(), "GetValue"),
        Err(e) => assert_eq!(
            e.message(),
            "Unable to find T GetValue<T>(FerroProperty<T>) on FerroObject"
        ),
    }

    // Real attached properties (`Grid.Row`) and properties of a base class of the target
    // (`Decorator.Padding` on a `Border`) are left to the standard resolver.
    let unresolved: Vec<String> = collect::<XamlAstNamePropertyReference>(&root)
        .iter()
        .map(|p| p.name())
        .collect();
    assert_eq!(unresolved, ["Row", "Padding"]);
}

#[test]
fn instance_attached_properties_need_a_registered_instance_property() {
    let fw = create_objects_test_framework();
    // A CLR property without a registered property field, a missing property, and a target
    // that is not a FerroObject.
    let text_block = fw.fake_type("FerroUI.Controls.TextBlock");
    if !text_block.properties().iter().any(|p| p.name() == "PlainValue") {
        text_block.add_property("PlainValue", fw.t("System.String"));
    }
    let root = transform_until(
        &fw,
        "FerroXamlIlTransformInstanceAttachedProperties",
        &format!(
            "<Border {XMLNS} TextBlock.PlainValue='x' TextBlock.Missing='y'><Border.Resources><ResourceDictionary TextBlock.FontSize='1'/></Border.Resources></Border>"
        ),
    );
    assert!(collect::<XamlAstClrProperty>(&root).is_empty());
}

// FerroXamlIlTransformRoutedEvent

fn define_event_host(fw: &TestFramework) {
    if fw.as_type_system().find_type("Tests.EventHost").is_some() {
        return;
    }
    define_main_view(fw);
    let view = fw.fake_type("Tests.MainView");
    let key_event_args = fw.base.define_class("FerroUI.Input", "KeyEventArgs");
    key_event_args.set_base_type(fw.t("FerroUI.Interactivity.RoutedEventArgs"));
    view.add_method(
        "OnKey",
        fw.t("System.Void"),
        vec![fw.t("System.Object"), key_event_args.as_type()],
        false,
    );
    let routed_event_of_key = fw
        .t("FerroUI.Interactivity.RoutedEvent`1")
        .make_generic_type(&[key_event_args.as_type()])
        .expect("RoutedEvent<KeyEventArgs>");
    let host = fw.controls.define_class("Tests", "EventHost");
    host.set_base_type(fw.t("FerroUI.Controls.Control"));
    host.add_constructor(vec![]);
    host.add_field("KeyDownEvent", routed_event_of_key, true, None);
    host.add_field("PlainEvent", fw.t("FerroUI.Interactivity.RoutedEvent"), true, None);
    host.add_field("BrokenEvent", fw.t("System.String"), true, None);
    host.add_field("InstanceEvent", fw.t("FerroUI.Interactivity.RoutedEvent"), false, None);
}

const TESTS_XMLNS: &str = "xmlns:t='clr-namespace:Tests;assembly=FerroUI.Controls'";

#[test]
fn routed_event_attributes_become_add_handler_setters() {
    let fw = create_objects_test_framework();
    define_event_host(&fw);
    let compiler = pipeline_until(&fw, "FerroXamlIlTransformRoutedEvent");
    let document = compiler
        .parse(
            &format!(
                "<ContentControl {XMLNS} {TESTS_XMLNS} x:Class='Tests.MainView' Button.Click='OnClick' t:EventHost.KeyDown='OnKey' t:EventHost.Plain='OnClick' t:EventHost.Instance='OnClick' Tag='t'><Border.Resources><ResourceDictionary Button.Click='OnClick'/></Border.Resources></ContentControl>"
            ),
            None,
        );
    let mut document = document.expect("parsed");
    compiler.transform(&mut document).expect("transformed");
    no_errors(&fw);
    let root = document.root().expect("root");

    let properties = collect::<XamlAstClrProperty>(&root);
    let names: Vec<String> = properties.iter().map(|p| p.name()).collect();
    assert_eq!(names, ["Click", "KeyDown", "Plain"]);

    // The property belongs to the target type and has no getter.
    let click = &properties[0];
    assert_eq!(click.declaring_type().name(), "MainView");
    assert!(click.getter().is_none());
    // RoutedEvent<RoutedEventArgs>: only the non-generic AddHandler.
    let setters = click.setters();
    assert_eq!(setters.len(), 1);
    let add_handler = setters[0]
        .as_any()
        .downcast_ref::<XamlDirectCallAddHandler>()
        .expect("add handler");
    assert_eq!(add_handler.event_field.name(), "ClickEvent");
    assert_eq!(add_handler.declaring_type.name(), "MainView");
    assert!(add_handler
        .add_method
        .equals(&*fw.types.interactivity.add_handler));
    assert!(setters[0].parameters()[0].equals(&*fw.types.interactivity.routed_event_handler));
    assert_eq!(setters[0].target_type().name(), "MainView");
    assert!(setters[0].custom_attributes().is_empty());
    assert_eq!(XamlDirectCallAddHandler::ROUTING_STRATEGIES, 5);
    assert!(!XamlDirectCallAddHandler::HANDLED_EVENTS_TOO);

    // RoutedEvent<KeyEventArgs>: also the generic AddHandler<KeyEventArgs>.
    let setters = properties[1].setters();
    assert_eq!(setters.len(), 2);
    let typed = setters[1]
        .as_any()
        .downcast_ref::<XamlDirectCallAddHandler>()
        .expect("typed add handler");
    assert_eq!(typed.add_method.name(), "AddHandler");
    let handler_type = setters[1].parameters()[0].clone();
    assert_eq!(handler_type.generic_arguments()[0].name(), "KeyEventArgs");
    assert_eq!(typed.add_method.parameters()[1].generic_arguments()[0].name(), "KeyEventArgs");

    // A non-generic routed event field.
    assert_eq!(properties[2].setters().len(), 1);

    // Instance fields and targets that are not interactive are not events.
    let unresolved: Vec<String> = collect::<XamlAstNamePropertyReference>(&root)
        .iter()
        .map(|p| p.name())
        .collect();
    assert_eq!(unresolved, ["Instance", "Tag", "Resources", "Click"]);
}

#[test]
fn routed_event_handlers_are_assigned_through_the_whole_pipeline() {
    let fw = create_objects_test_framework();
    define_event_host(&fw);
    let compiler = objects_pipeline(&fw);
    let mut document = compiler
        .parse(
            &format!(
                "<ContentControl {XMLNS} {TESTS_XMLNS} x:Class='Tests.MainView'><Button Click='OnClick' t:EventHost.KeyDown='OnKey'/></ContentControl>"
            ),
            None,
        )
        .expect("parsed");
    compiler.transform(&mut document).expect("transformed");
    no_errors(&fw);
    let root = document.root().expect("root");

    let click = &assignments(&root, "Click")[0];
    assert_eq!(setter_names(click), ["XamlDirectCallAddHandler"]);
    assert!(click.values.borrow()[0].is::<XamlLoadMethodDelegateNode>());
    let key_down = &assignments(&root, "KeyDown")[0];
    assert_eq!(setter_names(key_down), ["XamlDirectCallAddHandler"]);
    let delegate = key_down.values.borrow()[0]
        .cast::<XamlLoadMethodDelegateNode>()
        .expect("delegate");
    assert_eq!(delegate.method.name(), "OnKey");
}

#[test]
fn incompatible_event_fields_are_reported() {
    let fw = create_objects_test_framework();
    define_event_host(&fw);
    let root = transform_until(
        &fw,
        "FerroXamlIlTransformRoutedEvent",
        &format!("<Button {XMLNS} {TESTS_XMLNS} t:EventHost.Broken='OnClick'/>"),
    );
    let diagnostics = fw.reported_diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, FerroXamlDiagnosticCodes::TRANSFORM_ERROR);
    assert_eq!(diagnostics[0].severity, XamlDiagnosticSeverity::Error);
    assert_eq!(
        diagnostics[0].title,
        "Event definition Broken found, but its type System.Runtime:System.String is not compatible with RoutedEvent."
    );
    assert_eq!(diagnostics[0].line_number, Some(1));
    assert!(collect::<XamlAstClrProperty>(&root).is_empty());
}

// FerroXamlIlFerroPropertyResolver

#[test]
fn clr_properties_with_a_property_field_become_ferro_properties() {
    let fw = create_objects_test_framework();
    let root = transform_until(
        &fw,
        "FerroXamlIlFerroPropertyResolver",
        &format!("<Border {XMLNS} Padding='1' Classes='a' Grid.Row='2'/>"),
    );
    let properties = collect::<XamlAstClrProperty>(&root);
    let padding = properties.iter().find(|p| p.name() == "Padding").expect("Padding");
    let registered = XamlIlFerroProperty::from_clr_property(padding).expect("registered");
    assert_eq!(registered.ferro_property().name(), "PaddingProperty");
    assert!(padding.setters().len() > 1);
    // Attached properties are resolved the same way, through the declaring type's field.
    let row = properties.iter().find(|p| p.name() == "Row").expect("Row");
    assert_eq!(
        XamlIlFerroProperty::from_clr_property(row)
            .expect("registered")
            .ferro_property()
            .name(),
        "RowProperty"
    );
    // No `ClassesProperty` field: a plain CLR property.
    let classes = properties.iter().find(|p| p.name() == "Classes").expect("Classes");
    assert!(XamlIlFerroProperty::from_clr_property(classes).is_none());
    assert!(classes.extension().is_none());
}

// FerroXamlIlConstructorServiceProviderTransformer

#[test]
fn service_provider_is_injected_into_constructors_that_need_it() {
    let fw = create_objects_test_framework();
    let root = transform_until(
        &fw,
        "FerroXamlIlConstructorServiceProviderTransformer",
        &format!(
            "<ResourceDictionary {XMLNS}><ResourceDictionary.MergedDictionaries><ResourceInclude Source='/a.xaml'/><ResourceDictionary/></ResourceDictionary.MergedDictionaries><x:String x:Key='s'><x:Arguments><x:String>a</x:String></x:Arguments></x:String></ResourceDictionary>"
        ),
    );
    let injected = collect::<InjectServiceProviderNode>(&root);
    assert_eq!(injected.len(), 1);
    let include = collect::<XamlAstObjectNode>(&root)
        .into_iter()
        .find(|o| o.type_.borrow().to_node_string().ends_with("ResourceInclude"))
        .expect("include");
    assert_eq!(include.arguments.borrow().len(), 1);
    assert!(include.arguments.borrow()[0].same_node(&injected[0]));
    let node: Rc<dyn IXamlAstNode> = injected[0].clone();
    assert!(node
        .as_needs_parent_stack()
        .expect("needs parent stack")
        .needs_parent_stack());
    let injected_value: Rc<dyn IXamlAstValueNode> = injected[0].clone();
    assert!(injected_value
        .type_()
        .get_clr_type()
        .expect("typed")
        .equals(&*fw.configuration.type_mappings.service_provider().expect("sp")));
    // Line info of the object.
    assert_eq!(
        (xamlx::ast::IXamlLineInfo::line(&*injected[0]), xamlx::ast::IXamlLineInfo::line(&*include)),
        (1, 1)
    );
}

// FerroXamlIlTransitionsTypeMetadataTransformer, FerroXamlIlMetadataRemover

#[test]
fn transitions_values_get_a_target_type_scope() {
    let fw = create_objects_test_framework();
    let xaml = format!(
        "<Border {XMLNS} Tag='t'><Border.Transitions><Transitions/></Border.Transitions></Border>"
    );
    let root = transform_until(&fw, "FerroXamlIlTransitionsTypeMetadataTransformer", &xaml);
    let scopes = collect::<FerroXamlIlTargetTypeMetadataNode>(&root);
    assert_eq!(scopes.len(), 1);
    assert_eq!(scopes[0].scope_type, ScopeTypes::Transitions);
    assert_eq!(
        scopes[0].target_type().get_clr_type().expect("typed").name(),
        "Border"
    );
    assert!(scopes[0].value().is::<XamlAstObjectNode>());

    // The metadata remover unwraps the node again.
    let root = transform(&fw, &xaml);
    no_errors(&fw);
    assert!(collect::<FerroXamlIlTargetTypeMetadataNode>(&root).is_empty());
    let transitions = &assignments(&root, "Transitions")[0];
    assert!(transitions.values.borrow()[0].is::<XamlValueWithManipulationNode>());
}

#[test]
fn metadata_remover_unwraps_nested_metadata_nodes() {
    let fw = create_objects_test_framework();
    let context = fw.create_context();
    let text: Rc<dyn IXamlAstValueNode> =
        XamlAstTextNode::new(&xamlx::ast::XamlLineInfo::new(1, 1), "x", true);
    let type_reference = text.type_();
    let inner =
        FerroXamlIlTargetTypeMetadataNode::new(text.clone(), type_reference.clone(), ScopeTypes::Style);
    let outer =
        FerroXamlIlTargetTypeMetadataNode::new(inner, type_reference, ScopeTypes::ControlTemplate);
    let result = FerroXamlIlMetadataRemover
        .transform(&context, outer)
        .expect("transformed");
    assert!(result.same_node(&text));
    let unchanged = FerroXamlIlMetadataRemover
        .transform(&context, text.as_node())
        .expect("transformed");
    assert!(unchanged.same_node(&text));
}

// FerroXamlIlResolveByNameMarkupExtensionReplacer

#[test]
fn resolve_by_name_text_becomes_a_markup_extension() {
    let fw = create_objects_test_framework();
    let root = transform(
        &fw,
        &format!("<StackPanel {XMLNS}><Label Target='input' Tag='t'/><Label><Label.Target><Border/></Label.Target></Label></StackPanel>"),
    );
    no_errors(&fw);
    let targets = assignments(&root, "Target");
    assert_eq!(targets.len(), 2);
    let extension = targets[0].values.borrow()[0]
        .cast::<XamlMarkupExtensionNode>()
        .expect("markup extension");
    assert_eq!(extension.provide_value.name(), "ProvideValue");
    let created = collect::<XamlAstNewClrObjectNode>(&extension.as_node());
    assert_eq!(
        created[0].type_.borrow().clone().get_clr_type().expect("typed").name(),
        "ResolveByNameExtension"
    );
    assert_eq!(text_of(&created[0].arguments.borrow()[0]), "input");
    // A value that is not text and properties without the attribute are left alone.
    assert!(targets[1].values.borrow()[0].is::<XamlValueWithManipulationNode>());
    assert!(assignments(&root, "Tag")[0].values.borrow()[0].is::<XamlAstTextNode>());
}

// FerroXamlIlThemeVariantProviderTransformer

#[test]
fn theme_dictionary_keys_are_copied_to_the_key_property() {
    let fw = create_objects_test_framework();
    let root = transform_until(
        &fw,
        "FerroXamlIlThemeVariantProviderTransformer",
        &format!(
            "<ResourceDictionary {XMLNS} x:Key='root'><ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key='Dark'/><ResourceDictionary/></ResourceDictionary.ThemeDictionaries><ResourceDictionary.MergedDictionaries><ResourceDictionary x:Key='Light'/></ResourceDictionary.MergedDictionaries><ResourceDictionary x:Key='nested'/></ResourceDictionary>"
        ),
    );
    let keys: Vec<Rc<XamlAstXamlPropertyValueNode>> = collect::<XamlAstXamlPropertyValueNode>(&root)
        .into_iter()
        .filter(|v| {
            v.property()
                .cast::<XamlAstClrProperty>()
                .is_some_and(|p| p.name() == "Key")
        })
        .collect();
    // Only the keyed entry of the theme dictionaries collection gets a `Key` assignment.
    assert_eq!(keys.len(), 1);
    assert!(keys[0].is_attribute_syntax);
    assert_eq!(text_of(&keys[0].values.borrow()[0]), "Dark");
    let property = keys[0].property().cast::<XamlAstClrProperty>().expect("clr");
    assert_eq!(property.declaring_type().name(), "IThemeVariantProvider");
    // The directive itself stays (it still is the dictionary key).
    let directives = collect::<XamlAstXmlDirective>(&root);
    assert_eq!(directives.len(), 4);

    // Through the whole pipeline the key is converted to a theme variant.
    let root = transform(
        &fw,
        &format!(
            "<ResourceDictionary {XMLNS}><ResourceDictionary.ThemeDictionaries><ResourceDictionary x:Key='Dark'/></ResourceDictionary.ThemeDictionaries></ResourceDictionary>"
        ),
    );
    no_errors(&fw);
    let key = &assignments(&root, "Key")[0];
    assert_eq!(
        key.values.borrow()[0].type_().get_clr_type().expect("typed").name(),
        "ThemeVariant"
    );
}

// FerroXamlIlOptionMarkupExtensionTransformer

fn options_node(root: &Rc<dyn IXamlAstNode>) -> (Rc<XamlMarkupExtensionNode>, Rc<OptionsMarkupExtensionNode>) {
    let node = collect::<XamlMarkupExtensionNode>(root)
        .into_iter()
        .next()
        .expect("markup extension");
    let options = OptionsMarkupExtensionNode::from_node(&node).expect("options node");
    (node, options)
}

#[test]
fn option_properties_become_branches() {
    let fw = create_objects_test_framework();
    let root = transform(
        &fw,
        &format!("<TextBlock {XMLNS} Text=\"{{OnPlatform Default='a', Windows='b', macOS='c'}}\"/>"),
    );
    no_errors(&fw);
    let (node, options) = options_node(&root);
    assert_eq!(xamlx::ast::IXamlAstNode::type_name(&*node), "OptionsMarkupExtensionNode");
    let method = options.provide_value();
    assert!(node.provide_value.equals(&**method as &dyn IXamlMethod));
    assert_eq!(method.name(), "ProvideValue");
    assert!(method.is_public() && !method.is_static());
    assert_eq!(method.declaring_type().name(), "OnPlatformExtension");
    // No condition needs the service provider.
    assert!(method.parameters().is_empty());
    let value_node: Rc<dyn IXamlAstNode> = node.clone();
    assert!(!value_node
        .as_needs_parent_stack()
        .expect("needs parent stack")
        .needs_parent_stack());
    assert!(method.make_generic_method(&[]).is_err());
    assert!(method.custom_attributes().is_empty());
    assert!(method.get_parameter_info(0).is_err());

    let container = &method.extension_node_container;
    let branches = container.branches();
    assert_eq!(branches.len(), 2);
    let options_text: Vec<XamlValue> = branches
        .iter()
        .map(|b| b.option().cast::<XamlConstantNode>().expect("constant").constant.clone())
        .collect();
    assert_eq!(
        options_text,
        [
            XamlValue::String("WINDOWS".to_string()),
            XamlValue::String("OSX".to_string())
        ]
    );
    assert_eq!(text_of(&branches[0].value()), "b");
    assert_eq!(text_of(&branches[1].value()), "c");
    assert!(!branches[0].has_context());
    assert_eq!(branches[0].condition_method.name(), "ShouldProvideOption");
    assert_eq!(text_of(&container.default_node().expect("default")), "a");

    // The node is typed with the common type of the branches, and was accepted by the
    // string property.
    assert_eq!(method.return_type().name(), "String");
    let typed: Rc<dyn IXamlAstValueNode> = node.clone();
    assert_eq!(typed.type_().get_clr_type().expect("typed").name(), "String");
    // The option properties were removed from the extension object.
    assert!(assignments(&root, "Windows").is_empty());
    assert!(assignments(&root, "Default").is_empty());
    let text = &assignments(&root, "Text")[0];
    assert!(text.values.borrow()[0].same_node(&node));
}

#[test]
fn positional_argument_is_the_default_option() {
    let fw = create_objects_test_framework();
    let root = transform(
        &fw,
        &format!("<TextBlock {XMLNS} Text=\"{{OnPlatform fallback, Linux='l'}}\"/>"),
    );
    no_errors(&fw);
    let (node, options) = options_node(&root);
    let container = &options.provide_value().extension_node_container;
    assert_eq!(text_of(&container.default_node().expect("default")), "fallback");
    assert_eq!(container.branches().len(), 1);
    // The argument was taken away from the constructor call.
    let created = collect::<XamlAstNewClrObjectNode>(&node.value().as_node());
    assert!(created[0].arguments.borrow().is_empty());
}

#[test]
fn conditions_with_a_service_provider_and_enum_options() {
    let fw = create_objects_test_framework();
    let root = transform(
        &fw,
        &format!("<TextBlock {XMLNS} Text=\"{{OnFormFactor Desktop='d', Mobile='m'}}\"/>"),
    );
    no_errors(&fw);
    let (node, options) = options_node(&root);
    let method = options.provide_value();
    // An instance condition method that takes the service provider.
    let parameters: Vec<String> = method.parameters().iter().map(|p| p.name()).collect();
    assert_eq!(parameters, ["IServiceProvider"]);
    assert_eq!(
        method.get_parameter_info(0).expect("info").parameter_type().name(),
        "IServiceProvider"
    );
    let value_node: Rc<dyn IXamlAstNode> = node.clone();
    assert!(value_node
        .as_needs_parent_stack()
        .expect("needs parent stack")
        .needs_parent_stack());
    let container = &method.extension_node_container;
    assert!(container.default_node().is_none());
    let branches = container.branches();
    assert!(branches[0].has_context());
    assert!(!branches[0].condition_method.is_static());
    let option = branches[1].option().cast::<XamlConstantNode>().expect("constant");
    assert_eq!(option.constant, XamlValue::Int32(2));
    assert_eq!(
        branches[1].option().type_().get_clr_type().expect("typed").name(),
        "FormFactorType"
    );
}

#[test]
fn on_children_select_options_by_property_name() {
    let fw = create_objects_test_framework();
    let root = transform(
        &fw,
        &format!(
            "<TextBlock {XMLNS}><TextBlock.Text><OnPlatform><OnPlatform.Default>a</OnPlatform.Default><On Options='Windows, Linux'>wl</On><On Options='Default'>d</On></OnPlatform></TextBlock.Text></TextBlock>"
        ),
    );
    no_errors(&fw);
    let (_, options) = options_node(&root);
    let container = &options.provide_value().extension_node_container;
    let branches = container.branches();
    let options_text: Vec<XamlValue> = branches
        .iter()
        .map(|b| b.option().cast::<XamlConstantNode>().expect("constant").constant.clone())
        .collect();
    assert_eq!(
        options_text,
        [
            XamlValue::String("WINDOWS".to_string()),
            XamlValue::String("LINUX".to_string())
        ]
    );
    assert_eq!(text_of(&branches[0].value()), "wl");
    assert_eq!(text_of(&branches[1].value()), "wl");
    // `Options="Default"` names the default option property; the later one wins.
    assert_eq!(text_of(&container.default_node().expect("default")), "d");
}

#[test]
fn option_values_are_converted_to_the_property_type() {
    let fw = create_objects_test_framework();
    let root = transform(
        &fw,
        &format!("<Border {XMLNS} Padding=\"{{OnPlatform Default='1', Windows='2,3'}}\"/>"),
    );
    no_errors(&fw);
    let (node, options) = options_node(&root);
    let typed: Rc<dyn IXamlAstValueNode> = node.clone();
    assert_eq!(typed.type_().get_clr_type().expect("typed").name(), "Thickness");
    let container = &options.provide_value().extension_node_container;
    assert_eq!(
        container.branches()[0].value().type_().get_clr_type().expect("typed").name(),
        "Thickness"
    );
    assert_eq!(
        container.get_return_type().expect("return type").name(),
        "Thickness"
    );

    // The conversion is offered through the interface the custom value converter queries.
    let context = fw.create_context();
    let interface = query_node_interface::<dyn IOptionsMarkupExtensionNode>(&node.as_node())
        .expect("IOptionsMarkupExtensionNode");
    let thickness = fw.t("FerroUI.Thickness");
    let converted = interface
        .convert_to_return_type(&context, &thickness)
        .expect("no error")
        .expect("converted");
    assert!(!converted.same_node(&node));
    assert_eq!(converted.type_().get_clr_type().expect("typed").name(), "Thickness");
    // A type the values cannot be converted to.
    let control = fw.t("FerroUI.Controls.Control");
    assert!(interface
        .convert_to_return_type(&context, &control)
        .expect("no error")
        .is_none());
}

#[test]
fn option_markup_extension_errors() {
    let cases = [
        (
            "<TextBlock Text=\"{OnPlatform}\"/>",
            "Options markup extension requires at least one option to be set",
        ),
        (
            "<TextBlock Text=\"{OnPlatform a, b}\"/>",
            "Options MarkupExtensions allow only single argument",
        ),
        (
            "<TextBlock><TextBlock.Text><OnPlatform><On>x</On></OnPlatform></TextBlock.Text></TextBlock>",
            "On.Options string must be set",
        ),
        (
            "<TextBlock><TextBlock.Text><OnPlatform><On Options='Windows'/></OnPlatform></TextBlock.Text></TextBlock>",
            "On content object must be set",
        ),
        (
            "<TextBlock><TextBlock.Text><OnPlatform><On Options='Amiga'>x</On></OnPlatform></TextBlock.Text></TextBlock>",
            "Property \"Amiga\" wasn't found on the \"OnPlatformExtension\" type",
        ),
        (
            "<TextBlock><TextBlock.Text><OnPlatform><OnPlatform.Windows><x:String>a</x:String><x:String>b</x:String></OnPlatform.Windows></OnPlatform></TextBlock.Text></TextBlock>",
            "Options markup extension supports only a singular value",
        ),
        (
            "<TextBlock Text=\"{OnFormFactor Tv='t'}\"/>",
            "Option value \"Television\" is not assignable to any of existing ShouldProvideOption methods",
        ),
    ];
    for (body, expected) in cases {
        let fw = create_objects_test_framework();
        let xaml = body.replacen('>', &format!(" {XMLNS}>"), 1);
        let xaml = if body.starts_with("<TextBlock Text") {
            body.replacen("<TextBlock ", &format!("<TextBlock {XMLNS} "), 1)
        } else {
            xaml
        };
        let _ = transform_objects(&fw, &xaml);
        let errors = error_titles(&fw);
        assert!(
            errors.iter().any(|e| e.contains(expected)),
            "{body}: {errors:?}"
        );
    }
}

#[test]
fn markup_extensions_without_option_methods_are_left_alone() {
    let fw = create_objects_test_framework();
    let root = transform(&fw, &format!("<Label {XMLNS} Target='x'/>"));
    let node = &collect::<XamlMarkupExtensionNode>(&root)[0];
    assert!(OptionsMarkupExtensionNode::from_node(node).is_none());
    assert_eq!(xamlx::ast::IXamlAstNode::type_name(&**node), "XamlMarkupExtensionNode");
}

// FerroXamlResourceTransformer, FerroXamlIlEnsureResourceDictionaryCapacityTransformer

fn resource_setter(assignment: &XamlPropertyAssignmentNode) -> Rc<dyn xamlx::ast::IXamlPropertySetter> {
    let setters = assignment.possible_setters.borrow();
    assert_eq!(setters.len(), 1);
    setters[0].clone()
}

#[test]
fn element_resources_are_added_deferred() {
    let fw = create_objects_test_framework();
    let root = transform(
        &fw,
        &format!(
            "<Border {XMLNS}>\n<Border.Resources>\n<SolidColorBrush x:Key='b'/>\n<x:String x:Key='s'>text</x:String>\n<Border x:Key='named' x:Name='n'/>\n<SolidColorBrush x:Key='c' x:Shared='False'/>\n<SolidColorBrush x:Key='d' x:Shared='True'/>\n</Border.Resources>\n</Border>"
        ),
    );
    no_errors(&fw);
    let resources = assignments(&root, "Resources");
    assert_eq!(resources.len(), 5);

    // A reference type resource: deferred.
    let setter = resource_setter(&resources[0]);
    let adder_setter = setter
        .as_any()
        .downcast_ref::<ResourceAdderSetter>()
        .expect("resource adder setter");
    assert_eq!(adder_setter.adder.name(), "AddDeferred");
    assert_eq!(adder_setter.getter.as_ref().expect("getter").name(), "get_Resources");
    assert!(adder_setter.emit_source_info);
    assert_eq!(adder_setter.document.as_deref(), Some("test.xaml"));
    assert_eq!((adder_setter.line, adder_setter.position), (3, 2));
    assert_eq!(setter.target_type().name(), "StyledElement");
    let parameters: Vec<String> = setter.parameters().iter().map(|p| p.name()).collect();
    assert_eq!(parameters, ["Object", "IDeferredContent"]);
    assert!(!setter.binder_parameters().allow_multiple.get());
    assert!(!setter.binder_parameters().allow_attribute_syntax.get());
    assert!(setter.custom_attributes().is_empty());
    assert_eq!(text_of(&resources[0].values.borrow()[0]), "b");
    let deferred = resources[0].values.borrow()[1]
        .cast::<XamlDeferredContentNode>()
        .expect("deferred");
    assert!(deferred
        .deferred_content_customization_type_parameter()
        .expect("type parameter")
        .equals(&*fw.configuration.well_known_types().object));

    // Strings are never deferred.
    let setter = resource_setter(&resources[1]);
    let adder_setter = setter.as_any().downcast_ref::<ResourceAdderSetter>().expect("setter");
    assert_eq!(adder_setter.adder.name(), "Add");
    assert!(!resources[1].values.borrow()[1].is::<XamlDeferredContentNode>());

    // A resource with a name registration is not deferred.
    let setter = resource_setter(&resources[2]);
    let adder_setter = setter.as_any().downcast_ref::<ResourceAdderSetter>().expect("setter");
    assert_eq!(adder_setter.adder.name(), "Add");

    // A valid x:Shared directive is removed and selects the not-shared adder (as upstream,
    // whatever its value).
    for resource in &resources[3..] {
        let setter = resource_setter(resource);
        let adder_setter = setter.as_any().downcast_ref::<ResourceAdderSetter>().expect("setter");
        assert_eq!(adder_setter.adder.name(), "AddNotSharedDeferred");
    }
    assert!(collect::<XamlAstXmlDirective>(&root).is_empty());

    // Setters with a getter are equal when getter and adder are.
    let first = resource_setter(&resources[0]);
    assert!(first.equals(&*first));
    assert!(!first.equals(&*resource_setter(&resources[1])));
    assert!(resource_setter(&resources[3]).equals(&*resource_setter(&resources[4])));

    // The capacity is reserved once, through the same getter.
    let capacity = collect::<EnsureCapacityNode>(&root);
    assert_eq!(capacity.len(), 1);
    assert_eq!(capacity[0].capacity, 5);
    assert_eq!(
        capacity[0].resources_getter.as_ref().expect("getter").name(),
        "get_Resources"
    );
}

#[test]
fn resource_dictionary_content_is_added_deferred() {
    let fw = create_objects_test_framework();
    let compiler = objects_pipeline(&fw);
    compiler.set_create_source_info(false);
    assert!(!compiler.create_source_info());
    let root = transform_with(
        &compiler,
        &format!(
            "<ResourceDictionary {XMLNS}><SolidColorBrush x:Key='b'/><x:Double x:Key='n'>1</x:Double></ResourceDictionary>"
        ),
    )
    .expect("transformed");
    no_errors(&fw);
    let content = assignments(&root, "Content");
    assert_eq!(content.len(), 2);
    let setter = resource_setter(&content[0]);
    let adder_setter = setter.as_any().downcast_ref::<ResourceAdderSetter>().expect("setter");
    assert!(adder_setter.getter.is_none());
    assert!(!adder_setter.emit_source_info);
    assert_eq!(adder_setter.adder.name(), "AddDeferred");
    assert_eq!(setter.target_type().name(), "ResourceDictionary");
    assert!(setter.binder_parameters().allow_multiple.get());
    assert!(setter.binder_parameters().allow_attribute_syntax.get());
    // Without a getter two setters are only equal when they are the same object.
    assert!(setter.equals(&*setter));
    let other = resource_setter(&content[1]);
    assert!(!other.equals(&*setter));
    // Value types are not deferred.
    let adder_setter = other.as_any().downcast_ref::<ResourceAdderSetter>().expect("setter");
    assert_eq!(adder_setter.adder.name(), "Add");
    assert_eq!(adder_setter.target_type.name(), "IResourceDictionary");

    let capacity = collect::<EnsureCapacityNode>(&root);
    assert_eq!(capacity.len(), 1);
    assert_eq!(capacity[0].capacity, 2);
    assert!(capacity[0].resources_getter.is_none());
}

#[test]
fn invalid_shared_directives_are_reported() {
    let fw = create_objects_test_framework();
    let _ = transform_objects(
        &fw,
        &format!("<ResourceDictionary {XMLNS}><SolidColorBrush x:Key='b' x:Shared='maybe'/></ResourceDictionary>"),
    );
    let errors = error_titles(&fw);
    assert!(
        errors.iter().any(|e| e.contains("Invalid argument type for x:Shared directive.")),
        "{errors:?}"
    );
}

#[test]
fn capacity_is_not_reserved_for_a_single_resource_or_other_groups() {
    let fw = create_objects_test_framework();
    let root = transform(
        &fw,
        &format!(
            "<Border {XMLNS} Tag='t' Padding='1'><Border.Resources><SolidColorBrush x:Key='b'/></Border.Resources></Border>"
        ),
    );
    no_errors(&fw);
    assert!(collect::<EnsureCapacityNode>(&root).is_empty());

    // Applying the transformer twice to the same group does nothing the second time.
    let transformer = FerroXamlIlEnsureResourceDictionaryCapacityTransformer::new();
    let context = fw.create_context();
    let root = transform(
        &fw,
        &format!(
            "<ResourceDictionary {XMLNS}><x:String x:Key='a'>1</x:String><x:String x:Key='b'>2</x:String></ResourceDictionary>"
        ),
    );
    let group = collect::<XamlManipulationGroupNode>(&root)
        .into_iter()
        .find(|g| g.children.borrow().iter().any(|c| c.is::<EnsureCapacityNode>()))
        .expect("group");
    group.children.borrow_mut().retain(|c| !c.is::<EnsureCapacityNode>());
    transformer.apply(&context, &group).expect("applied");
    assert_eq!(collect::<EnsureCapacityNode>(&root).len(), 1);
    transformer.apply(&context, &group).expect("applied");
    assert_eq!(collect::<EnsureCapacityNode>(&root).len(), 1);
}

// AddNameScopeRegistration

#[test]
fn names_are_registered_in_the_name_scope() {
    let fw = create_objects_test_framework();
    let root = transform(
        &fw,
        &format!(
            "<StackPanel {XMLNS}><Border x:Name='b'/><TextBlock Name='t'/><ContentControl><ContentControl.ContentTemplate><DataTemplate><Border x:Name='inner'/></DataTemplate></ContentControl.ContentTemplate></ContentControl></StackPanel>"
        ),
    );
    no_errors(&fw);
    let registrations = collect::<FerroNameScopeRegistrationXamlIlNode>(&root);
    let names: Vec<(String, String)> = registrations
        .iter()
        .map(|r| {
            (
                text_of(&r.name()),
                r.target_type.as_ref().map(|t| t.name()).unwrap_or_default(),
            )
        })
        .collect();
    assert_eq!(
        names,
        [
            ("b".to_string(), "Border".to_string()),
            ("t".to_string(), "TextBlock".to_string()),
            ("inner".to_string(), "Border".to_string())
        ]
    );
    // The registration follows the assignment in a group of its own.
    let groups: Vec<Rc<XamlManipulationGroupNode>> = collect::<XamlManipulationGroupNode>(&root)
        .into_iter()
        .filter(|g| {
            g.children
                .borrow()
                .iter()
                .any(|c| c.is::<FerroNameScopeRegistrationXamlIlNode>())
        })
        .collect();
    assert_eq!(groups.len(), 3);
    for group in groups {
        let children = group.children.borrow();
        assert_eq!(children.len(), 2);
        let assignment = children[0].cast::<XamlPropertyAssignmentNode>().expect("assignment");
        assert_eq!(assignment.property.name(), "Name");
    }

    // The template content is a nested scope for the name scope visitor.
    let root = transform_until(
        &fw,
        "AddNameScopeRegistration",
        &format!(
            "<ContentControl {XMLNS} x:Name='outer'><ContentControl.ContentTemplate><DataTemplate><Border x:Name='inner'/></DataTemplate></ContentControl.ContentTemplate></ContentControl>"
        ),
    );
    assert_eq!(collect::<NestedScopeMetadataNode>(&root).len(), 1);
    let mut visitor = crate::compiler_extensions::visitors::NameScopeRegistrationVisitor::new(0, 0);
    xamlx::ast::visit_node(&root, &mut visitor).expect("visited");
    assert_eq!(visitor.len(), 1);
    assert!(visitor.contains_key("outer"));
}

#[test]
fn names_that_are_not_constant_are_kept_in_a_local() {
    let fw = create_objects_test_framework();
    let name_holder = fw.fake_type("FerroUI.Controls.Control");
    if !name_holder.fields().iter().any(|f| f.name() == "DefaultName") {
        name_holder.add_field("DefaultName", fw.t("System.String"), true, None);
    }
    let root = transform(
        &fw,
        &format!("<Border {XMLNS} Name='{{x:Static Control.DefaultName}}'/>"),
    );
    no_errors(&fw);
    let registration = &collect::<FerroNameScopeRegistrationXamlIlNode>(&root)[0];
    let local = registration
        .name()
        .cast::<xamlx::ast::XamlAstCompilerLocalNode>()
        .expect("local");
    let name = &assignments(&root, "Name")[0];
    let initialization = name.values.borrow()[0]
        .cast::<xamlx::ast::XamlAstLocalInitializationNodeEmitter>()
        .expect("local initialization");
    assert!(initialization.local().same_node(&local));
}

// FerroXamlIlRootObjectScope

#[test]
fn root_object_scope_is_handled_after_the_root_manipulation() {
    let fw = create_objects_test_framework();
    let root = transform(&fw, &format!("<StackPanel {XMLNS}><Border/></StackPanel>"));
    no_errors(&fw);
    let scopes = collect::<HandleRootObjectScopeNode>(&root);
    assert_eq!(scopes.len(), 1);
    let root_node = root.as_value_with_manipulation_node().expect("root");
    let group = root_node
        .manipulation()
        .expect("manipulation")
        .cast::<XamlManipulationGroupNode>()
        .expect("group");
    let children = group.children.borrow();
    assert_eq!(children.len(), 2);
    assert!(children[1].same_node(&scopes[0]));
    assert!(Rc::ptr_eq(&scopes[0].types, &fw.types));
}

// FerroXamlIlAddSourceInfoTransformer

#[test]
fn source_info_is_attached_to_created_objects() {
    let fw = create_objects_test_framework();
    let xaml = format!("<StackPanel {XMLNS}>\n  <Border Padding='1'/>\n</StackPanel>");
    let compiler = objects_pipeline(&fw);
    compiler.set_create_source_info(false);
    let root = transform_with(&compiler, &xaml).expect("transformed");
    assert!(collect::<XamlSourceInfoValueManipulation>(&root).is_empty());

    compiler.set_create_source_info(true);
    assert!(compiler.create_source_info());
    let root = transform_with(&compiler, &xaml).expect("transformed");
    no_errors(&fw);
    let infos = collect::<XamlSourceInfoValueManipulation>(&root);
    // The panel and the border; the thickness constant is not an object creation.
    assert_eq!(infos.len(), 2);
    let positions: Vec<(i32, i32)> = infos
        .iter()
        .map(|i| {
            (
                xamlx::ast::IXamlLineInfo::line(&**i),
                xamlx::ast::IXamlLineInfo::position(&**i),
            )
        })
        .collect();
    assert_eq!(positions, [(1, 2), (2, 4)]);
    assert_eq!(infos[0].document.as_deref(), Some("test.xaml"));
    // Each object creation is wrapped exactly once.
    for creation in collect::<XamlAstNewClrObjectNode>(&root) {
        let wrappers: Vec<Rc<XamlValueWithManipulationNode>> =
            collect::<XamlValueWithManipulationNode>(&root)
                .into_iter()
                .filter(|w| {
                    w.value().same_node(&creation)
                        && w.manipulation()
                            .is_some_and(|m| m.is::<XamlSourceInfoValueManipulation>())
                })
                .collect();
        assert_eq!(wrappers.len(), 1);
    }
}
