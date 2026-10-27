use super::*;

const PROVIDER: &str = r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><SolidColorBrush x:Key="Accent" Color="Red"/><ControlTheme x:Key="Base" TargetType="Button"><Setter Property="Background" Value="{StaticResource Accent}"/></ControlTheme><ControlTheme x:Key="Derived" TargetType="Button" BasedOn="{StaticResource Base}"><Setter Property="Padding" Value="12"/></ControlTheme></ResourceDictionary>"#;

fn provider(source: &str, key: &str) -> DefinitionSource {
    let document = Rc::new(Document::new(source).unwrap());
    let node = ComponentCatalog::from_document(&document)
        .unwrap()
        .entries
        .into_iter()
        .find(|entry| entry.key.as_deref() == Some(key))
        .unwrap()
        .node;
    DefinitionSource { document, node }
}

#[test]
fn copied_definition_imports_dependency_closure_with_unique_keys_and_one_undo() {
    let input = r#"<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><!-- café preserved --><UserControl.Resources><SolidColorBrush x:Key="Accent" Color="Blue"/></UserControl.Resources><Border Name="Card" Background="{StaticResource Accent}"/></UserControl>"#;
    let mut target = Document::new(input).unwrap();
    let card = target.ast().find_name("Card").unwrap();
    let old_brush = resolve_component_resource(&target, card, "Accent").unwrap();
    let source = provider(PROVIDER, "Derived");
    let owner = target.ast().root();
    let edits = plan_import(&target, owner, "Custom", &source, &|_| false).unwrap();
    assert_eq!(edits.len(), 1);
    target.apply("Copy definition", edits).unwrap();
    let catalog = ComponentCatalog::from_document(&target).unwrap();
    let derived = catalog
        .entries
        .iter()
        .find(|entry| entry.key.as_deref() == Some("Custom"))
        .unwrap();
    assert!(
        target
            .ast()
            .node(derived.node)
            .unwrap()
            .attribute("BasedOn")
            .unwrap()
            .contains("Custom_Base")
    );
    let base = resolve_component_resource(&target, derived.node, "Custom_Base").unwrap();
    assert_ne!(
        resolve_component_resource(&target, base, "Custom_Accent"),
        Some(old_brush)
    );
    assert_eq!(
        resolve_component_resource(&target, card, "Accent"),
        Some(old_brush)
    );
    assert!(target.source().contains("<!-- café preserved -->"));
    assert_eq!(
        source.document.source(),
        PROVIDER,
        "Library providers stay immutable"
    );
    target.undo().unwrap();
    assert_eq!(target.source(), input);
}

#[test]
fn missing_external_resource_rejects_copy_before_any_source_mutation() {
    let source = provider(
        r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><ControlTheme x:Key="Theme" TargetType="Button"><Setter Property="Background" Value="{StaticResource AppAccent}"/></ControlTheme></ResourceDictionary>"#,
        "Theme",
    );
    let target = Document::new(r#"<UserControl xmlns="https://github.com/ferroui" />"#).unwrap();
    let before = target.source().to_owned();
    let error = plan_import(&target, target.ast().root(), "Copy", &source, &|_| false).unwrap_err();
    assert!(error.contains("AppAccent"));
    assert_eq!(target.source(), before);
    assert!(
        plan_import(&target, target.ast().root(), "Copy", &source, &|key| key
            .as_str()
            == Some("AppAccent"))
        .is_ok()
    );
}

#[test]
fn cyclic_provider_dependencies_and_destination_key_conflicts_are_rejected_atomically() {
    let source = provider(
        r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><ControlTheme x:Key="A" TargetType="Button" BasedOn="{StaticResource B}"/><ControlTheme x:Key="B" TargetType="Button" BasedOn="{StaticResource A}"/></ResourceDictionary>"#,
        "A",
    );
    let target = Document::new(r#"<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><UserControl.Resources><SolidColorBrush x:Key="Custom" Color="Blue"/></UserControl.Resources></UserControl>"#).unwrap();
    assert!(
        plan_import(&target, target.ast().root(), "Copy", &source, &|_| false)
            .unwrap_err()
            .contains("cyclic")
    );
    let source = provider(PROVIDER, "Derived");
    assert!(plan_import(&target, target.ast().root(), "Custom", &source, &|_| false).is_err());
    assert!(!target.can_undo());
}

#[test]
fn imported_dependency_closure_includes_qualified_extension_objects_and_full_names() {
    let source = provider(
        r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:m="using:FerroUI.Markup.Xaml.MarkupExtensions" xmlns:app="urn:application"><SolidColorBrush x:Key="Accent" Color="Red"/><SolidColorBrush x:Key="Outline" Color="Blue"/><ControlTheme x:Key="Card" TargetType="Button"><Setter Property="Background" Value="{m:StaticResourceExtension Accent}"/><Setter Property="BorderBrush"><Setter.Value><m:DynamicResourceExtension ResourceKey="Outline"/></Setter.Value></Setter><Setter Property="Tag" Value="{app:StaticResourceExtension MissingApplicationValue}"/></ControlTheme></ResourceDictionary>"#,
        "Card",
    );
    let mut target = Document::new(r#"<UserControl xmlns="https://github.com/ferroui"/>"#).unwrap();
    let edits = plan_import(&target, target.ast().root(), "Copy", &source, &|_| false).unwrap();
    target.apply("Copy definition", edits).unwrap();
    let catalog = ComponentCatalog::from_document(&target).unwrap();
    for key in ["Copy_Accent", "Copy_Outline", "Copy"] {
        assert!(
            catalog
                .entries
                .iter()
                .any(|entry| entry.key.as_deref() == Some(key))
        );
    }
    assert!(target.source().contains("MissingApplicationValue"));
    let copied = catalog
        .entries
        .iter()
        .find(|entry| entry.key.as_deref() == Some("Copy"))
        .unwrap();
    let source = component_fragment(&target, copied.node).unwrap();
    assert!(source.contains("StaticResourceExtension 'Copy_Accent'"));
    assert!(source.contains("ResourceKey=\"Copy_Outline\""));
}

#[test]
fn type_key_resource_resolution_uses_registered_xml_namespace_mappings() {
    use ferroui_base::metadata::{MarkupAssembly, XmlnsDefinition};
    static ASSEMBLY: MarkupAssembly = MarkupAssembly {
        name: "Designer.ComponentImport.Tests",
        crate_name: "designer_component_import_tests",
        xmlns_definitions: &[XmlnsDefinition {
            xml_namespace: "urn:designer-component-import-native-controls",
            namespace: "FerroUI.Controls",
        }],
        xmlns_prefixes: &[],
        metadata: &[],
    };
    ferroui_controls::register_types();
    MarkupAssembly::register(&ASSEMBLY);
    let source = provider(
        r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:ui="urn:designer-component-import-native-controls"><ControlTheme x:Key="Card" TargetType="ui:Button" BasedOn="{StaticResource {x:Type ui:Button}}"/></ResourceDictionary>"#,
        "Card",
    );
    let target = Document::new(r#"<UserControl xmlns="https://github.com/ferroui"/>"#).unwrap();
    let edits = plan_import(&target, target.ast().root(), "Copy", &source, &|key| {
        matches!(key, ResourceKey::Type(type_) if std::ptr::eq(*type_, ferroui_controls::Button::TYPE))
    }).unwrap();
    assert_eq!(edits.len(), 1);
    assert!(
        super::super::control_types::registered_xml_type(
            "clr-namespace:FerroUI.Controls;assembly=Other.Assembly",
            "Button"
        )
        .is_none()
    );
    assert!(
        super::super::control_types::registered_xml_type(
            "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls",
            "Button"
        )
        .is_some()
    );
}
