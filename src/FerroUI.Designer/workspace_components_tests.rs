use super::*;
use ferroui_base::{ObjectType, Ref};
use ferroui_controls::TextBox;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

const SOURCE: &str = r##"<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><UserControl.Resources><SolidColorBrush x:Key="Accent" Color="#FF6599DD"/><ControlTemplate x:Key="CardTemplate" TargetType="Button"><Border x:Name="Frame" Background="{TemplateBinding Background}"><ContentPresenter Content="{TemplateBinding Content}"/></Border></ControlTemplate><ControlTheme x:Key="ActionTheme" TargetType="Button"><Setter Property="Background" Value="{StaticResource Accent}"/><Setter Property="Padding" Value="12,6"/></ControlTheme></UserControl.Resources><StackPanel><Button x:Name="Action" Content="Continue"/><Button x:Name="Secondary" Content="Cancel"/></StackPanel></UserControl>"##;
fn workspace() -> Rc<DesignerWorkspace> {
    DesignerWorkspace::with_uri(SOURCE, "file:///Components.xaml".into()).unwrap()
}
fn node(workspace: &DesignerWorkspace, key: &str) -> NodeId {
    workspace
        .session
        .borrow()
        .component_catalog()
        .unwrap()
        .entries
        .into_iter()
        .find(|entry| entry.key.as_deref() == Some(key))
        .unwrap()
        .node
}
fn named<T: ObjectType>(workspace: &DesignerWorkspace, name: &str) -> Ref<T> {
    // Programmatically assembled inspectors have their own logical subtrees;
    // they are not declarations in the workspace shell's XAML namescope.
    workspace
        .control()
        .get_logical_descendants()
        .find(|control| control.name().as_deref() == Some(name))
        .and_then(|control| control.cast::<T>())
        .unwrap_or_else(|| panic!("Missing native workspace control {name}"))
}
fn click(workspace: &DesignerWorkspace, name: &str) {
    named::<Button>(workspace, name)
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

#[test]
fn scope_roundtrip_retires_definition_drafts_even_with_the_same_source_and_selection() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    let brush = node(&workspace, "Accent");
    let template = node(&workspace, "CardTemplate");
    workspace.inspect_component(brush).unwrap();
    let old = named::<Button>(&workspace, "DefinitionRename");
    named::<TextBox>(&workspace, "DefinitionResourceKey").set_text(Some("Stale"));
    let source = workspace.session.borrow().document().source().to_owned();
    let selected = workspace.session.borrow().selection().to_vec();
    let revision = workspace.session.borrow().document().revision();
    workspace
        .session
        .borrow_mut()
        .enter_definition_scope(template)
        .unwrap();
    workspace
        .session
        .borrow_mut()
        .exit_definition_scope()
        .unwrap();
    assert_eq!(workspace.session.borrow().selection(), selected);
    assert_eq!(workspace.session.borrow().document().revision(), revision);
    old.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(workspace.session.borrow().document().source(), source);
}

#[test]
fn inspector_extract_resource_creates_scope_and_reference_in_one_history_entry() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    let target = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Secondary")
        .unwrap();
    let owner = workspace.session.borrow().document().ast().root();
    workspace.session.borrow_mut().select([target]);
    workspace.rebuild_properties();
    let row = workspace
        .inspectors
        .property_source
        .items()
        .to_vec()
        .into_iter()
        .find(|row| row.name == "Content")
        .unwrap();
    let metadata = row.metadata.as_ref().unwrap();
    assert!(metadata.resource_owners.iter().any(|(id, _)| *id == owner));
    let before = workspace.session.borrow().document().source().to_owned();
    metadata
        .apply(
            row.context.as_ref().unwrap(),
            PropertyValueEdit::ExtractResource {
                owner,
                key: "CancelCaption".into(),
            },
        )
        .unwrap();
    let session = workspace.session.borrow();
    assert_eq!(
        session
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Content"),
        Some("{StaticResource 'CancelCaption'}")
    );
    assert!(
        session
            .component_catalog()
            .unwrap()
            .entries
            .iter()
            .any(|entry| entry.key.as_deref() == Some("CancelCaption"))
    );
    drop(session);
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), before);
    assert!(!workspace.session.borrow().can_undo());
}

#[test]
fn instance_edit_a_copy_retargets_only_that_instance_and_opens_its_definition() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    let target = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Action")
        .unwrap();
    let theme = node(&workspace, "ActionTheme");
    workspace
        .session
        .borrow_mut()
        .edit_component(ComponentAction::ApplyResource {
            resource: theme,
            targets: vec![target],
            property: "Theme".into(),
            dynamic: false,
        })
        .unwrap();
    workspace.session.borrow_mut().select([target]);
    workspace.refresh(true);
    let before = workspace.session.borrow().document().source().to_owned();
    click(&workspace, &format!("LinkedDefinitionCopy_{theme}"));
    let session = workspace.session.borrow();
    let scope = session
        .definition_scope()
        .expect("Edit a copy opens the native definition preview");
    assert_ne!(scope.node, theme);
    assert_eq!(
        session
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Theme"),
        Some("{StaticResource 'ActionThemeCopy'}")
    );
    drop(session);
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), before);
}

#[test]
fn library_selection_retains_canvas_targets_and_resource_apply_is_one_undoable_edit() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    let target = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Action")
        .unwrap();
    workspace.session.borrow_mut().select([target]);
    workspace.refresh(true);
    let brush = node(&workspace, "Accent");
    let before = workspace.session.borrow().document().source().to_owned();
    workspace.inspect_component(brush).unwrap();
    assert_eq!(workspace.session.borrow().selection(), &[target]);
    assert_eq!(workspace.navigation_pane(), crate::NavigationPane::Library);
    click(&workspace, "DefinitionApply");
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Background"),
        Some("{StaticResource 'Accent'}")
    );
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), before);
}

#[test]
fn retired_definition_editor_cannot_change_a_new_revision() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    let brush = node(&workspace, "Accent");
    workspace.inspect_component(brush).unwrap();
    let old = named::<Button>(&workspace, "DefinitionRename");
    named::<TextBox>(&workspace, "DefinitionResourceKey").set_text(Some("StaleKey"));
    workspace
        .session
        .borrow_mut()
        .edit_component(ComponentAction::RenameResource {
            resource: brush,
            key: "FreshKey".into(),
        })
        .unwrap();
    workspace.refresh(true);
    let before = workspace.session.borrow().document().source().to_owned();
    old.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(workspace.session.borrow().document().source(), before);
}

#[test]
fn definition_scope_change_retires_old_dialog_without_changing_document_revision() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    let brush = node(&workspace, "Accent");
    let template = node(&workspace, "CardTemplate");
    workspace.inspect_component(brush).unwrap();
    let old = named::<Button>(&workspace, "DefinitionRename");
    named::<TextBox>(&workspace, "DefinitionResourceKey").set_text(Some("Retired"));
    let before = workspace.session.borrow().document().source().to_owned();
    let revision = workspace.session.borrow().document().revision();
    workspace
        .session
        .borrow_mut()
        .enter_definition_scope(template)
        .unwrap();
    assert_eq!(workspace.session.borrow().document().revision(), revision);
    old.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
    assert_eq!(workspace.session.borrow().document().source(), before);
}

#[test]
fn registered_library_documents_preserve_source_provenance_without_document_edits() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    let before = workspace.session.borrow().document().source().to_owned();
    let count = workspace
        .register_component_document("library:///Controls.xaml", SOURCE)
        .unwrap();
    assert_eq!(count, 3);
    let state = workspace.components.borrow();
    let registered = state
        .definitions
        .values()
        .filter(|asset| asset.origin_uri.as_deref() == Some("library:///Controls.xaml"))
        .collect::<Vec<_>>();
    assert_eq!(registered.len(), 3);
    for asset in registered {
        assert_eq!(
            asset.origin_uri.as_deref(),
            Some("library:///Controls.xaml")
        );
        assert!(asset.source.contains("xmlns:x="));
    }
    let accepted = state.definitions.clone();
    drop(state);
    assert_eq!(workspace.session.borrow().document().source(), before);
    assert!(
        workspace
            .register_component_document("library:///Broken.xaml", "<ControlTheme>")
            .is_err()
    );
    assert_eq!(workspace.components.borrow().definitions, accepted);
}

#[test]
fn linked_definition_banner_opens_definition_without_retargeting_instance() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    let target = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Action")
        .unwrap();
    let theme = node(&workspace, "ActionTheme");
    workspace
        .session
        .borrow_mut()
        .edit_component(ComponentAction::ApplyResource {
            resource: theme,
            targets: vec![target],
            property: "Theme".into(),
            dynamic: false,
        })
        .unwrap();
    workspace.session.borrow_mut().select([target]);
    workspace.refresh(true);
    assert!(
        workspace
            .control()
            .get_control::<Control>("ComponentInstanceHost")
            .is_visible()
    );
    workspace.inspect_component(theme).unwrap();
    assert_eq!(workspace.session.borrow().selection(), &[target]);
    click(&workspace, "StatesMode");
    assert!(
        !workspace
            .control()
            .get_control::<Control>("ComponentInspectorPane")
            .is_visible()
    );
    assert!(
        workspace
            .control()
            .get_control::<Control>("AuthoringScroll")
            .is_visible()
    );
}

#[test]
fn source_backed_definition_registration_rejects_mislabeled_types_atomically() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    let accepted = workspace.components.borrow().definitions.clone();
    let asset = DesignerDefinitionAsset {
        id: "custom".into(),
        label: "Invalid theme".into(),
        kind: ComponentKind::ControlTheme,
        target_type: None,
        source:
            r#"<ControlTemplate xmlns="https://github.com/ferroui"><Border/></ControlTemplate>"#
                .into(),
        origin_uri: None,
    };
    assert!(workspace.register_component_definition(asset).is_err());
    assert_eq!(workspace.components.borrow().definitions, accepted);
}

#[test]
fn library_edit_copy_keeps_based_on_and_brush_dependencies_independent_from_document() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    let library = r#"<ResourceDictionary xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><SolidColorBrush x:Key="Accent" Color="Red"/><ControlTheme x:Key="Base" TargetType="Button"><Setter Property="Background" Value="{StaticResource Accent}"/></ControlTheme><ControlTheme x:Key="Derived" TargetType="Button" BasedOn="{StaticResource Base}"/></ResourceDictionary>"#;
    workspace
        .register_component_document("library:///DependencyTheme.xaml", library)
        .unwrap();
    let asset = workspace
        .components
        .borrow()
        .definitions
        .values()
        .find(|asset| asset.label == "Derived")
        .cloned()
        .unwrap();
    let before = workspace.session.borrow().document().source().to_owned();
    let owner = workspace.session.borrow().document().ast().root();
    workspace
        .copy_provided_definition(&asset, owner, "CustomAction")
        .unwrap();
    let copy = node(&workspace, "CustomAction");
    assert!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(copy)
            .unwrap()
            .attribute("BasedOn")
            .unwrap()
            .contains("CustomAction_Base")
    );
    assert!(
        workspace
            .session
            .borrow()
            .document()
            .source()
            .contains("CustomAction_Accent")
    );
    let original = node(&workspace, "Accent");
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(original)
            .unwrap()
            .attribute("Color"),
        Some("#FF6599DD")
    );
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), before);
}

#[cfg(feature = "render-smoke")]
#[test]
fn real_winui_theme_source_edit_copy_loads_base_theme_and_preserves_provider() {
    let mut services = TestServices::styled_window();
    services.theme = Some(Rc::new(|| {
        ferroui_themes_winui::WinUITheme::new().as_style()
    }));
    let _app = UnitTestApplication::start(services);
    ferroui_themes_winui::register_types();
    let workspace = workspace();
    let source = include_str!("../FerroUI.Themes.WinUI/Controls/Button.xaml");
    workspace
        .register_component_document("ferres://FerroUI.Themes.WinUI/Controls/Button.xaml", source)
        .unwrap();
    let asset = workspace
        .components
        .borrow()
        .definitions
        .values()
        .find(|asset| asset.label == "AccentButtonStyle")
        .cloned()
        .unwrap();
    let original_asset = asset.clone();
    let before = workspace.session.borrow().document().source().to_owned();
    let owner = workspace.session.borrow().document().ast().root();
    workspace
        .copy_provided_definition(&asset, owner, "BrandButton")
        .unwrap();
    let copy = node(&workspace, "BrandButton");
    workspace
        .session
        .borrow_mut()
        .enter_definition_scope(copy)
        .unwrap();
    assert!(
        matches!(
            workspace.session.borrow().preview_status(),
            crate::PreviewStatus::Ready { .. }
        ),
        "{:?}",
        workspace.session.borrow().preview_status()
    );
    assert!(
        workspace
            .session
            .borrow()
            .document()
            .source()
            .contains("BrandButton_DefaultButtonStyle")
    );
    assert_eq!(
        workspace.components.borrow().definitions.get(&asset.id),
        Some(&original_asset)
    );
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), before);
}

#[test]
fn user_control_definition_opens_a_source_tab_without_losing_current_unsaved_work() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    let target = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Action")
        .unwrap();
    workspace
        .session
        .borrow_mut()
        .set_property(&[target], "Content", "Unsaved work")
        .unwrap();
    let before = workspace.session.borrow().document().source().to_owned();
    let uri = workspace.session.borrow().document_uri().to_owned();
    workspace.register_component_document("file:///UserControlDefinition.xaml", r#"<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="Example.ProductCard"><TextBlock Text="Definition"/></UserControl>"#).unwrap();
    let id = workspace
        .components
        .borrow()
        .definitions
        .values()
        .find(|asset| {
            asset.kind == ComponentKind::UserControl
                && asset.origin_uri.as_deref() == Some("file:///UserControlDefinition.xaml")
        })
        .unwrap()
        .id
        .clone();
    workspace.open_component_document(&id).unwrap();
    assert!(
        workspace
            .session
            .borrow()
            .document()
            .source()
            .contains("Example.ProductCard")
    );
    assert_eq!(workspace.document_uris().len(), 2);
    workspace.activate_document(&uri).unwrap();
    assert_eq!(workspace.session.borrow().document().source(), before);
    assert!(workspace.session.borrow().can_undo());
}

#[test]
fn code_behind_control_definition_inserts_a_native_type_reference_and_never_nested_class_source() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    ferroui_markup_xaml::register_types();
    let asset = DesignerDefinitionAsset {
        id: "native-user-control".into(), label: "UserControl definition".into(), kind: ComponentKind::UserControl,
        target_type: None,
        source: r#"<UserControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" x:Class="FerroUI.Controls.UserControl"><TextBlock Text="Source content belongs to the constructor"/></UserControl>"#.into(),
        origin_uri: Some("file:///UserControlDefinition.xaml".into()),
    };
    let snippet = control_types::control_instance_source(&asset).unwrap();
    assert!(snippet.contains("dc:UserControl"));
    assert!(snippet.contains("using:FerroUI.Controls"));
    assert!(!snippet.contains("x:Class"));
    assert!(!snippet.contains("TextBlock"));
    let mut unregistered = asset;
    unregistered.source = unregistered
        .source
        .replace("FerroUI.Controls.UserControl", "Unregistered.ProductCard");
    assert!(
        control_types::control_instance_source(&unregistered)
            .unwrap_err()
            .contains("Register")
    );
}
