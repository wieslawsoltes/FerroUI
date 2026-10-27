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
fn namespace_qualified_definitions_and_resource_extensions_keep_concrete_type_identity() {
    let source = r#"<c:UserControl xmlns:c="using:FerroUI.Controls" xmlns:r="clr-namespace:FerroUI.Controls;assembly=FerroUI.Base" xmlns:s="clr-namespace:FerroUI.Styling;assembly=FerroUI.Base" xmlns:t="using:FerroUI.Markup.Xaml.Templates" xmlns:m="using:FerroUI.Markup.Xaml.MarkupExtensions" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:app="urn:application" xmlns="https://github.com/ferroui"><c:UserControl.Resources><r:ResourceDictionary><Color x:Key="Accent">Blue</Color><s:ControlTheme x:Key="Card" TargetType="c:Button"><s:Setter Property="Template"><t:ControlTemplate><Border Background="{m:StaticResource Accent}"/></t:ControlTemplate></s:Setter></s:ControlTheme><t:TreeDataTemplate x:Key="Item"><TextBlock/></t:TreeDataTemplate></r:ResourceDictionary></c:UserControl.Resources><StackPanel><Button Name="Target" Theme="{m:StaticResource Card}"><Button.Background><m:DynamicResourceExtension ResourceKey="Accent"/></Button.Background></Button><app:ControlTemplate/><app:ControlTheme/><app:StaticResource ResourceKey="Accent"/></StackPanel></c:UserControl>"#;
    let mut document = Document::new(source).unwrap();
    let catalog = ComponentCatalog::from_document(&document).unwrap();
    assert_eq!(
        catalog
            .entries
            .iter()
            .find(|entry| entry.node == document.ast().root())
            .unwrap()
            .kind,
        ComponentKind::UserControl
    );
    assert_eq!(
        catalog
            .entries
            .iter()
            .find(|entry| entry.key.as_deref() == Some("Card"))
            .unwrap()
            .kind,
        ComponentKind::ControlTheme
    );
    assert_eq!(
        catalog
            .entries
            .iter()
            .find(|entry| entry.key.as_deref() == Some("Item"))
            .unwrap()
            .kind,
        ComponentKind::DataTemplate
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
            .filter(|entry| entry.kind == ComponentKind::CustomControl
                && entry.type_name.starts_with("{urn:application}"))
            .count(),
        2
    );
    let accent = keyed(&document, "Accent");
    assert_eq!(
        component_resource_references(&document, accent)
            .unwrap()
            .len(),
        2
    );
    apply(
        &mut document,
        ComponentAction::RenameResource {
            resource: accent,
            key: "Primary".into(),
        },
    );
    assert!(document.source().contains("{m:StaticResource 'Primary'}"));
    assert!(
        document
            .source()
            .contains("<m:DynamicResourceExtension ResourceKey=\"Primary\"")
    );
    assert!(
        document
            .source()
            .contains("<app:StaticResource ResourceKey=\"Accent\"")
    );
    document.undo().unwrap();
    assert_eq!(document.source(), source);
}

#[test]
fn resource_reference_generation_uses_registered_markup_extension_namespace_without_default_alias()
{
    let document = Document::new(r#"<c:Button xmlns:c="using:FerroUI.Controls" xmlns:m="clr-namespace:FerroUI.Markup.Xaml.MarkupExtensions;assembly=FerroUI.Markup.Xaml"/>"#).unwrap();
    assert_eq!(
        component_resource_expression(&document, document.ast().root(), "Accent", false).unwrap(),
        "{m:StaticResource 'Accent'}"
    );
    assert_eq!(
        component_resource_expression(&document, document.ast().root(), "Accent", true).unwrap(),
        "{m:DynamicResource 'Accent'}"
    );
    assert!(!framework_type_namespace(
        "ControlTheme",
        "clr-namespace:FerroUI.Styling;assembly=Application"
    ));
    assert!(!framework_type_namespace(
        "ControlTemplate",
        "using:FerroUI.Controls"
    ));
}

#[test]
fn property_replacement_preserves_same_member_names_in_other_namespaces_and_owners() {
    let source = r#"<Border xmlns="https://github.com/ferroui" xmlns:n="using:FerroUI.Controls" xmlns:a="urn:application" a:Background="CustomScalar"><a:Border.Background><!-- custom graph --><a:Brush Value="keep"/></a:Border.Background><n:Border.Background><SolidColorBrush Color="Red"/></n:Border.Background><a:Other.Background><a:Brush Value="also keep"/></a:Other.Background></Border>"#;
    let mut document = Document::new(source).unwrap();
    let root = document.ast().root();
    let object = component_property_object(&document, root, "Background")
        .unwrap()
        .unwrap();
    assert!(object.contains("SolidColorBrush"));
    assert!(!object.contains("CustomScalar"));
    assert_eq!(
        component_property_elements(&document, root, "Background")
            .unwrap()
            .len(),
        1
    );
    assert!(
        component_property_attribute(&document, root, "Background")
            .unwrap()
            .is_none()
    );
    apply(
        &mut document,
        ComponentAction::SetProperty {
            node: root,
            property: "Background".into(),
            value: Some("Blue".into()),
        },
    );
    assert!(document.source().contains("a:Background=\"CustomScalar\""));
    assert!(document.source().contains(
        "<a:Border.Background><!-- custom graph --><a:Brush Value=\"keep\"/></a:Border.Background>"
    ));
    assert!(
        document
            .source()
            .contains("<a:Other.Background><a:Brush Value=\"also keep\"/></a:Other.Background>")
    );
    assert!(!document.source().contains("n:Border.Background"));
    assert_eq!(
        component_property_attribute(&document, root, "Background")
            .unwrap()
            .unwrap()
            .value,
        "Blue"
    );
    document.undo().unwrap();
    assert_eq!(document.source(), source);
    apply(
        &mut document,
        ComponentAction::SetObjectProperty {
            node: root,
            property: "a:Border.Background".into(),
            source: Some("<a:Brush Value=\"updated\"/>".into()),
        },
    );
    assert!(
        document.source().contains(
            "<n:Border.Background><SolidColorBrush Color=\"Red\"/></n:Border.Background>"
        )
    );
    assert!(
        document
            .source()
            .contains("<a:Other.Background><a:Brush Value=\"also keep\"/></a:Other.Background>")
    );
    assert!(document.source().contains("<a:Brush Value=\"updated\"/>"));
}

#[test]
fn scalar_property_namespace_aliases_preserve_original_prefix_and_reject_duplicate_identities() {
    let mut document = Document::new(r#"<Border xmlns="https://github.com/ferroui" xmlns:a="urn:application" xmlns:b="urn:application" b:Mode="Original" Mode="Native"/>"#).unwrap();
    let root = document.ast().root();
    assert_eq!(
        component_property_attribute(&document, root, "a:Mode")
            .unwrap()
            .unwrap()
            .name,
        "b:Mode"
    );
    apply(
        &mut document,
        ComponentAction::SetProperty {
            node: root,
            property: "a:Mode".into(),
            value: Some("Changed".into()),
        },
    );
    assert!(document.source().contains("b:Mode=\"Changed\""));
    assert_eq!(
        component_property_attribute(&document, root, "Mode")
            .unwrap()
            .unwrap()
            .value,
        "Native"
    );
    assert!(Document::new(r#"<Border xmlns="https://github.com/ferroui" xmlns:a="urn:application" xmlns:b="urn:application" a:Mode="First" b:Mode="Second"/>"#).is_err());
}

#[test]
fn native_setter_value_replacement_keeps_application_value_attribute_and_object_graph() {
    let mut document = document(
        r#"<Grid.Resources><ControlTheme x:Key="Card" TargetType="Button"><Setter xmlns:a="urn:application" Property="Background" a:Value="Custom"><a:Setter.Value><a:Brush Value="keep"/></a:Setter.Value><Setter.Value><SolidColorBrush Color="Red"/></Setter.Value></Setter></ControlTheme></Grid.Resources>"#,
    );
    let owner = keyed(&document, "Card");
    apply(
        &mut document,
        ComponentAction::SetSetter {
            owner,
            property: "Background".into(),
            value: Some("Blue".into()),
        },
    );
    assert!(document.source().contains("a:Value=\"Custom\""));
    assert!(
        document
            .source()
            .contains("<a:Setter.Value><a:Brush Value=\"keep\"/></a:Setter.Value>")
    );
    assert!(!document.source().contains("<Setter.Value>"));
    assert!(document.source().contains("Value=\"Blue\""));
}

#[test]
fn same_owner_qualified_attributes_are_editable_without_creating_duplicate_members() {
    let mut document = document(
        r#"<Grid.Resources><ControlTheme xmlns:s="using:FerroUI.Styling" x:Key="Card" s:ControlTheme.TargetType="Button"><Setter s:Setter.Property="Background" s:Setter.Value="Red"/></ControlTheme></Grid.Resources>"#,
    );
    let owner = keyed(&document, "Card");
    assert_eq!(
        ComponentCatalog::from_document(&document)
            .unwrap()
            .entries
            .into_iter()
            .find(|entry| entry.node == owner)
            .unwrap()
            .target_type
            .as_deref(),
        Some("Button")
    );
    apply(
        &mut document,
        ComponentAction::SetSetter {
            owner,
            property: "Background".into(),
            value: Some("Blue".into()),
        },
    );
    assert!(document.source().contains("s:Setter.Value=\"Blue\""));
    assert_eq!(
        document
            .ast()
            .elements()
            .filter(|node| node.local_name() == "Setter")
            .count(),
        1
    );
    assert!(!document.source().contains(" Value="));
}

#[test]
fn custom_resources_properties_do_not_capture_native_scope_or_references() {
    let mut document = document(
        r#"<Grid.Resources><Color x:Key="Accent" Name="Native">Red</Color></Grid.Resources><a:Grid.Resources xmlns:a="urn:application"><Color x:Key="Accent" Name="Custom">Blue</Color></a:Grid.Resources><Button Name="Target" Background="{StaticResource Accent}"/>"#,
    );
    let target = named(&document, "Target");
    let native = named(&document, "Native");
    let custom = named(&document, "Custom");
    assert_eq!(
        resolve_component_resource(&document, target, "Accent"),
        Some(native)
    );
    let catalog = ComponentCatalog::from_document(&document).unwrap();
    assert!(
        catalog
            .entries
            .iter()
            .find(|entry| entry.node == custom)
            .unwrap()
            .owner
            .is_none()
    );
    apply(
        &mut document,
        ComponentAction::RenameResource {
            resource: native,
            key: "Primary".into(),
        },
    );
    assert_eq!(
        literal_key(document.ast().node(custom).unwrap()).as_deref(),
        Some("Accent")
    );
    assert!(
        document
            .ast()
            .node(target)
            .unwrap()
            .attribute("Background")
            .unwrap()
            .contains("Primary")
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

#[test]
fn resource_object_reference_aliases_are_indexed_renamed_and_preserved() {
    let mut d = document(
        r#"<Grid.Resources><Color x:Key="Accent">Red</Color></Grid.Resources><Button><Button.Background><m:StaticResourceExtension xmlns:m="using:FerroUI.Markup.Xaml.MarkupExtensions" xmlns:alias="https://github.com/ferroui" xmlns:app="urn:application" alias:StaticResourceExtension.ResourceKey="Accent" app:ResourceKey="Accent"/></Button.Background></Button>"#,
    );
    let original = d.source().to_owned();
    let resource = keyed(&d, "Accent");
    let references = component_resource_references(&d, resource).unwrap();
    assert_eq!(references.len(), 1);
    assert_eq!(
        references[0].property,
        "alias:StaticResourceExtension.ResourceKey"
    );
    assert!(plan_component_action(&d, &ComponentAction::RemoveResource { resource }).is_err());
    apply(
        &mut d,
        ComponentAction::RenameResource {
            resource,
            key: "Primary".into(),
        },
    );
    assert!(
        d.source()
            .contains("alias:StaticResourceExtension.ResourceKey=\"Primary\"")
    );
    assert!(d.source().contains("app:ResourceKey=\"Accent\""));
    assert_eq!(
        component_resource_references(&d, resource).unwrap().len(),
        1
    );
    d.undo().unwrap();
    assert_eq!(d.source(), original);
}

#[test]
fn external_dictionary_source_aliases_and_object_values_are_not_editable_local_scopes() {
    for source in [
        r#"<ResourceDictionary xmlns:r="using:FerroUI.Controls" r:ResourceDictionary.Source="Other.xaml"/>"#,
        r#"<ResourceDictionary><ResourceDictionary.Source><x:String>Other.xaml</x:String></ResourceDictionary.Source></ResourceDictionary>"#,
    ] {
        let d = document(&format!(
            r#"<Grid.Resources><Color x:Key="Accent">Red</Color></Grid.Resources><Border Name="Scope"><Border.Resources>{source}</Border.Resources><Button Name="Target"/></Border>"#,
        ));
        assert_eq!(
            resolve_component_resource(&d, named(&d, "Target"), "Accent"),
            None
        );
        assert!(
            plan_component_action(
                &d,
                &ComponentAction::CreateResource {
                    owner: named(&d, "Scope"),
                    key: "New".into(),
                    source: "<Color>Blue</Color>".into(),
                }
            )
            .is_err()
        );
    }
}

#[test]
fn copying_templates_rejects_shadowed_missing_and_newly_captured_dependencies() {
    for (reference, original_resource, destination_resource) in [
        (
            "Accent",
            r#"<Color x:Key="Accent">Red</Color>"#,
            r#"<Color x:Key="Accent">Blue</Color>"#,
        ),
        ("Private", r#"<Color x:Key="Private">Red</Color>"#, ""),
        ("Unknown", "", r#"<Color x:Key="Unknown">Blue</Color>"#),
    ] {
        let d = document(&format!(
            r#"<Border Name="Origin"><Border.Resources>{original_resource}<ControlTemplate x:Key="Original"><Border Background="{{StaticResource {reference}}}"/></ControlTemplate></Border.Resources></Border><Border Name="Destination"><Border.Resources>{destination_resource}</Border.Resources></Border>"#,
        ));
        let source = d.source().to_owned();
        let error = plan_component_action(
            &d,
            &ComponentAction::CopyTemplate {
                template: keyed(&d, "Original"),
                owner: named(&d, "Destination"),
                key: "Copy".into(),
            },
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("would change the resource"),
            "{error}"
        );
        assert_eq!(d.source(), source);
    }
}

#[test]
fn copying_templates_preserves_external_dependencies_and_remaps_copied_local_resources() {
    let mut d = document(
        r#"<Grid.Resources><Color x:Key="Accent">Red</Color><ControlTemplate x:Key="Original"><Border Background="{StaticResource Accent}"><Border.Resources><Color x:Key="Local">Green</Color></Border.Resources><ContentPresenter Background="{StaticResource Local}"/></Border></ControlTemplate></Grid.Resources><Border Name="Destination"/>"#,
    );
    let original = d.source().to_owned();
    let template = keyed(&d, "Original");
    let destination = named(&d, "Destination");
    apply(
        &mut d,
        ComponentAction::CopyTemplate {
            template,
            owner: destination,
            key: "Copy".into(),
        },
    );
    let copied = keyed(&d, "Copy");
    let inner = d
        .ast()
        .subtree(copied)
        .find(|node| node.local_name() == "ContentPresenter")
        .unwrap();
    let local = resolve_component_resource(&d, inner.id, "Local").unwrap();
    assert!(d.ast().subtree(copied).any(|node| node.id == local));
    assert_eq!(
        component_resource_references(&d, keyed(&d, "Accent"))
            .unwrap()
            .len(),
        2
    );
    d.undo().unwrap();
    assert_eq!(d.source(), original);
}

#[test]
fn theme_setter_members_resolve_owner_and_attached_aliases_without_duplicate_writes() {
    let mut d = document(
        r#"<Grid.Resources><ControlTheme xmlns:c="using:FerroUI.Controls" xmlns:one="urn:application" xmlns:two="urn:application" x:Key="Theme" TargetType="{x:Type c:Button}"><Setter Property="c:Button.Background" Value="Red"/><Setter Property="two:Layout.Mode" Value="First"/><Setter xmlns:two="urn:other" Property="two:Layout.Mode" Value="Separate"/></ControlTheme></Grid.Resources>"#,
    );
    let theme = keyed(&d, "Theme");
    apply(
        &mut d,
        ComponentAction::SetSetter {
            owner: theme,
            property: "Background".into(),
            value: Some("Blue".into()),
        },
    );
    apply(
        &mut d,
        ComponentAction::SetSetter {
            owner: theme,
            property: "one:Layout.Mode".into(),
            value: Some("Second".into()),
        },
    );
    let setters = d
        .ast()
        .subtree(theme)
        .filter(|node| node.local_name() == "Setter")
        .collect::<Vec<_>>();
    assert_eq!(setters.len(), 3);
    assert_eq!(
        setters[0].attribute("Property"),
        Some("c:Button.Background")
    );
    assert_eq!(setters[0].attribute("Value"), Some("Blue"));
    assert_eq!(setters[1].attribute("Property"), Some("two:Layout.Mode"));
    assert_eq!(setters[1].attribute("Value"), Some("Second"));
    assert_eq!(setters[2].attribute("Value"), Some("Separate"));
    apply(
        &mut d,
        ComponentAction::SetSetter {
            owner: theme,
            property: "Button.Background".into(),
            value: None,
        },
    );
    assert_eq!(
        d.ast()
            .subtree(theme)
            .filter(|node| node.local_name() == "Setter")
            .count(),
        2
    );
}

#[test]
fn theme_setter_alias_duplicates_and_mixed_containers_are_rejected_atomically() {
    for setters in [
        r#"<Setter Property="Background" Value="Red"/><Setter Property="Button.Background" Value="Blue"/>"#,
        r#"<Setter Property="Background" Value="Red"/><ControlTheme.Setters><Setter Property="Padding" Value="8"/></ControlTheme.Setters>"#,
    ] {
        let d = document(&format!(
            r#"<Grid.Resources><ControlTheme x:Key="Theme" TargetType="Button">{setters}</ControlTheme></Grid.Resources>"#
        ));
        let source = d.source().to_owned();
        assert!(
            plan_component_action(
                &d,
                &ComponentAction::SetSetter {
                    owner: keyed(&d, "Theme"),
                    property: "Background".into(),
                    value: Some("Green".into()),
                }
            )
            .is_err()
        );
        assert_eq!(d.source(), source);
    }
}

#[test]
fn qualified_setter_value_replaces_implicit_object_and_preserves_custom_values() {
    let mut d = document(
        r#"<Grid.Resources><ControlTheme x:Key="Theme" TargetType="Button"><Setter xmlns:s="using:FerroUI.Styling" xmlns:app="urn:application" Property="Template" app:Value="keep"><ControlTemplate><Border/></ControlTemplate></Setter></ControlTheme></Grid.Resources>"#,
    );
    let setter = d
        .ast()
        .elements()
        .find(|node| node.local_name() == "Setter")
        .unwrap()
        .id;
    assert!(
        component_property_object(&d, setter, "s:Setter.Value")
            .unwrap()
            .unwrap()
            .contains("ControlTemplate")
    );
    apply(
        &mut d,
        ComponentAction::SetProperty {
            node: setter,
            property: "s:Setter.Value".into(),
            value: Some("{StaticResource New}".into()),
        },
    );
    assert!(d.ast().element_children(setter).is_empty());
    assert_eq!(
        d.ast().node(setter).unwrap().attribute("app:Value"),
        Some("keep")
    );
    assert_eq!(
        component_property_attribute(&d, setter, "Value")
            .unwrap()
            .unwrap()
            .value,
        "{StaticResource New}"
    );
}

#[test]
fn copying_a_template_keeps_explicit_references_to_the_original_resource_key() {
    let mut d = document(
        r#"<Grid.Resources><ControlTemplate x:Key="Original"><ContentPresenter ContentTemplate="{StaticResource Original}"/></ControlTemplate></Grid.Resources>"#,
    );
    let template = keyed(&d, "Original");
    let owner = d.ast().root();
    apply(
        &mut d,
        ComponentAction::CopyTemplate {
            template,
            owner,
            key: "Copy".into(),
        },
    );
    assert_eq!(
        component_resource_references(&d, template).unwrap().len(),
        2
    );
}
