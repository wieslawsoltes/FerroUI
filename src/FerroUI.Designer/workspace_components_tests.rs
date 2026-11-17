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

// The Preview button of a workspace in a single view (a browser, a phone). The window it opened
// there stopped the application with the session borrowed, and every later event failed on that
// borrow: the preview opens in the view, and nothing is left borrowed.
#[test]
fn the_preview_button_of_a_single_view_application_opens_no_window() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    let window = ferroui_controls::Window::new();
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    let application = ferroui_controls::Application::current().expect("an application");
    application.set_application_lifetime(Some(Rc::new(crate::design_time::tests::SingleView)));

    click(&workspace, "NativePreview");

    assert!(workspace.session.try_borrow_mut().is_ok(), "the session is not left borrowed");
    let status = workspace.status.text().unwrap_or_default();
    assert!(!status.contains("no windows"), "the preview opened in the view: {status}");
    // The command can be given again, and the workspace goes on working.
    click(&workspace, "NativePreview");
    workspace.refresh(true);

    application.set_application_lifetime(None);
    window.close();
}

#[test]
fn a_preview_outside_the_artboard_has_the_theme_the_flow_direction_and_the_backing_of_the_artboard() {
    use crate::{PreviewEnvironment, PreviewFlowDirection, PreviewTheme};
    use ferroui_base::media::FlowDirection;
    use ferroui_base::styling::ThemeVariant;
    use ferroui_controls::ThemeVariantScope;

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = workspace();
    // The editor around the artboard is dark, as the chrome of the designer is.
    let editor = ThemeVariantScope::new();
    editor.set_requested_theme_variant(Some(ThemeVariant::dark()));
    editor.set_child(Some(workspace.control()));
    let window = ferroui_controls::Window::new();
    window.set_content(Some(Control::boxed(editor)));
    window.show();
    let host = |workspace: &DesignerWorkspace| {
        let document: Ref<Control> = Border::new().upcast();
        let host = workspace.view_preview_host(document.clone(), ferroui_base::Size::new(400.0, 300.0));
        let fit = host.cast::<ferroui_controls::Viewbox>().expect("a box that fits the preview");
        assert_eq!(fit.stretch_direction(), ferroui_base::media::StretchDirection::DownOnly);
        assert_eq!((fit.max_width(), fit.max_height()), (352.0, 252.0), "the room, less the margin");
        let scope = fit.child().and_then(|child| child.cast::<ThemeVariantScope>()).expect("a theme scope");
        let backing = scope.child().and_then(|child| child.cast::<Border>()).expect("a backing");
        assert_eq!((backing.width(), backing.height()), workspace.artboard_size());
        assert!(backing.child().is_some_and(|child| child.ptr_eq(&document)));
        (scope, backing)
    };
    let same_backing = |workspace: &DesignerWorkspace, border: &Border| {
        let artboard = workspace.artboard.background().expect("the artboard has a background");
        border.background().is_some_and(|brush| Rc::ptr_eq(&brush, &artboard))
    };

    // The styles of the editor chrome do not reach the document: the presenter has none of their
    // classes, and nothing above what the flyout opens over has (the styles select what is below
    // a class of the chrome, and a flyout is below what it opens at).
    let flyout = workspace.view_preview_flyout(Border::new().upcast(), ferroui_base::Size::new(400.0, 300.0));
    assert!(!flyout.flyout_presenter_classes().contains("designer-chrome"));
    let mut above: Option<Ref<ferroui_base::StyledElement>> = Some(workspace.view_preview_target().upcast());
    while let Some(element) = above {
        assert!(!element.classes().contains("designer-chrome"), "{:?} is of the chrome", element.name());
        above = element.parent();
    }
    // The button that opens the preview is of the chrome: the flyout does not open at it.
    let button = named::<Button>(&workspace, "NativePreview");
    let mut above: Option<Ref<ferroui_base::StyledElement>> = Some(button.upcast());
    let mut of_the_chrome = false;
    while let Some(element) = above {
        of_the_chrome |= element.classes().contains("designer-chrome");
        above = element.parent();
    }
    assert!(of_the_chrome);
    // A room that has no size yet limits nothing.
    let unsized_host = workspace.view_preview_host(Border::new().upcast(), ferroui_base::Size::default());
    assert!(unsized_host.max_width().is_infinite());

    // The default artboard is light inside the dark editor.
    let (scope, backing) = host(&workspace);
    assert_eq!(scope.requested_theme_variant(), Some(ThemeVariant::light()));
    assert!(!scope.is_set(ferroui_base::Visual::flow_direction_property()));
    let light = workspace.artboard.background().expect("a background");
    assert!(same_backing(&workspace, &backing));

    let environment = PreviewEnvironment {
        theme: PreviewTheme::Dark,
        flow_direction: PreviewFlowDirection::RightToLeft,
        ..workspace.preview_environment()
    };
    workspace.set_preview_environment(environment).unwrap();
    let (scope, backing) = host(&workspace);
    assert_eq!(scope.requested_theme_variant(), Some(ThemeVariant::dark()));
    assert_eq!(scope.flow_direction(), FlowDirection::RightToLeft);
    assert!(same_backing(&workspace, &backing));
    assert!(!Rc::ptr_eq(&light, &workspace.artboard.background().unwrap()), "the dark artboard has its own backing");

    // "App theme" is the theme the artboard inherits, here the one of the editor.
    let environment = PreviewEnvironment { theme: PreviewTheme::Inherit, ..workspace.preview_environment() };
    workspace.set_preview_environment(environment).unwrap();
    let (scope, _) = host(&workspace);
    assert_eq!(scope.requested_theme_variant(), Some(ThemeVariant::dark()));

    window.close();
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
