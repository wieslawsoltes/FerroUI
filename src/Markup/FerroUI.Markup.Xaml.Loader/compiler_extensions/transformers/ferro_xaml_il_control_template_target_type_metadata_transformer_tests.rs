//! Tests of `FerroXamlIlControlTemplateTargetTypeMetadataTransformer` and
//! `FerroXamlIlTargetTypeMetadataNode`. The XAML shapes come from the upstream
//! `ControlTemplateTests`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstNode, IXamlAstValueNode, IXamlLineInfo, XamlAstExtensions, XamlAstNodeExtensions,
    XamlAstObjectNode,
};
use xamlx::transform::IXamlAstTransformer;

use super::*;
use crate::testing::styles::{create_styles_test_framework, find_nodes, run_transformers, xmlns};
use crate::testing::TestFramework;

fn run(fw: &TestFramework, xaml: &str) -> Rc<dyn IXamlAstNode> {
    let pipeline: Vec<Box<dyn IXamlAstTransformer>> = vec![
        Box::new(FerroXamlIlSelectorTransformer),
        Box::new(FerroXamlIlControlTemplateTargetTypeMetadataTransformer),
        Box::new(FerroXamlIlControlTemplateTargetTypeMetadataTransformer),
    ];
    run_transformers(fw, xaml, pipeline).expect("errors are not fatal in the fixture")
}

/// `scope:target(value type)` of every scope in the tree.
fn scopes(xaml: &str) -> Vec<String> {
    let fw = create_styles_test_framework();
    let root = run(&fw, xaml);
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    find_nodes::<FerroXamlIlTargetTypeMetadataNode>(&root)
        .iter()
        .map(|s| {
            format!(
                "{}:{}({})",
                s.scope_type.name(),
                s.target_type().get_clr_type().expect("clr").name(),
                s.type_().get_clr_type().expect("clr").name()
            )
        })
        .collect()
}

#[test]
fn target_type_property_as_text_or_type_extension() {
    assert_eq!(
        scopes(&format!("<ControlTemplate {} TargetType='Button'/>", xmlns())),
        ["ControlTemplate:Button(ControlTemplate)"]
    );
    assert_eq!(
        scopes(&format!("<ControlTemplate {} TargetType='{{x:Type TextBlock}}'/>", xmlns())),
        ["ControlTemplate:TextBlock(ControlTemplate)"]
    );
}

#[test]
fn target_type_falls_back_to_the_style_scope_then_the_parent_control_then_control() {
    // The nearest scope is a style scope
    assert_eq!(
        scopes(&format!(
            "<Style {} Selector='Button'><Setter Property='Template'><ControlTemplate/></Setter></Style>",
            xmlns()
        )),
        ["Style:Button(Style)", "ControlTemplate:Button(ControlTemplate)"]
    );
    // The object the template is assigned to is a control
    assert_eq!(
        scopes(&format!(
            "<Button {}><Button.Template><ControlTemplate/></Button.Template></Button>",
            xmlns()
        )),
        ["ControlTemplate:Button(ControlTemplate)"]
    );
    // Neither
    assert_eq!(
        scopes(&format!("<ControlTemplate {}/>", xmlns())),
        ["ControlTemplate:Control(ControlTemplate)"]
    );
    assert_eq!(
        scopes(&format!(
            "<ResourceDictionary {}><ControlTemplate x:Key='a'/></ResourceDictionary>",
            xmlns()
        )),
        ["ControlTemplate:Control(ControlTemplate)"]
    );
    // A nested template: the nearest scope is a control template scope, not a style scope,
    // and the direct parent is not a control
    assert_eq!(
        scopes(&format!(
            "<ControlTemplate {} TargetType='Button'><Border><Border.Tag><ControlTemplate/></Border.Tag></Border></ControlTemplate>",
            xmlns()
        )),
        [
            "ControlTemplate:Button(ControlTemplate)",
            "ControlTemplate:Border(ControlTemplate)"
        ]
    );
}

#[test]
fn only_control_template_scope_types_are_wrapped() {
    assert!(scopes(&format!("<DataTemplate {}/>", xmlns())).is_empty());
    assert!(scopes(&format!("<Template {}/>", xmlns())).is_empty());
    assert!(scopes(&format!("<Button {}/>", xmlns())).is_empty());
}

#[test]
fn scope_attribute_is_found_on_base_types_and_direct_interfaces() {
    let fw = create_styles_test_framework();
    // A class deriving from ControlTemplate inherits the scope through its base type's interface
    // only when the attribute is on a base type: upstream looks at the type's own interfaces.
    let derived = fw.markup_xaml.define_class("FerroUI.Markup.Xaml.Templates", "DerivedTemplate");
    derived.set_base_type(fw.t("FerroUI.Markup.Xaml.Templates.ControlTemplate"));
    derived.add_constructor(vec![]);
    let attributed = fw.markup_xaml.define_class("FerroUI.Markup.Xaml.Templates", "AttributedTemplate");
    attributed.add_constructor(vec![]);
    attributed.add_attribute(xamlx::testing::FakeCustomAttribute::new(
        fw.t("FerroUI.Metadata.ControlTemplateScopeAttribute"),
        vec![],
    ));
    let derived_attributed =
        fw.markup_xaml.define_class("FerroUI.Markup.Xaml.Templates", "DerivedAttributedTemplate");
    derived_attributed.set_base_type(attributed.as_type());
    derived_attributed.add_constructor(vec![]);

    let root = run(
        &fw,
        &format!(
            "<Border {}><AttributedTemplate/><DerivedAttributedTemplate/><DerivedTemplate/></Border>",
            xmlns()
        ),
    );
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    let wrapped: Vec<String> = find_nodes::<FerroXamlIlTargetTypeMetadataNode>(&root)
        .iter()
        .map(|s| s.type_().get_clr_type().expect("clr").name())
        .collect();
    let inherits_interfaces = fw
        .t("FerroUI.Markup.Xaml.Templates.DerivedTemplate")
        .interfaces()
        .iter()
        .any(|i| i.name() == "IControlTemplate");
    let mut expected = vec!["AttributedTemplate", "DerivedAttributedTemplate"];
    if inherits_interfaces {
        expected.push("DerivedTemplate");
    }
    assert_eq!(wrapped, expected);
}

#[test]
fn metadata_node_wraps_its_value() {
    let fw = create_styles_test_framework();
    let root = run(&fw, &format!("<ControlTemplate {} TargetType='Button'/>", xmlns()));
    let scope = root.cast::<FerroXamlIlTargetTypeMetadataNode>().expect("scope");
    assert!(scope.value().is::<XamlAstObjectNode>());
    assert_eq!(scope.type_name(), "FerroXamlIlTargetTypeMetadataNode");
    assert!(scope.as_value_with_side_effect_node_base().is_some());
    assert_eq!((scope.line(), scope.position()), (scope.value().line(), scope.value().position()));
    assert_eq!(ScopeTypes::Style as i32, 1);
    assert_eq!(ScopeTypes::ControlTemplate as i32, 2);
    assert_eq!(ScopeTypes::Transitions as i32, 3);
    assert_eq!(ScopeTypes::Container as i32, 4);
    assert_eq!(ScopeTypes::Transitions.name(), "Transitions");
    let value: Rc<dyn IXamlAstValueNode> = scope;
    assert!(value.is::<FerroXamlIlTargetTypeMetadataNode>());
}
