//! Tests of `FerroXamlIlControlThemeTransformer`, `FerroXamlIlDuplicateSettersChecker`,
//! `FerroXamlIlStyleValidatorTransformer` and `FerroXamlIlDataTemplateWarningsTransformer`.
//! The XAML shapes come from the upstream `ControlThemeTests` and `StyleTests`.

use std::rc::Rc;

use xamlx::ast::{IXamlAstNode, XamlAstExtensions, XamlAstNodeExtensions, XamlAstObjectNode};
use xamlx::diagnostics::{XamlDiagnostic, XamlDiagnosticSeverity};
use xamlx::transform::IXamlAstTransformer;

use super::*;
use crate::compiler_extensions::FerroXamlDiagnosticCodes;
use crate::testing::styles::{create_styles_test_framework, find_nodes, run_transformers, xmlns};

fn run(
    xaml: &str,
    pipeline: Vec<Box<dyn IXamlAstTransformer>>,
) -> (Rc<dyn IXamlAstNode>, Vec<XamlDiagnostic>) {
    let fw = create_styles_test_framework();
    let root = run_transformers(&fw, xaml, pipeline).expect("errors are not fatal in the fixture");
    (root, fw.reported_diagnostics())
}

fn theme(xaml: &str) -> (Rc<dyn IXamlAstNode>, Vec<XamlDiagnostic>) {
    run(
        xaml,
        vec![
            Box::new(FerroXamlIlControlThemeTransformer),
            Box::new(FerroXamlIlControlThemeTransformer),
        ],
    )
}

#[test]
fn control_theme_is_wrapped_in_a_style_scope_for_its_target_type() {
    for target in ["Button", "{x:Type Button}"] {
        let (root, diagnostics) = theme(&format!(
            "<ControlTheme {} TargetType='{target}'><Setter Property='Width' Value='1'/></ControlTheme>",
            xmlns()
        ));
        assert_eq!(diagnostics.len(), 0, "{diagnostics:?}");
        let scopes = find_nodes::<FerroXamlIlTargetTypeMetadataNode>(&root);
        assert_eq!(scopes.len(), 1);
        assert!(root.same_node(&scopes[0]));
        assert_eq!(scopes[0].scope_type, ScopeTypes::Style);
        assert_eq!(scopes[0].target_type().get_clr_type().expect("clr").name(), "Button");
        assert!(scopes[0].value().is::<XamlAstObjectNode>());
    }

    // Setters of the theme resolve their properties against the target type
    let fw = create_styles_test_framework();
    let root = run_transformers(
        &fw,
        &format!(
            "<ControlTheme {} TargetType='Button'><Setter Property='Content' Value='x'/></ControlTheme>",
            xmlns()
        ),
        vec![
            Box::new(FerroXamlIlControlThemeTransformer),
            Box::new(FerroXamlIlSetterTransformer),
        ],
    )
    .expect("transformed");
    assert_eq!(fw.reported_diagnostics().len(), 0, "{:?}", fw.reported_diagnostics());
    assert_eq!(
        find_nodes::<crate::compiler_extensions::XamlIlFerroPropertyNode>(&root).len(),
        1
    );
}

#[test]
fn control_theme_errors() {
    let (_, diagnostics) = theme(&format!("<ControlTheme {}/>", xmlns()));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, FerroXamlDiagnosticCodes::TRANSFORM_ERROR);
    assert!(diagnostics[0].title.starts_with("ControlTheme must have a TargetType."));

    let (_, diagnostics) = theme(&format!(
        "<ControlTheme {}><ControlTheme.TargetType><Button/></ControlTheme.TargetType></ControlTheme>",
        xmlns()
    ));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert!(diagnostics[0].title.starts_with("Could not determine TargetType for ControlTheme."));

    let (_, diagnostics) = theme(&format!("<ControlTheme {} TargetType='Missing'/>", xmlns()));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert!(diagnostics[0].title.starts_with("Unable to resolve type Missing from namespace"));

    // Other objects are not touched
    let (root, diagnostics) = theme(&format!("<Style {}/>", xmlns()));
    assert_eq!(diagnostics.len(), 0);
    assert!(root.is::<XamlAstObjectNode>());
}

#[test]
fn duplicate_setters_are_reported_once_per_repetition() {
    let checker = || -> Vec<Box<dyn IXamlAstTransformer>> { vec![Box::new(FerroXamlIlDuplicateSettersChecker)] };
    let setters = "<Setter Property='Width' Value='1'/><Setter Property='Height' Value='1'/>\n<Setter Property='Width' Value='2'/><Setter Property='Width' Value='3'/>";
    for xaml in [
        format!("<Style {} Selector='Button'>{setters}</Style>", xmlns()),
        format!("<ControlTheme {} TargetType='Button'>{setters}</ControlTheme>", xmlns()),
    ] {
        let (root, diagnostics) = run(&xaml, checker());
        assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
        for diagnostic in &diagnostics {
            assert_eq!(diagnostic.code, FerroXamlDiagnosticCodes::DUPLICATE_SETTER_ERROR);
            assert_eq!(diagnostic.severity, XamlDiagnosticSeverity::Warning);
            assert_eq!(diagnostic.title, "Duplicate setter encountered for property 'Width'");
            // Reported at the style, not at the setter
            assert_eq!(
                (diagnostic.line_number, diagnostic.line_position),
                (Some(root.line()), Some(root.position()))
            );
        }
    }

    // No duplicates, setters of nested styles and setters outside of styles are not compared
    for xaml in [
        format!(
            "<Style {} Selector='Button'><Setter Property='Width' Value='1'/><Setter Property='Height' Value='1'/></Style>",
            xmlns()
        ),
        format!(
            "<Style {} Selector='Button'><Setter Property='Width' Value='1'/><Style Selector='^.a'><Setter Property='Width' Value='1'/></Style></Style>",
            xmlns()
        ),
        format!(
            "<Border {}><Setter Property='Width' Value='1'/><Setter Property='Width' Value='1'/></Border>",
            xmlns()
        ),
    ] {
        let (_, diagnostics) = run(&xaml, checker());
        assert_eq!(diagnostics.len(), 0, "{diagnostics:?}");
    }
}

#[test]
fn style_in_merged_dictionaries_is_reported() {
    let validator = || -> Vec<Box<dyn IXamlAstTransformer>> { vec![Box::new(FerroXamlIlStyleValidatorTransformer)] };
    let (_, diagnostics) = run(
        &format!(
            "<ResourceDictionary {}><ResourceDictionary.MergedDictionaries><Style Selector='Button'/><Styles/><ResourceDictionary/></ResourceDictionary.MergedDictionaries></ResourceDictionary>",
            xmlns()
        ),
        validator(),
    );
    assert_eq!(diagnostics.len(), 2, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, FerroXamlDiagnosticCodes::STYLE_IN_MERGED_DICTIONARIES);
    assert_eq!(diagnostics[0].severity, XamlDiagnosticSeverity::Warning);
    assert_eq!(
        diagnostics[0].title,
        "Including Style as part of MergedDictionaries will ignore any nested styles.Instead, you can add Style to the Styles collection on the same control or application."
    );
    assert_eq!(
        diagnostics[1].title,
        "Including Styles as part of MergedDictionaries will ignore any nested styles.Instead, you can add Styles to the Styles collection on the same control or application."
    );

    // Styles anywhere else are fine
    for xaml in [
        format!("<Border {}><Style Selector='Button'/></Border>", xmlns()),
        format!("<Border {}><Border.Resources><Style Selector='Button'/></Border.Resources></Border>", xmlns()),
        format!("<Styles {}><Style Selector='Button'/></Styles>", xmlns()),
    ] {
        let (_, diagnostics) = run(&xaml, validator());
        assert_eq!(diagnostics.len(), 0, "{diagnostics:?}");
    }
}

#[test]
fn item_container_inside_item_template_is_reported() {
    let transformer =
        || -> Vec<Box<dyn IXamlAstTransformer>> { vec![Box::new(FerroXamlIlDataTemplateWarningsTransformer)] };
    let (_, diagnostics) = run(
        &format!(
            "<ListBox {}><ListBox.ItemTemplate><DataTemplate><ListBoxItem/></DataTemplate></ListBox.ItemTemplate></ListBox>",
            xmlns()
        ),
        transformer(),
    );
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert_eq!(diagnostics[0].code, FerroXamlDiagnosticCodes::ITEM_CONTAINER_INSIDE_TEMPLATE);
    assert_eq!(diagnostics[0].severity, XamlDiagnosticSeverity::Warning);
    assert_eq!(
        diagnostics[0].title,
        "Unexpected 'ListBoxItem' inside of 'ListBox.ItemTemplate'. 'ListBox.ItemTemplate' defines template of the container content, not the container itself."
    );

    let (_, diagnostics) = run(
        &format!(
            "<ListBox {}><ListBox.DataTemplates><DataTemplate><ListBoxItem/></DataTemplate></ListBox.DataTemplates></ListBox>",
            xmlns()
        ),
        transformer(),
    );
    assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
    assert!(diagnostics[0].title.starts_with("Unexpected 'ListBoxItem' inside of 'ListBox.DataTemplates'. "));

    for xaml in [
        // Another content control
        "<ListBox {}><ListBox.ItemTemplate><DataTemplate><Button/></DataTemplate></ListBox.ItemTemplate></ListBox>",
        // Not a content control
        "<ListBox {}><ListBox.ItemTemplate><DataTemplate><Border/></DataTemplate></ListBox.ItemTemplate></ListBox>",
        // Not directly inside of the template
        "<ListBox {}><ListBox.ItemTemplate><DataTemplate><Border><ListBoxItem/></Border></DataTemplate></ListBox.ItemTemplate></ListBox>",
        // An items control without a known container type
        "<ItemsControl {}><ItemsControl.ItemTemplate><DataTemplate><ListBoxItem/></DataTemplate></ItemsControl.ItemTemplate></ItemsControl>",
        // Another template property
        "<Button {}><Button.ContentTemplate><DataTemplate><ListBoxItem/></DataTemplate></Button.ContentTemplate></Button>",
        // DataTemplates of a control that is not an items control
        "<Button {}><Button.DataTemplates><DataTemplate><ListBoxItem/></DataTemplate></Button.DataTemplates></Button>",
        // Not inside of a data template
        "<ListBox {}><ListBoxItem/></ListBox>",
    ] {
        let (_, diagnostics) = run(&xaml.replace("{}", &xmlns()), transformer());
        assert_eq!(diagnostics.len(), 0, "{xaml}: {diagnostics:?}");
    }
}
