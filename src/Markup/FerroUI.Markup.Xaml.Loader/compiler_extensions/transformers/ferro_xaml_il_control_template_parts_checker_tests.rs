//! Tests of `FerroXamlIlControlTemplatePartsChecker` and
//! `FerroXamlIlControlTemplatePriorityTransformer`, on hand-built trees (the nodes these
//! transformers look at are produced late in the pipeline).

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstManipulationNode, IXamlAstNode, IXamlAstValueNode, IXamlPropertySetter,
    XamlAstClrProperty, XamlAstClrTypeReference, XamlAstNodeExtensions, XamlAstObjectNode,
    XamlAstTextNode, XamlConstantNode, XamlLineInfo, XamlPropertyAssignmentNode,
};
use xamlx::diagnostics::XamlDiagnosticSeverity;
use xamlx::type_system::{IXamlType, XamlValue};

use super::*;
use crate::compiler_extensions::{
    BindingSetter, BindingWithPrioritySetter, FerroXamlDiagnosticCodes, SetValueWithPrioritySetter,
    UnsetValueSetter, XamlIlFerroProperty,
};
use crate::testing::styles::create_styles_test_framework;
use crate::testing::TestFramework;

fn object(fw: &TestFramework, type_name: &str, line: i32) -> Rc<XamlAstObjectNode> {
    let line = XamlLineInfo::new(line, 1);
    XamlAstObjectNode::new(&line, XamlAstClrTypeReference::new(&line, fw.t(type_name), false))
}

/// A control template for `target` whose content registers `names` (name, type, line).
fn template(
    fw: &TestFramework,
    value_type: &str,
    target: &str,
    scope: ScopeTypes,
    names: &[(&str, &str, i32)],
) -> Rc<dyn IXamlAstNode> {
    let content = object(fw, "FerroUI.Controls.Border", 2);
    for (name, type_name, line) in names {
        let line = XamlLineInfo::new(*line, 7);
        let text: Rc<dyn IXamlAstValueNode> = XamlAstTextNode::new(&line, name, true);
        let registration: Rc<dyn IXamlAstNode> =
            FerroNameScopeRegistrationXamlIlNode::new(text, Some(fw.t(type_name)));
        content.children.borrow_mut().push(registration);
    }
    let template = object(fw, value_type, 1);
    let content_value: Rc<dyn IXamlAstValueNode> = content;
    let nested: Rc<dyn IXamlAstNode> = NestedScopeMetadataNode::new(content_value);
    template.children.borrow_mut().push(nested);
    let template_value: Rc<dyn IXamlAstValueNode> = template;
    let line = XamlLineInfo::new(1, 1);
    FerroXamlIlTargetTypeMetadataNode::new(
        template_value,
        XamlAstClrTypeReference::new(&line, fw.t(target), false),
        scope,
    )
}

fn check_parts(fw: &TestFramework, root: Rc<dyn IXamlAstNode>) -> Vec<(String, XamlDiagnosticSeverity, String, Option<i32>)> {
    let context = fw.create_context();
    let result = context
        .visit(&root, &FerroXamlIlControlTemplatePartsChecker)
        .expect("errors are not fatal in the fixture");
    assert!(result.same_node(&root));
    fw.reported_diagnostics()
        .into_iter()
        .map(|d| (d.code, d.severity, d.title, d.line_number))
        .collect()
}

const CONTROL_TEMPLATE: &str = "FerroUI.Markup.Xaml.Templates.ControlTemplate";

#[test]
fn template_parts_are_checked_against_the_registered_names() {
    // Everything present with the right types (a derived type is fine)
    let fw = create_styles_test_framework();
    let root = template(
        &fw,
        CONTROL_TEMPLATE,
        "FerroUI.Controls.TextBox",
        ScopeTypes::ControlTemplate,
        &[
            ("PART_TextPresenter", "FerroUI.Controls.Presenters.TextPresenter", 3),
            ("PART_ScrollViewer", "FerroUI.Controls.ScrollViewer", 4),
            ("PART_ClearButton", "FerroUI.Controls.Button", 5),
            ("other", "FerroUI.Controls.Border", 6),
        ],
    );
    assert_eq!(check_parts(&fw, root), []);

    // Required part missing, optional parts missing
    let fw = create_styles_test_framework();
    let root = template(&fw, CONTROL_TEMPLATE, "FerroUI.Controls.TextBox", ScopeTypes::ControlTemplate, &[]);
    assert_eq!(
        check_parts(&fw, root),
        [
            (
                FerroXamlDiagnosticCodes::REQUIRED_TEMPLATE_PART_MISSING.to_string(),
                XamlDiagnosticSeverity::Error,
                "Required template part with x:Name 'PART_TextPresenter' must be defined on 'TextBox' ControlTemplate.".to_string(),
                Some(1)
            ),
            (
                FerroXamlDiagnosticCodes::OPTIONAL_TEMPLATE_PART_MISSING.to_string(),
                XamlDiagnosticSeverity::None,
                "Optional template part with x:Name 'PART_ScrollViewer' can be defined on 'TextBox' ControlTemplate.".to_string(),
                Some(1)
            ),
            (
                FerroXamlDiagnosticCodes::OPTIONAL_TEMPLATE_PART_MISSING.to_string(),
                XamlDiagnosticSeverity::None,
                "Optional template part with x:Name 'PART_ClearButton' can be defined on 'TextBox' ControlTemplate.".to_string(),
                Some(1)
            ),
        ]
    );

    // Wrong types are reported at the name
    let fw = create_styles_test_framework();
    let root = template(
        &fw,
        CONTROL_TEMPLATE,
        "FerroUI.Controls.TextBox",
        ScopeTypes::ControlTemplate,
        &[
            ("PART_TextPresenter", "FerroUI.Controls.Border", 3),
            ("PART_ScrollViewer", "FerroUI.Controls.ScrollViewer", 4),
            ("PART_ClearButton", "FerroUI.Controls.ContentControl", 5),
        ],
    );
    assert_eq!(
        check_parts(&fw, root),
        [
            (
                FerroXamlDiagnosticCodes::TEMPLATE_PART_WRONG_TYPE.to_string(),
                XamlDiagnosticSeverity::Error,
                "Template part 'PART_TextPresenter' is expected to be assignable to 'TextPresenter', but actual type is Border.".to_string(),
                Some(3)
            ),
            (
                FerroXamlDiagnosticCodes::TEMPLATE_PART_WRONG_TYPE.to_string(),
                XamlDiagnosticSeverity::Error,
                "Template part 'PART_ClearButton' is expected to be assignable to 'Button', but actual type is ContentControl.".to_string(),
                Some(5)
            ),
        ]
    );
}

#[test]
fn template_parts_of_a_derived_type_override_the_base_type_parts() {
    let fw = create_styles_test_framework();
    let derived = fw.controls.define_class("FerroUI.Controls", "MaskedTextBox");
    derived.set_base_type(fw.t("FerroUI.Controls.TextBox"));
    // Overrides the required presenter part with an optional, untyped one
    derived.add_attribute(xamlx::testing::FakeCustomAttribute::with_properties(
        fw.t("FerroUI.Controls.Metadata.TemplatePartAttribute"),
        vec![],
        vec![("Name", XamlValue::String("PART_TextPresenter".to_string()))],
    ));
    // Attributes without a name are ignored
    derived.add_attribute(xamlx::testing::FakeCustomAttribute::new(
        fw.t("FerroUI.Controls.Metadata.TemplatePartAttribute"),
        vec![XamlValue::String(String::new())],
    ));
    let root = template(
        &fw,
        CONTROL_TEMPLATE,
        "FerroUI.Controls.MaskedTextBox",
        ScopeTypes::ControlTemplate,
        &[("PART_TextPresenter", "FerroUI.Controls.Border", 3)],
    );
    let diagnostics = check_parts(&fw, root);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert!(diagnostics.iter().all(|d| d.1 == XamlDiagnosticSeverity::None));
    assert!(diagnostics[0].2.contains("'PART_ScrollViewer' can be defined on 'MaskedTextBox'"));
}

#[test]
fn only_control_templates_with_template_parts_are_checked() {
    // The target type has no template parts
    let fw = create_styles_test_framework();
    let root = template(&fw, CONTROL_TEMPLATE, "FerroUI.Controls.Button", ScopeTypes::ControlTemplate, &[]);
    assert_eq!(check_parts(&fw, root), []);

    // A style with a template selector has a control template scope, too
    let fw = create_styles_test_framework();
    let root = template(&fw, "FerroUI.Styling.Style", "FerroUI.Controls.TextBox", ScopeTypes::ControlTemplate, &[]);
    assert_eq!(check_parts(&fw, root), []);

    // Another scope type
    let fw = create_styles_test_framework();
    let root = template(&fw, CONTROL_TEMPLATE, "FerroUI.Controls.TextBox", ScopeTypes::Style, &[]);
    assert_eq!(check_parts(&fw, root), []);

    // Names registered outside of the template's own name scope don't count
    let fw = create_styles_test_framework();
    let scope = template(&fw, CONTROL_TEMPLATE, "FerroUI.Controls.TextBox", ScopeTypes::ControlTemplate, &[]);
    let line = XamlLineInfo::new(9, 9);
    let text: Rc<dyn IXamlAstValueNode> = XamlAstTextNode::new(&line, "PART_TextPresenter", true);
    let registration: Rc<dyn IXamlAstNode> = FerroNameScopeRegistrationXamlIlNode::new(
        text,
        Some(fw.t("FerroUI.Controls.Presenters.TextPresenter")),
    );
    let template_object = scope
        .cast::<FerroXamlIlTargetTypeMetadataNode>()
        .and_then(|s| s.value().cast::<XamlAstObjectNode>())
        .expect("template");
    template_object.children.borrow_mut().push(registration);
    let diagnostics = check_parts(&fw, scope);
    assert_eq!(diagnostics[0].0, FerroXamlDiagnosticCodes::REQUIRED_TEMPLATE_PART_MISSING);
}

/// `Border.Background` as a registered property node and an assignment of `value_count`
/// values to it with the given possible setters, inside (or not) a control template scope.
struct Assignment {
    fw: TestFramework,
    property: Rc<XamlAstClrProperty>,
    assignment: Rc<XamlPropertyAssignmentNode>,
    root: Rc<dyn IXamlAstNode>,
}

fn assignment(
    in_template: bool,
    registered: bool,
    value_count: usize,
    pick: impl Fn(&[Rc<dyn IXamlPropertySetter>]) -> Vec<Rc<dyn IXamlPropertySetter>>,
) -> Assignment {
    let fw = create_styles_test_framework();
    let line = XamlLineInfo::new(1, 1);
    let border = fw.t("FerroUI.Controls.Border");
    let clr = border
        .properties()
        .into_iter()
        .find(|p| p.name() == "Background")
        .expect("Background");
    let original = XamlAstClrProperty::from_property(&line, &clr, &fw.configuration).expect("property");
    let field = border
        .fields()
        .into_iter()
        .find(|f| f.name() == "BackgroundProperty")
        .expect("field");
    let property = if registered {
        XamlIlFerroProperty::new(&original, field, &fw.types).expect("registered property")
    } else {
        original
    };
    let values: Vec<Rc<dyn IXamlAstValueNode>> = (0..value_count)
        .map(|_| {
            let value: Rc<dyn IXamlAstValueNode> = XamlAstTextNode::new(&line, "Red", true);
            value
        })
        .collect();
    let assignment =
        XamlPropertyAssignmentNode::new(&line, property.clone(), pick(&property.setters()), values);
    let owner = object(&fw, "FerroUI.Controls.Border", 1);
    let manipulation: Rc<dyn IXamlAstManipulationNode> = assignment.clone();
    let child: Rc<dyn IXamlAstNode> = manipulation;
    owner.children.borrow_mut().push(child);
    let owner_value: Rc<dyn IXamlAstValueNode> = owner;
    let scope_type = if in_template { ScopeTypes::ControlTemplate } else { ScopeTypes::Style };
    let root: Rc<dyn IXamlAstNode> = FerroXamlIlTargetTypeMetadataNode::new(
        owner_value,
        XamlAstClrTypeReference::new(&line, fw.t("FerroUI.Controls.Button"), false),
        scope_type,
    );
    let context = fw.create_context();
    context
        .visit(&root, &FerroXamlIlControlTemplatePriorityTransformer)
        .expect("transformed");
    Assignment {
        fw,
        property,
        assignment,
        root,
    }
}

fn is_value_setter(s: &Rc<dyn IXamlPropertySetter>) -> bool {
    s.parameters().len() == 1 && s.parameters()[0].name() == "IBrush"
}

fn is_binding_setter(s: &Rc<dyn IXamlPropertySetter>) -> bool {
    s.as_any().is::<BindingSetter>()
}

#[test]
fn assignments_in_control_templates_use_the_priority_setters() {
    let a = assignment(true, true, 1, |setters| {
        setters
            .iter()
            .filter(|s| is_value_setter(s) || is_binding_setter(s) || s.as_any().is::<UnsetValueSetter>())
            .cloned()
            .collect()
    });
    assert_eq!(a.fw.reported_diagnostics().len(), 0);
    assert!(a.root.is::<FerroXamlIlTargetTypeMetadataNode>());
    // The unset value setter has no priority counterpart and is dropped; the two others are
    // replaced, in the order of the possible setters they stand for.
    let unset_index = a.property.setters().iter().position(|s| s.as_any().is::<UnsetValueSetter>());
    assert_eq!(unset_index, Some(0));
    let setters = a.assignment.possible_setters.borrow().clone();
    assert_eq!(setters.len(), 2);
    assert!(setters.iter().any(|s| s.as_any().is::<SetValueWithPrioritySetter>()));
    assert!(setters.iter().any(|s| s.as_any().is::<BindingWithPrioritySetter>()));
    for setter in &setters {
        let parameters: Vec<Rc<dyn IXamlType>> = setter.parameters();
        assert_eq!(parameters.len(), 2);
        assert_eq!(parameters[0].name(), "BindingPriority");
    }
    // The priority is inserted in front of the value: BindingPriority.Template
    let values = a.assignment.values.borrow().clone();
    assert_eq!(values.len(), 2);
    let priority = values[0].cast::<XamlConstantNode>().expect("constant");
    assert!(matches!(priority.constant, XamlValue::Int32(2)));
    assert_eq!(
        xamlx::ast::XamlAstExtensions::get_clr_type(&priority.type_()).expect("type").name(),
        "BindingPriority"
    );
    assert!(values[1].is::<XamlAstTextNode>());
}

#[test]
fn other_assignments_are_left_alone() {
    let all = |setters: &[Rc<dyn IXamlPropertySetter>]| setters.to_vec();
    // Not in a control template
    let a = assignment(false, true, 1, all);
    assert_eq!(a.assignment.values.borrow().len(), 1);
    assert_eq!(a.assignment.possible_setters.borrow().len(), a.property.setters().len());
    // Not a registered property
    let a = assignment(true, false, 1, all);
    assert_eq!(a.assignment.values.borrow().len(), 1);
    // More than one value
    let a = assignment(true, true, 2, all);
    assert_eq!(a.assignment.values.borrow().len(), 2);
    assert_eq!(a.assignment.possible_setters.borrow().len(), a.property.setters().len());
    // No possible setter has a priority counterpart
    let a = assignment(true, true, 1, |setters| {
        setters.iter().filter(|s| s.as_any().is::<UnsetValueSetter>()).cloned().collect()
    });
    assert_eq!(a.assignment.values.borrow().len(), 1);
    assert_eq!(a.assignment.possible_setters.borrow().len(), 1);
    assert_eq!(a.fw.reported_diagnostics().len(), 0);
}
