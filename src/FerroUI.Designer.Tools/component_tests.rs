use super::*;

const NS: &str =
    r#"xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml""#;
fn document(body: &str) -> Document {
    Document::new(format!("<Grid {NS}>{body}</Grid>")).unwrap()
}
fn named(document: &Document, name: &str) -> NodeId {
    document.ast().find_name(name).unwrap()
}
fn keyed(document: &Document, key: &str) -> NodeId {
    document
        .ast()
        .elements()
        .find(|node| literal_key(node).as_deref() == Some(key))
        .unwrap()
        .id
}
fn apply(document: &mut Document, action: ComponentAction) {
    let edits = plan_component_action(document, &action).unwrap();
    document.apply("Component edit", edits).unwrap();
}

#[test]
fn catalog_reports_definitions_custom_types_scopes_and_resolved_usages() {
    let d = document(
        r##"<Grid.Resources><SolidColorBrush x:Key="Accent" Color="#FF335577"/><ControlTheme x:Key="Card" TargetType="Button"><Setter Property="Template"><ControlTemplate><Border Background="{StaticResource Accent}"/></ControlTemplate></Setter></ControlTheme><DataTemplate x:Key="Item"><TextBlock Text="Item"/></DataTemplate></Grid.Resources><UserControl Name="Composite"/><c:Badge xmlns:c="urn:acme" Name="Custom"/><Button Name="Target" Theme="{StaticResource Card}"/>"##,
    );
    let catalog = ComponentCatalog::from_document(&d).unwrap();
    assert!(
        catalog
            .entries
            .iter()
            .any(|entry| entry.kind == ComponentKind::UserControl)
    );
    assert!(
        catalog
            .entries
            .iter()
            .any(|entry| entry.kind == ComponentKind::CustomControl
                && entry.type_name == "{urn:acme}Badge")
    );
    assert!(
        catalog
            .entries
            .iter()
            .any(|entry| entry.kind == ComponentKind::ControlTemplate)
    );
    assert_eq!(
        catalog
            .entries
            .iter()
            .find(|entry| entry.key.as_deref() == Some("Card"))
            .unwrap()
            .usages,
        1
    );
    assert_eq!(
        catalog
            .entries
            .iter()
            .find(|entry| entry.key.as_deref() == Some("Accent"))
            .unwrap()
            .usages,
        1
    );
}

#[test]
fn resource_lookup_preserves_local_shadowing_merge_order_and_external_uncertainty() {
    let d = document(r#"<Grid.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceDictionary><Color x:Key="Merged" Name="First">Red</Color></ResourceDictionary><ResourceDictionary><Color x:Key="Merged" Name="Last">Blue</Color></ResourceDictionary></ResourceDictionary.MergedDictionaries><Color x:Key="Local" Name="Outer">Red</Color></ResourceDictionary></Grid.Resources><Border Name="Inner"><Border.Resources><Color x:Key="Local" Name="Shadow">Green</Color></Border.Resources><Button Name="Child" /></Border><Button Name="Sibling"/></Grid>"#.trim_end_matches("</Grid>"));
    assert_eq!(
        resolve_component_resource(&d, named(&d, "Child"), "Local"),
        Some(named(&d, "Shadow"))
    );
    assert_eq!(
        resolve_component_resource(&d, named(&d, "Sibling"), "Local"),
        Some(named(&d, "Outer"))
    );
    assert_eq!(
        resolve_component_resource(&d, named(&d, "Child"), "Merged"),
        Some(named(&d, "Last"))
    );
    let external = document(
        r#"<Grid.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceDictionary><Color x:Key="Accent">Red</Color></ResourceDictionary><ResourceInclude Source="Unknown.xaml"/></ResourceDictionary.MergedDictionaries></ResourceDictionary></Grid.Resources><Button Name="Target"/>"#,
    );
    assert_eq!(
        resolve_component_resource(&external, named(&external, "Target"), "Accent"),
        None
    );
}

#[test]
fn rename_changes_only_resolved_nested_references_and_preserves_shadowed_keys() {
    let mut d = document(
        r#"<Grid.Resources><Color x:Key="A&amp;B" Name="Outer">Red</Color></Grid.Resources><!-- retained --><Border><Border.Resources><Color x:Key="A&amp;B">Blue</Color></Border.Resources><Button Name="Shadowed" Background="{StaticResource 'A&amp;B'}"/></Border><Button Name="Target" Background="{Binding Converter={StaticResource ResourceKey='A&amp;B'}, ConverterParameter='A&amp;B'}"/><Button><Button.Background><DynamicResource ResourceKey="A&amp;B"/></Button.Background></Button>"#,
    );
    let original = d.source().to_owned();
    let resource = named(&d, "Outer");
    apply(
        &mut d,
        ComponentAction::RenameResource {
            resource,
            key: "New ' key\\".into(),
        },
    );
    assert!(d.source().contains("<!-- retained -->"));
    assert!(
        d.ast()
            .node(named(&d, "Shadowed"))
            .unwrap()
            .attribute("Background")
            .unwrap()
            .contains("A&B")
    );
    assert_eq!(
        component_resource_references(&d, resource).unwrap().len(),
        2
    );
    assert_eq!(
        literal_key(d.ast().node(resource).unwrap()).unwrap(),
        "New ' key\\"
    );
    assert!(
        d.ast()
            .node(named(&d, "Target"))
            .unwrap()
            .attribute("Background")
            .unwrap()
            .contains("ConverterParameter='A&B'")
    );
    d.undo().unwrap();
    assert_eq!(d.source(), original);
}

#[test]
fn rename_rejects_scope_capture_and_duplicate_keys_without_mutating() {
    let d = document(
        r#"<Grid.Resources><Color x:Key="First" Name="First">Red</Color><Color x:Key="Other">Blue</Color></Grid.Resources><Border><Border.Resources><Color x:Key="New">Green</Color></Border.Resources><Button Background="{StaticResource First}"/></Border>"#,
    );
    let source = d.source().to_owned();
    for key in ["New", "Other"] {
        assert!(
            plan_component_action(
                &d,
                &ComponentAction::RenameResource {
                    resource: named(&d, "First"),
                    key: key.into()
                }
            )
            .is_err()
        );
    }
    assert_eq!(d.source(), source);
}

#[test]
fn create_duplicate_apply_and_delete_are_atomic_and_namespace_safe() {
    let mut d = Document::new(r#"<u:Grid xmlns="urn:app" xmlns:u="https://github.com/ferroui" xmlns:k="http://schemas.microsoft.com/winfx/2006/xaml"><u:Button Name="Target"/></u:Grid>"#).unwrap();
    let root = d.ast().root();
    apply(
        &mut d,
        ComponentAction::CreateResource {
            owner: root,
            key: "Accent".into(),
            source: "<u:SolidColorBrush Color='Red'/>".into(),
        },
    );
    let resource = keyed(&d, "Accent");
    assert_eq!(
        d.ast().node(resource).unwrap().namespace_uri.as_deref(),
        Some(FRAMEWORK)
    );
    assert_eq!(
        key_attribute(d.ast().node(resource).unwrap())
            .unwrap()
            .namespace_uri
            .as_deref(),
        Some(XAML)
    );
    apply(
        &mut d,
        ComponentAction::DuplicateResource {
            resource,
            key: "Second".into(),
        },
    );
    let target = named(&d, "Target");
    apply(
        &mut d,
        ComponentAction::ApplyResource {
            resource,
            targets: vec![target],
            property: "Background".into(),
            dynamic: true,
        },
    );
    assert_eq!(
        d.ast().node(target).unwrap().attribute("Background"),
        Some("{u:DynamicResource 'Accent'}")
    );
    assert!(plan_component_action(&d, &ComponentAction::RemoveResource { resource }).is_err());
    let second = keyed(&d, "Second");
    apply(&mut d, ComponentAction::RemoveResource { resource: second });
    assert!(d.ast().node(second).is_none());
}

#[test]
fn object_property_replacement_preserves_unknown_graph_and_namespace_closure() {
    let mut d = document(
        r#"<Border Name="A" Background="Red"/><Border Name="B"><Border.Background><SolidColorBrush Color="Blue"/></Border.Background></Border>"#,
    );
    let original = d.source().to_owned();
    let nodes = [named(&d, "A"), named(&d, "B")];
    let fragment = "<LinearGradientBrush StartPoint='0%,0%' EndPoint='100%,0%'><!-- authored --><GradientStop Offset='0.5' Color='Red'/></LinearGradientBrush>";
    let edits = plan_property_object(&d, &nodes, "Background", Some(fragment)).unwrap();
    d.apply("Replace brush", edits).unwrap();
    for node in nodes {
        assert!(
            d.ast()
                .node(node)
                .unwrap()
                .attribute("Background")
                .is_none()
        );
        let exported = component_property_object(&d, node, "Background")
            .unwrap()
            .unwrap();
        assert!(exported.contains("<!-- authored -->"));
        assert!(exported.contains("xmlns=\"https://github.com/ferroui\""));
        Document::new(exported).unwrap();
    }
    d.undo().unwrap();
    assert_eq!(d.source(), original);
    assert!(plan_property_object(&d, &nodes, "Background", Some("<Unknown:Brush/>")).is_err());
}

#[test]
fn theme_setters_edit_objects_reset_and_keep_definition_identity() {
    let mut d = document(
        r#"<Grid.Resources><ControlTheme x:Key="Card" TargetType="Button"><!-- keep --><Setter Property="Padding" Value="8"/></ControlTheme></Grid.Resources>"#,
    );
    let theme = keyed(&d, "Card");
    apply(
        &mut d,
        ComponentAction::SetSetter {
            owner: theme,
            property: "Padding".into(),
            value: Some("12".into()),
        },
    );
    apply(
        &mut d,
        ComponentAction::SetSetterObject {
            owner: theme,
            property: "Background".into(),
            source: Some("<SolidColorBrush Color='Blue'/>".into()),
        },
    );
    assert!(d.source().contains("<!-- keep -->"));
    assert_eq!(keyed(&d, "Card"), theme);
    let padding = d
        .ast()
        .subtree(theme)
        .find(|node| node.attribute("Property") == Some("Padding"))
        .unwrap();
    assert_eq!(padding.attribute("Value"), Some("12"));
    apply(
        &mut d,
        ComponentAction::SetSetter {
            owner: theme,
            property: "Padding".into(),
            value: None,
        },
    );
    assert!(
        !d.ast()
            .subtree(theme)
            .any(|node| node.attribute("Property") == Some("Padding"))
    );
    assert!(
        d.ast()
            .subtree(theme)
            .any(|node| node.local_name() == "SolidColorBrush")
    );
}

#[test]
fn implicit_setter_values_export_and_replace_without_leaving_an_old_template() {
    let mut d = document(
        r#"<Grid.Resources><ControlTheme x:Key="Card" TargetType="Button"><Setter Property="Template"><!-- keep context --><ControlTemplate><Border Name="Old"><!-- keep nested --><ContentPresenter/></Border></ControlTemplate></Setter></ControlTheme></Grid.Resources>"#,
    );
    let original = d.source().to_owned();
    let theme = keyed(&d, "Card");
    let setter = d
        .ast()
        .subtree(theme)
        .find(|node| node.local_name() == "Setter")
        .unwrap()
        .id;
    let value = component_property_object(&d, setter, "Value")
        .unwrap()
        .unwrap();
    let exported = Document::new(value).unwrap();
    assert_eq!(
        exported
            .ast()
            .node(exported.ast().root())
            .unwrap()
            .local_name(),
        "ControlTemplate"
    );
    assert!(exported.source().contains("<!-- keep nested -->"));
    apply(
        &mut d,
        ComponentAction::SetSetterObject {
            owner: theme,
            property: "Template".into(),
            source: Some("<ControlTemplate><Border Name='New'/></ControlTemplate>".into()),
        },
    );
    assert_eq!(
        d.ast()
            .subtree(setter)
            .filter(|node| node.local_name() == "ControlTemplate")
            .count(),
        1
    );
    assert!(
        d.ast()
            .subtree(setter)
            .any(|node| node.xaml_name() == Some("New"))
    );
    assert!(
        !d.ast()
            .subtree(setter)
            .any(|node| node.xaml_name() == Some("Old"))
    );
    assert!(d.source().contains("<!-- keep context -->"));
    d.undo().unwrap();
    assert_eq!(d.source(), original);
    apply(
        &mut d,
        ComponentAction::SetSetter {
            owner: theme,
            property: "Template".into(),
            value: Some("{StaticResource OtherTemplate}".into()),
        },
    );
    assert!(d.ast().element_children(setter).is_empty());
    assert_eq!(
        d.ast().node(setter).unwrap().attribute("Value"),
        Some("{StaticResource OtherTemplate}")
    );
}

#[test]
fn source_exports_decline_invalid_buffers_instead_of_using_stale_spans() {
    let mut d = document(
        "<Border Name='Target'><Border.Background><SolidColorBrush Color='Red'/></Border.Background></Border>",
    );
    let target = named(&d, "Target");
    d.update_source("Incomplete source", "<Grid");
    assert!(component_fragment(&d, target).is_err());
    assert!(component_property_object(&d, target, "Background").is_err());
    assert!(component_resource_expression(&d, target, "Accent", false).is_err());
    assert_eq!(resolve_component_resource(&d, target, "Accent"), None);
}

#[test]
fn external_resource_dictionary_wrappers_block_guessed_ancestor_resolution() {
    let d = document(
        r#"<Grid.Resources><Color x:Key="Accent">Red</Color></Grid.Resources><Border><Border.Resources><ResourceDictionary Source="Unknown.xaml"/></Border.Resources><Button Name="Target"/></Border>"#,
    );
    assert_eq!(
        resolve_component_resource(&d, named(&d, "Target"), "Accent"),
        None
    );
}

#[test]
fn escaped_resource_object_keys_are_resolved_and_renamed_as_literal_keys() {
    let mut d = document(
        r#"<Grid.Resources><Color x:Key="{}{old">Red</Color></Grid.Resources><Button><Button.Background><StaticResource ResourceKey="{}{old"/></Button.Background></Button>"#,
    );
    let resource = keyed(&d, "{old");
    assert_eq!(
        component_resource_references(&d, resource).unwrap().len(),
        1
    );
    apply(
        &mut d,
        ComponentAction::RenameResource {
            resource,
            key: "{new".into(),
        },
    );
    let reference = d
        .ast()
        .elements()
        .find(|node| node.local_name() == "StaticResource")
        .unwrap();
    assert_eq!(reference.attribute("ResourceKey"), Some("{}{new"));
    assert_eq!(
        component_resource_references(&d, resource).unwrap().len(),
        1
    );
}

#[test]
fn create_theme_template_and_copy_preserve_namespaces_and_source() {
    let mut d = document("<Button Name='Target'/>");
    let root = d.ast().root();
    apply(
        &mut d,
        ComponentAction::CreateTheme {
            owner: root,
            key: "Card".into(),
            target_type: "Button".into(),
            based_on: None,
        },
    );
    let theme = keyed(&d, "Card");
    assert_eq!(
        d.ast().node(theme).unwrap().namespace_uri.as_deref(),
        Some(FRAMEWORK)
    );
    apply(
        &mut d,
        ComponentAction::CreateTemplate {
            owner: root,
            key: "CardTemplate".into(),
            target_type: Some("Button".into()),
            content: "<Border Name='Chrome'><ContentPresenter/></Border>".into(),
            data_template: false,
        },
    );
    let template = keyed(&d, "CardTemplate");
    let original = component_fragment(&d, template).unwrap();
    apply(
        &mut d,
        ComponentAction::CopyTemplate {
            template,
            owner: root,
            key: "CardTemplateCopy".into(),
        },
    );
    assert_eq!(component_fragment(&d, template).unwrap(), original);
    assert!(
        d.ast()
            .subtree(keyed(&d, "CardTemplateCopy"))
            .any(|node| node.xaml_name() == Some("Chrome"))
    );
    assert!(
        plan_component_action(
            &d,
            &ComponentAction::CreateTemplate {
                owner: root,
                key: "Bad".into(),
                target_type: None,
                content: "<Border/><Border/>".into(),
                data_template: false
            }
        )
        .is_err()
    );
}

#[test]
fn extract_property_resource_creates_and_applies_in_one_undo() {
    let mut d = document("<Border Name='Target' Background='Red'/>");
    let original = d.source().to_owned();
    let root = d.ast().root();
    let target = named(&d, "Target");
    apply(
        &mut d,
        ComponentAction::ExtractProperty {
            node: target,
            property: "Background".into(),
            owner: root,
            key: "Accent".into(),
        },
    );
    let resource = keyed(&d, "Accent");
    assert_eq!(
        d.ast().node(resource).unwrap().attribute("Color"),
        Some("Red")
    );
    assert_eq!(
        resolve_component_resource(&d, target, "Accent"),
        Some(resource)
    );
    assert_eq!(
        component_resource_references(&d, resource).unwrap().len(),
        1
    );
    d.undo().unwrap();
    assert_eq!(d.source(), original);
}

#[test]
fn primitive_resource_text_and_explicit_definition_source_preserve_identity() {
    let mut d = document(
        "<Grid.Resources><x:String x:Key='Caption'><!-- keep -->Hello</x:String><Color x:Key='Color'/></Grid.Resources>",
    );
    let caption = keyed(&d, "Caption");
    apply(
        &mut d,
        ComponentAction::SetResourceValue {
            resource: caption,
            value: "A < B & C".into(),
        },
    );
    assert!(d.source().contains("<!-- keep -->A &lt; B &amp; C"));
    let color = keyed(&d, "Color");
    apply(
        &mut d,
        ComponentAction::SetResourceValue {
            resource: color,
            value: "Blue".into(),
        },
    );
    assert!(d.source().contains(">Blue</Color>"));
    assert!(
        plan_component_action(
            &d,
            &ComponentAction::ReplaceDefinition {
                node: caption,
                source: "<x:String x:Key='Other'>Text</x:String>".into()
            }
        )
        .is_err()
    );
    apply(
        &mut d,
        ComponentAction::ReplaceDefinition {
            node: caption,
            source: "<x:String x:Key='Caption'>Updated</x:String>".into(),
        },
    );
    assert_eq!(keyed(&d, "Caption"), caption);
}

#[test]
fn escaped_resource_keys_and_custom_extensions_are_never_confused() {
    let mut d = document(
        r#"<Grid.Resources><Color x:Key="{}{literal}" Name="Resource">Red</Color></Grid.Resources><Button Name="Target"/><Button xmlns:c="urn:custom" Content="{c:StaticResource '{literal}'}"/>"#,
    );
    let resource = named(&d, "Resource");
    let target = named(&d, "Target");
    apply(
        &mut d,
        ComponentAction::ApplyResource {
            resource,
            targets: vec![target],
            property: "Background".into(),
            dynamic: false,
        },
    );
    assert_eq!(
        component_resource_references(&d, resource).unwrap().len(),
        1
    );
    apply(
        &mut d,
        ComponentAction::RenameResource {
            resource,
            key: "new".into(),
        },
    );
    assert!(d.source().contains("c:StaticResource"));
}
