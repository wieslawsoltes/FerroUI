//! Tests of `FerroXamlIlSelectorTransformer` and the selector nodes. The selector shapes come
//! from the upstream `StyleTests` and `SelectorGrammarTests`.

use std::rc::Rc;

use xamlx::ast::{IXamlAstNode, IXamlAstValueNode, XamlAstExtensions, XamlAstNodeExtensions, XamlAstObjectNode};
use xamlx::diagnostics::XamlDiagnosticSeverity;
use xamlx::transform::IXamlAstTransformer;

use super::*;
use crate::compiler_extensions::FerroXamlDiagnosticCodes;
use crate::testing::styles::{
    create_styles_test_framework, describe_selector, find_nodes, run_transformers, xmlns,
};
use crate::testing::TestFramework;

fn pipeline() -> Vec<Box<dyn IXamlAstTransformer>> {
    vec![Box::new(FerroXamlIlSelectorTransformer)]
}

fn run(fw: &TestFramework, xaml: &str) -> Rc<dyn IXamlAstNode> {
    run_transformers(fw, xaml, pipeline()).expect("errors are not fatal in the fixture")
}

fn style(fw: &TestFramework, selector: &str) -> Rc<dyn IXamlAstNode> {
    run(fw, &format!("<Style {} Selector=\"{selector}\"/>", xmlns()))
}

/// The description of the only selector in the tree.
fn selector_of(root: &Rc<dyn IXamlAstNode>) -> Rc<dyn XamlIlSelectorNode> {
    let scopes = find_nodes::<XamlAstObjectNode>(root);
    let style = scopes.first().expect("a style");
    let selectors: Vec<Rc<dyn XamlIlSelectorNode>> = style
        .children
        .borrow()
        .iter()
        .flat_map(|c| find_nodes::<dyn XamlIlSelectorNode>(c))
        .collect();
    assert_eq!(selectors.len(), 1);
    selectors[0].clone()
}

fn scopes(root: &Rc<dyn IXamlAstNode>) -> Vec<String> {
    find_nodes::<FerroXamlIlTargetTypeMetadataNode>(root)
        .iter()
        .map(|s| {
            let target = get_nullable_clr_type(&s.target_type()).expect("clr type");
            format!(
                "{}:{}",
                s.scope_type.name(),
                target.map(|t| t.name()).unwrap_or_else(|| "<null>".to_string())
            )
        })
        .collect()
}

fn check(selector: &str, expected: &str, expected_scopes: &[&str]) {
    let fw = create_styles_test_framework();
    let root = style(&fw, selector);
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    assert_eq!(describe_selector(&selector_of(&root)), expected, "{selector}");
    assert_eq!(scopes(&root), expected_scopes, "{selector}");
}

fn check_error(xaml: &str, code: &str, message: &str) {
    let fw = create_styles_test_framework();
    let _ = run(&fw, xaml);
    let diagnostics = fw.reported_diagnostics();
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, code, "{diagnostics:?}");
    assert_eq!(diagnostics[0].severity, XamlDiagnosticSeverity::Error);
    assert!(
        diagnostics[0].title.starts_with(message),
        "{:?} doesn't start with {message:?}",
        diagnostics[0].title
    );
}

fn selector_error(selector: &str, code: &str, message: &str) {
    check_error(&format!("<Style {} Selector=\"{selector}\"/>", xmlns()), code, message);
}

#[test]
fn type_class_and_name_selectors() {
    check("Button", "OfType(Button)", &["Style:Button"]);
    check(":is(Control)", "Is(Control)", &["Style:Control"]);
    check("Button.foo", "OfType(Button).Class(foo)", &["Style:Button"]);
    check(
        "Button.foo.bar:pointerover#name",
        "OfType(Button).Class(foo).Class(bar).Class(:pointerover).Name(name)",
        &["Style:Button"],
    );
    // No target type at all: the scope's target type is null.
    check(".foo", "Class(foo)", &["Style:<null>"]);
    check("#name", "Name(name)", &["Style:<null>"]);
    check(":pointerover", "Class(:pointerover)", &["Style:<null>"]);
}

#[test]
fn combinators() {
    check(
        "Button > TextBlock",
        "OfType(Button).Child().OfType(TextBlock)",
        &["Style:TextBlock"],
    );
    check(
        "Button TextBlock.foo",
        "OfType(Button).Descendant().OfType(TextBlock).Class(foo)",
        &["Style:TextBlock"],
    );
    // The steps after a combinator start without a target type.
    check("Button > .foo", "OfType(Button).Child().Class(foo)", &["Style:<null>"]);
}

#[test]
fn template_combinator_adds_a_control_template_scope_for_the_templated_type() {
    check(
        "Button /template/ Border",
        "OfType(Button).Template().OfType(Border)",
        &["ControlTemplate:Button", "Style:Border"],
    );
    // The last template combinator wins.
    check(
        "Button /template/ ContentControl /template/ Border#x",
        "OfType(Button).Template().OfType(ContentControl).Template().OfType(Border).Name(x)",
        &["ControlTemplate:ContentControl", "Style:Border"],
    );
    // Without a type in front of the combinator there is no template scope.
    check(
        ".foo /template/ Border",
        "Class(foo).Template().OfType(Border)",
        &["Style:Border"],
    );
}

#[test]
fn not_and_nth_child() {
    check(
        "Button:not(.foo)",
        "OfType(Button).Not(Class(foo))",
        &["Style:Button"],
    );
    check(
        "Button:not(TextBlock.a)",
        "OfType(Button).Not(OfType(TextBlock).Class(a))",
        &["Style:Button"],
    );
    check(
        "Button:nth-child(2n+1)",
        "OfType(Button).NthChild(2,1)",
        &["Style:Button"],
    );
    check(
        "Button:nth-last-child(3)",
        "OfType(Button).NthLastChild(0,3)",
        &["Style:Button"],
    );
}

#[test]
fn property_selectors_convert_the_value_to_the_property_type() {
    check(
        "Button[IsEnabled=true]",
        "OfType(Button).PropertyEquals(IsEnabled=true)",
        &["Style:Button"],
    );
    check(
        "TextBlock[Text=foo bar]",
        "OfType(TextBlock).PropertyEquals(Text=\"foo bar\")",
        &["Style:TextBlock"],
    );
    check(
        "Button[(Grid.Row)=1]",
        "OfType(Button).PropertyEquals(Grid.RowProperty=1i32)",
        &["Style:Button"],
    );
    check(
        "Button[Width=20]",
        "OfType(Button).PropertyEquals(Width=20f64)",
        &["Style:Button"],
    );
}

#[test]
fn or_selector_targets_the_common_base_type() {
    check(
        "Button, TextBlock.foo",
        "Or[OfType(Button), OfType(TextBlock).Class(foo)]",
        &["Style:Control"],
    );
    check(
        "Button, ContentControl",
        "Or[OfType(Button), OfType(ContentControl)]",
        &["Style:ContentControl"],
    );
    check("Button, .foo", "Or[OfType(Button), Class(foo)]", &["Style:<null>"]);
    // An or-node has no previous node, so a template combinator inside it adds no scope.
    check(
        "Button /template/ Border, TextBlock",
        "Or[OfType(Button).Template().OfType(Border), OfType(TextBlock)]",
        &["Style:Control"],
    );
}

#[test]
fn nesting_selector_takes_the_parent_scope_target_type() {
    let fw = create_styles_test_framework();
    let root = run(
        &fw,
        &format!(
            "<Style {} Selector='Button'><Style Selector='^:pointerover'/><Style Selector='^ > TextBlock'/></Style>",
            xmlns()
        ),
    );
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    let selectors: Vec<String> = find_nodes::<dyn XamlIlSelectorNode>(&root)
        .iter()
        .map(describe_selector)
        .collect();
    assert_eq!(
        selectors,
        ["OfType(Button)", "Nesting().Class(:pointerover)", "Nesting().Child().OfType(TextBlock)"]
    );
    assert_eq!(scopes(&root), ["Style:Button", "Style:Button", "Style:TextBlock"]);

    selector_error(
        "^:pointerover",
        FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
        "Cannot find parent style for nested selector.",
    );
}

#[test]
fn selector_errors() {
    let fw = create_styles_test_framework();
    let _ = style(&fw, "Button.");
    let diagnostics = fw.reported_diagnostics();
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, FerroXamlDiagnosticCodes::SELECTORS_TRANSFORM_ERROR);
    assert!(diagnostics[0].title.starts_with("Unable to parse selector: "), "{:?}", diagnostics[0].title);
    let inner = diagnostics[0].inner_exception.as_deref().expect("the exception");
    assert!(XamlSelectorsTransformException::is(inner));
    assert_eq!(inner.inner_exception().map(|e| e.type_name()), Some("ExpressionParseException"));

    selector_error(
        ".foo[IsEnabled=true]",
        FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
        "Property selectors must be applied to a type.",
    );
    selector_error(
        "Button[Missing=1]",
        FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
        "Cannot find 'Missing' on '",
    );
    selector_error(
        "Button[Template=abc]",
        FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
        "Cannot convert 'abc' to 'FerroUI.Controls:FerroUI.Controls.Templates.IControlTemplate",
    );
    // As upstream, a primitive that fails to parse is reported by the value conversion itself.
    selector_error(
        "Button[IsEnabled=maybe]",
        FerroXamlDiagnosticCodes::PARSE_ERROR,
        "String 'maybe' was not recognized as a valid Boolean.",
    );
    selector_error(
        ".foo[(Grid.Row)=1]",
        FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
        "Attached Property selectors must be applied to a type.",
    );
    selector_error(
        "Button[(Grid.Missing)=1]",
        FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
        "Cannot find 'Missing' on 'FerroUI.Controls:FerroUI.Controls.Grid",
    );
    // Width is a styled property, not an attached one.
    selector_error(
        "Button[(Button.Width)=1]",
        FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
        "Cannot find 'Width' on 'FerroUI.Controls:FerroUI.Controls.Button",
    );
    selector_error(
        "Missing",
        FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
        "Unable to resolve type Missing from namespace",
    );
    selector_error(
        "missing|Button",
        FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
        "Unable to resolve type namespace alias missing",
    );
}

#[test]
fn selector_value_must_be_a_single_text_node() {
    check_error(
        &format!("<Style {}><Style.Selector><Button/></Style.Selector></Style>", xmlns()),
        FerroXamlDiagnosticCodes::SELECTORS_TRANSFORM_ERROR,
        "Selector property should be a text node",
    );
    check_error(
        &format!(
            "<Style {}><Style.Selector><Button/><Button/></Style.Selector></Style>",
            xmlns()
        ),
        FerroXamlDiagnosticCodes::SELECTORS_TRANSFORM_ERROR,
        "Selector property should have exactly one value",
    );
}

#[test]
fn xmlns_prefixed_types_are_resolved() {
    let fw = create_styles_test_framework();
    let root = run(
        &fw,
        &format!(
            "<Style {} xmlns:c='clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls' Selector='c|Button:is(c|Control)'/>",
            xmlns()
        ),
    );
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    assert_eq!(describe_selector(&selector_of(&root)), "OfType(Button).Is(Control)");
}

#[test]
fn style_without_selector_takes_the_type_of_the_parent_element() {
    // Directly in a styled element
    let fw = create_styles_test_framework();
    let root = run(&fw, &format!("<Border {}><Style/></Border>", xmlns()));
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    assert_eq!(scopes(&root), ["Style:Border"]);

    // A `Styles` collection is skipped when looking for the parent
    let fw = create_styles_test_framework();
    let root = run(&fw, &format!("<Border {}><Styles><Style/></Styles></Border>", xmlns()));
    assert_eq!(scopes(&root), ["Style:Border"]);

    // An empty selector behaves the same
    let fw = create_styles_test_framework();
    let root = run(&fw, &format!("<Border {}><Style Selector=''/></Border>", xmlns()));
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    assert_eq!(scopes(&root), ["Style:Border"]);

    // No styled element parent: the style is left alone
    for xaml in [
        format!("<Style {}/>", xmlns()),
        format!("<Styles {}><Style/></Styles>", xmlns()),
        format!("<Style {} Selector=''/>", xmlns()),
        format!("<Style {} Selector='Button'><Style/></Style>", xmlns()),
    ] {
        let fw = create_styles_test_framework();
        let root = run(&fw, &xaml);
        assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
        let expected: &[&str] = if xaml.contains("Button") { &["Style:Button"] } else { &[] };
        assert_eq!(scopes(&root), expected, "{xaml}");
    }

    // Not allowed in a control theme
    check_error(
        &format!("<ControlTheme {} TargetType='Button'><Style/></ControlTheme>", xmlns()),
        FerroXamlDiagnosticCodes::TRANSFORM_ERROR,
        "Cannot add a Style without selector to a ControlTheme.",
    );
}

#[test]
fn transformer_is_idempotent() {
    let fw = create_styles_test_framework();
    let xaml = format!(
        "<Border {}><Style/><Style Selector='Button /template/ Border'/></Border>",
        xmlns()
    );
    let root = run_transformers(
        &fw,
        &xaml,
        vec![
            Box::new(FerroXamlIlSelectorTransformer),
            Box::new(FerroXamlIlSelectorTransformer),
        ],
    )
    .expect("transformed");
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    assert_eq!(
        scopes(&root),
        ["Style:Border", "ControlTemplate:Button", "Style:Border"]
    );
}

#[test]
fn selector_nodes_are_typed_as_the_selector_type_and_find_their_builder_methods() {
    let fw = create_styles_test_framework();
    let root = style(
        &fw,
        "Button[IsEnabled=true].foo#bar:not(.x):nth-child(2) > :is(Control)[(Grid.Row)=1] /template/ Border, TextBlock",
    );
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    let or = selector_of(&root);
    assert!(or.previous().is_none());
    assert_eq!(or.type_().get_clr_type().expect("type").full_name(), "FerroUI.Styling.Selector");
    let value: Rc<dyn IXamlAstValueNode> = or.clone();
    assert!(value.is::<dyn XamlIlSelectorNode>());

    let or_node = or.as_any().downcast_ref::<XamlIlOrSelectorNode>().expect("or node");
    let or_method = or_node.builder_method(&fw.types).expect("Or");
    assert!(or_method.parameters()[0].name().starts_with("IReadOnlyList"));

    let mut seen = Vec::new();
    let mut current = Some(or_node.selectors()[0].clone());
    while let Some(node) = current {
        assert_eq!(node.type_().get_clr_type().expect("type").full_name(), "FerroUI.Styling.Selector");
        let any = node.as_any();
        let method = if let Some(n) = any.downcast_ref::<XamlIlTypeSelector>() {
            Some(n.builder_method(&fw.types))
        } else if let Some(n) = any.downcast_ref::<XamlIlStringSelector>() {
            Some(n.builder_method(&fw.types))
        } else if let Some(n) = any.downcast_ref::<XamlIlCombinatorSelector>() {
            Some(n.builder_method(&fw.types))
        } else if let Some(n) = any.downcast_ref::<XamlIlNotSelector>() {
            let method = n.builder_method(&fw.types).expect("Not");
            // Not the overload taking a function
            assert_eq!(method.parameters()[1].name(), "Selector");
            Some(Ok(method))
        } else if let Some(n) = any.downcast_ref::<XamlIlNthChildSelector>() {
            Some(n.builder_method(&fw.types))
        } else if let Some(n) = any.downcast_ref::<XamlIlPropertyEqualsSelector>() {
            assert_eq!(n.resolve_ferro_property_field().expect("field").name(), "IsEnabledProperty");
            let method = n.builder_method(&fw.types).expect("PropertyEquals");
            // Not the generic overload
            assert!(!method.is_generic_method_definition());
            Some(Ok(method))
        } else if let Some(n) = any.downcast_ref::<XamlIlAttachedPropertyEqualsSelector>() {
            Some(n.builder_method(&fw.types))
        } else {
            assert!(any.is::<XamlIlSelectorInitialNode>());
            assert!(node.target_type().is_none());
            None
        };
        if let Some(method) = method {
            let method = method.expect("builder method");
            assert!(method.is_static());
            seen.push(format!("{}/{}", method.name(), method.parameters().len()));
        }
        current = node.previous();
    }
    seen.reverse();
    assert_eq!(
        seen,
        [
            "OfType/2", "PropertyEquals/3", "Class/2", "Name/2", "Not/2", "NthChild/3", "Child/1",
            "Is/2", "PropertyEquals/3", "Template/1", "OfType/2"
        ]
    );

    // A property without a registered property field cannot be emitted
    let classes = fw
        .t("FerroUI.StyledElement")
        .properties()
        .into_iter()
        .find(|p| p.name() == "Classes")
        .expect("Classes");
    let initial: Rc<dyn XamlIlSelectorNode> =
        XamlIlSelectorInitialNode::new(&*root, fw.t("FerroUI.Styling.Selector"));
    let value: Rc<dyn IXamlAstValueNode> = or;
    let selector = XamlIlPropertyEqualsSelector::new(initial.clone(), classes, value);
    let error = selector.resolve_ferro_property_field().err().expect("no field");
    assert!(error.message().starts_with(
        "Classes of FerroUI.Base:FerroUI.StyledElement doesn't seem to be an FerroProperty"
    ));

    // A missing builder method is a type system error
    let nesting = XamlIlNestingSelector::new(initial, None);
    assert!(nesting.builder_method(&fw.types).is_ok());
    let empty = crate::testing::create_test_framework();
    if empty.t("FerroUI.Styling.Selectors").methods().is_empty() {
        assert!(nesting.builder_method(&empty.types).err().expect("error").is_type_system_exception());
    }
}

#[test]
fn nullable_type_references_round_trip() {
    let fw = create_styles_test_framework();
    let root = style(&fw, "Button");
    let reference = create_nullable_clr_type_reference(&*root, None);
    let reference: Rc<dyn xamlx::ast::IXamlAstTypeReference> = reference;
    assert!(get_nullable_clr_type(&reference).expect("clr").is_none());
    let reference: Rc<dyn xamlx::ast::IXamlAstTypeReference> =
        create_nullable_clr_type_reference(&*root, Some(fw.t("FerroUI.Controls.Button")));
    assert_eq!(
        get_nullable_clr_type(&reference).expect("clr").map(|t| t.name()),
        Some("Button".to_string())
    );
}
