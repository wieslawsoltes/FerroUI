use super::*;
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{CheckBox, ComboBox, NumericUpDown, Window};

const DOCUMENT: &str = r#"<Canvas xmlns="https://github.com/ferroui" Width="400" Height="300"><StackPanel Name="Group"><Button Name="First" Width="80" Content="First" /><Button Name="Second" Width="100" Content="Second" /></StackPanel></Canvas>"#;

fn property(workspace: &DesignerWorkspace, name: &str) -> Rc<PropertyRow> {
    workspace
        .inspectors
        .property_source
        .items()
        .to_vec()
        .into_iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| panic!("missing {name}"))
}

fn select_named(workspace: &Rc<DesignerWorkspace>, names: &[&str]) -> Vec<NodeId> {
    let ids = names
        .iter()
        .map(|name| {
            workspace
                .session
                .borrow()
                .document()
                .ast()
                .find_name(name)
                .unwrap()
        })
        .collect::<Vec<_>>();
    workspace.session.borrow_mut().select(ids.iter().copied());
    workspace.refresh(true);
    ids
}

#[test]
fn inspector_outline_is_hierarchical_and_selection_synchronizes_without_rebuilding_models() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(DOCUMENT).unwrap();
    let first = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("First")
        .unwrap();
    let source = &workspace.inspectors.outline.source;
    let roots = source.items();
    assert_eq!(roots.count(), 1);
    assert_eq!(roots.get(0).children.len(), 1);
    assert_eq!(roots.get(0).children[0].children.len(), 2);
    assert_eq!(workspace.outline_grid().columns().unwrap().count(), 2);

    let selection = source.row_selection().unwrap();
    selection
        .base()
        .set_selected_index(IndexPath::from_indexes(&[0, 0, 0]));
    assert_eq!(workspace.session.borrow().selection(), &[first]);
    assert!(Rc::ptr_eq(&roots.get(0), &source.items().get(0)));
    assert!(workspace.source.selection_end() > workspace.source.selection_start());
    assert_eq!(
        workspace.view_mode(),
        super::super::shell::WorkspaceViewMode::Design
    );

    selection
        .base()
        .select(&IndexPath::from_indexes(&[0, 0, 1]));
    assert_eq!(workspace.session.borrow().selection().len(), 2);
    assert!(
        property(&workspace, "Width")
            .context
            .as_ref()
            .unwrap()
            .mixed
    );
    workspace
        .root
        .get_control::<TextBox>("OutlineSearch")
        .set_text(Some("Second"));
    Dispatcher::ui_thread().run_jobs(None);
    let root = source.items().get(0);
    assert_eq!(root.children[0].children.len(), 1);
    assert_eq!(root.children[0].children[0].entry.name, "Second");
    assert_eq!(
        workspace.session.borrow().selection().len(),
        2,
        "filtering is not a selection edit"
    );
}

#[test]
fn inspector_property_transactions_preserve_mixed_expression_reset_and_stale_edit_semantics() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(DOCUMENT).unwrap();
    let ids = select_named(&workspace, &["First", "Second"]);
    let width = property(&workspace, "Width");
    let context = width.context.as_ref().unwrap();
    assert!(context.mixed);
    assert!(width.can_reset);
    (context.apply)(Some("140".into()));
    for id in &ids {
        assert_eq!(
            workspace
                .session
                .borrow()
                .document()
                .ast()
                .node(*id)
                .unwrap()
                .attribute("Width"),
            Some("140")
        );
    }
    let accepted = workspace.session.borrow().document().source().to_owned();
    (context.apply)(Some("999".into()));
    assert_eq!(
        workspace.session.borrow().document().source(),
        accepted,
        "old virtual cells cannot overwrite a newer revision"
    );
    workspace.session.borrow_mut().undo().unwrap();
    workspace.refresh(true);
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(ids[0])
            .unwrap()
            .attribute("Width"),
        Some("80")
    );
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(ids[1])
            .unwrap()
            .attribute("Width"),
        Some("100")
    );
    let width = property(&workspace, "Width");
    (width.context.as_ref().unwrap().apply)(None);
    assert!(ids.iter().all(|id| workspace
        .session
        .borrow()
        .document()
        .ast()
        .node(*id)
        .unwrap()
        .attribute("Width")
        .is_none()));

    select_named(&workspace, &["First"]);
    let content = property(&workspace, "Content");
    (content.context.as_ref().unwrap().apply)(Some("{Binding Title}".into()));
    let expression = property(&workspace, "Content");
    assert_eq!(expression.origin, "Expression");
    assert_eq!(
        expression.context.as_ref().unwrap().value.as_deref(),
        Some("{Binding Title}")
    );
    assert!(
        crate::editors::built_in(expression.context.as_ref().unwrap())
            .cast::<TextBox>()
            .is_some()
    );
    (expression.context.as_ref().unwrap().apply)(Some(String::new()));
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(ids[0])
            .unwrap()
            .attribute("Content"),
        Some("")
    );
}

#[test]
fn inspector_stale_property_context_cannot_target_a_different_document_or_read_only_session() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::with_uri(DOCUMENT, "untitled:/first.xaml".into()).unwrap();
    let ids = select_named(&workspace, &["First"]);
    let stale = property(&workspace, "Width").context.clone().unwrap();
    let stale_root = workspace.inspectors.outline.source.items().get(0);
    let revision = workspace.session.borrow().document().revision();
    // Retire the previous reload URI before a host replaces the publicly shared session.
    // The replacement deliberately has the same URI, revision and local node identities.
    workspace
        .session
        .borrow_mut()
        .set_document_uri("untitled:/retired.xaml".into())
        .unwrap();
    let mut replacement = DesignerSession::with_uri(
        DOCUMENT,
        workspace.root.get_control::<ContentControl>("Preview"),
        "untitled:/first.xaml".into(),
    )
    .unwrap();
    replacement.select(ids.iter().copied());
    assert_eq!(replacement.document().revision(), revision);
    *workspace.session.borrow_mut() = replacement;
    assert!(!workspace
        .inspectors
        .source_outline_is_current(&workspace.session.borrow()));
    workspace
        .inspectors
        .outline
        .source
        .row_selection()
        .unwrap()
        .base()
        .set_selected_index(IndexPath::from_indexes(&[0, 0, 1]));
    assert_eq!(
        workspace.session.borrow().selection(),
        ids.as_slice(),
        "old tree rows cannot select identically numbered nodes in a replacement document"
    );
    (stale.apply)(Some("999".into()));
    assert_eq!(workspace.session.borrow().document().source(), DOCUMENT);
    workspace.rebuild_outline();
    assert!(workspace
        .inspectors
        .source_outline_is_current(&workspace.session.borrow()));
    assert!(
        !Rc::ptr_eq(
            &stale_root,
            &workspace.inspectors.outline.source.items().get(0)
        ),
        "same URI, revision and structure must still replace a different AST snapshot"
    );
    workspace
        .open_document(DOCUMENT.into(), "untitled:/second.xaml".into())
        .unwrap();
    (stale.apply)(Some("999".into()));
    assert_eq!(workspace.session.borrow().document().source(), DOCUMENT);
    select_named(&workspace, &["First"]);
    workspace.session.borrow_mut().set_read_only(true);
    workspace.rebuild_properties();
    assert!(property(&workspace, "Width").context.is_none());
}

#[test]
fn inspector_runtime_only_rows_remain_inspectable_and_authored_runtime_rows_remain_editable() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(DOCUMENT).unwrap();
    let window = Window::new();
    window.set_width(1280.0);
    window.set_height(850.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    workspace.set_outline_mode(OutlineMode::Visual);
    let source_path = workspace
        .inspectors
        .outline
        .paths
        .borrow()
        .iter()
        .find(|(row, _)| row.entry.name == "First" && row.entry.source.is_some())
        .map(|(_, path)| path.clone())
        .expect("authored runtime row");
    workspace
        .inspectors
        .outline
        .source
        .row_selection()
        .unwrap()
        .base()
        .set_selected_index(source_path);
    assert!(property(&workspace, "Width").context.is_some());
    let runtime_path = workspace
        .inspectors
        .outline
        .paths
        .borrow()
        .iter()
        .find(|(row, _)| row.entry.source.is_none() && row.entry.runtime.is_some())
        .map(|(_, path)| path.clone())
        .expect("template or preview host row");
    workspace
        .inspectors
        .outline
        .source
        .row_selection()
        .unwrap()
        .base()
        .set_selected_index(runtime_path);
    assert!(workspace.runtime_selection.get().is_some());
    let properties = workspace.inspectors.property_source.items();
    assert!(properties.count() > 0);
    assert!(properties.to_vec().iter().all(|row| row.context.is_none()));
    window.close();
}

#[test]
fn inspector_property_factories_and_tree_rows_are_virtualized_in_a_constrained_dock() {
    struct CountingFactory(Rc<Cell<usize>>);
    impl PropertyEditorFactory for CountingFactory {
        fn create(&self, _: &PropertyEditorContext) -> Option<Ref<Control>> {
            self.0.set(self.0.get() + 1);
            Some(TextBox::new().upcast())
        }
    }
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(DOCUMENT).unwrap();
    let created = Rc::new(Cell::new(0));
    workspace.register_property_editor(Rc::new(CountingFactory(created.clone())));
    select_named(&workspace, &["First"]);
    assert_eq!(
        created.get(),
        0,
        "property controls are created at realization, not model creation"
    );
    let property_count = workspace.inspectors.property_source.items().count();
    let entries = (0..2_000)
        .map(|index| TreeEntry {
            key: index.to_string(),
            label: format!("Button {index}"),
            ..Default::default()
        })
        .collect();
    // Use the explorer's public model refresh path so its synthetic large data set does not
    // clear the selected authored element whose property cells this test also measures.
    workspace.inspectors.replace_project(entries);
    workspace.set_navigation_pane(super::super::shell::NavigationPane::Project);
    let window = Window::new();
    window.set_width(1280.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let realized = workspace
        .project_grid()
        .rows_presenter()
        .expect("real native rows presenter")
        .realized_elements()
        .into_iter()
        .flatten()
        .count();
    assert!(
        realized > 0 && realized < 100,
        "2,000 rows realized {realized} controls"
    );
    assert!(
        created.get() > 0 && created.get() < property_count,
        "created {} of {property_count} property editors; window {:?}, property grid {:?}, presenter {:?}, rows {}",
        created.get(), window.bounds(), workspace.property_grid().bounds(),
        workspace.property_grid().rows_presenter().map(|presenter| (presenter.bounds(), presenter.viewport())),
        workspace.property_grid().rows_presenter().map(|presenter| presenter.realized_elements().into_iter().flatten().count()).unwrap_or(0),
    );
    assert_eq!(workspace.property_grid().columns().unwrap().count(), 2);
    window.close();
}

#[test]
fn inspector_project_grid_folders_activate_documents_through_the_shared_workspace() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let directory =
        std::env::temp_dir().join(format!("ferroui-inspector-project-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("Views")).unwrap();
    let path = directory.join("Views/Page.xaml");
    std::fs::write(&path, DOCUMENT).unwrap();
    let uri = ferroui_designer_project::canonical_file_uri(
        &ferroui_designer_project::file_uri(&path).unwrap(),
    )
    .unwrap();
    let workspace =
        DesignerWorkspace::new("<Canvas xmlns=\"https://github.com/ferroui\" />").unwrap();
    workspace
        .set_solution(ferroui_designer_project::Solution::discover(&directory).unwrap())
        .unwrap();
    let source = &workspace.inspectors.project.source;
    let path = workspace
        .inspectors
        .project
        .paths
        .borrow()
        .iter()
        .find(|(row, _)| row.entry.uri.as_deref() == Some(&uri))
        .map(|(_, path)| path.clone())
        .expect("project document row");
    assert!(path.count() >= 3, "project, folder, file hierarchy");
    source
        .row_selection()
        .unwrap()
        .base()
        .set_selected_index(path);
    assert_eq!(workspace.session.borrow().document_uri(), uri);
    assert_eq!(workspace.session.borrow().document().source(), DOCUMENT);
    drop(workspace);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn inspector_builtin_editors_use_native_types_and_keep_special_values_as_source() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let context = PropertyEditorContext {
        property: "Width".into(),
        targets: vec![1],
        value: Some("100".into()),
        mixed: false,
        runtime_type: Some("f64".into()),
        apply: Rc::new(|_| {}),
    };
    assert!(crate::editors::built_in(&context)
        .cast::<NumericUpDown>()
        .is_some());
    let mut special = context.clone();
    special.value = Some("Auto".into());
    assert!(crate::editors::built_in(&special)
        .cast::<TextBox>()
        .is_some());
    special.value = Some("{Binding Size}".into());
    assert!(crate::editors::built_in(&special)
        .cast::<TextBox>()
        .is_some());
    let boolean = PropertyEditorContext {
        property: "IsVisible".into(),
        runtime_type: Some("bool".into()),
        value: Some("False".into()),
        ..context.clone()
    };
    assert_eq!(
        crate::editors::built_in(&boolean)
            .cast::<CheckBox>()
            .unwrap()
            .is_checked(),
        Some(false)
    );
    let enumeration = PropertyEditorContext {
        property: "HorizontalAlignment".into(),
        runtime_type: None,
        value: None,
        mixed: true,
        ..context
    };
    let combo = crate::editors::built_in(&enumeration)
        .cast::<ComboBox>()
        .unwrap();
    assert_eq!(combo.selected_index(), 0);
    assert_eq!(
        combo.items().count(),
        6,
        "mixed placeholder, inherited, four enum values"
    );
}

#[test]
fn inspector_filter_temporarily_expands_matches_and_restores_user_expansion() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let grid = HierarchyGrid::new("test outline", false);
    let entries = vec![
        TreeEntry {
            key: "parent".into(),
            label: "Panel".into(),
            ..Default::default()
        },
        TreeEntry {
            key: "child".into(),
            parent: Some("parent".into()),
            name: "Needle".into(),
            ..Default::default()
        },
    ];
    grid.replace(entries.clone(), "");
    grid.source.collapse(&IndexPath::new(0));
    assert!(!grid.source.items().get(0).expanded.get());
    grid.replace(entries.clone(), "needle");
    assert!(grid.source.items().get(0).expanded.get());
    assert_eq!(grid.source.items().get(0).children.len(), 1);
    grid.replace(entries, "");
    assert!(!grid.source.items().get(0).expanded.get());
}

#[test]
fn inspector_palette_does_not_override_authored_tree_data_grid_theme_resources() {
    use ferroui_base::styling::ThemeVariant;

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(
        r#"<TreeDataGrid xmlns="https://github.com/ferroui" Name="Authored" />"#,
    )
    .unwrap();
    let window = Window::new();
    window.set_requested_theme_variant(Some(ThemeVariant::dark()));
    window.set_width(1280.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();

    let authored = {
        let session = workspace.session.borrow();
        let id = session.document().ast().find_name("Authored").unwrap();
        session
            .preview
            .visual(id)
            .unwrap()
            .cast::<TreeDataGrid>()
            .unwrap()
    };
    assert_eq!(authored.actual_theme_variant(), Some(ThemeVariant::light()));
    let brush = |control: &Control, key: &str| {
        let value = control
            .try_find_resource(&key.into(), control.actual_theme_variant().as_ref())
            .flatten()
            .expect("resolved theme brush");
        Ref::<SolidColorBrush>::from_xaml_object(&value).expect("solid color resource")
    };
    for inspector in [
        workspace.outline_grid(),
        workspace.property_grid(),
        workspace.project_grid(),
    ] {
        for name in [
            "TreeDataGridGridLinesBrush",
            "TreeDataGridSelectedCellBackgroundBrush",
            "TreeDataGridHeaderBackgroundPointerOverBrush",
            "TreeDataGridHeaderBackgroundPressedBrush",
            "TreeDataGridHeaderBorderBrushPointerOverBrush",
            "TreeDataGridHeaderBorderBrushPressedBrush",
            "TreeDataGridHeaderForegroundPointerOverBrush",
            "TreeDataGridHeaderForegroundPressedBrush",
        ] {
            let key = name.into();
            assert!(inspector.resources().contains_key(&key));
            assert!(
                !workspace
                    .root
                    .resources_if_created()
                    .is_some_and(|resources| resources.contains_key(&key)),
                "designer palette {name} must not leak into the preview resource scope"
            );
        }
        let chrome = brush(&inspector, "TreeDataGridGridLinesBrush");
        assert_eq!(chrome.color().to_uint32(), 0xff343434);
        assert_eq!(chrome.opacity(), 1.0);
    }
    let lines = brush(&authored, "TreeDataGridGridLinesBrush");
    assert_eq!(lines.color().to_uint32(), 0xff000000);
    assert_eq!(lines.opacity(), 0.1);
    let selection = brush(&authored, "TreeDataGridSelectedCellBackgroundBrush");
    assert_eq!(selection.color().to_uint32(), 0xff000000);
    assert_eq!(selection.opacity(), 0.4);

    workspace.set_preview_dark(true);
    assert_eq!(authored.actual_theme_variant(), Some(ThemeVariant::dark()));
    assert_eq!(
        brush(&authored, "TreeDataGridGridLinesBrush")
            .color()
            .to_uint32(),
        0xffffffff
    );
    window.close();
}

#[test]
fn inspector_realized_numeric_editor_commits_then_rebinds_to_the_new_document_revision() {
    use ferroui_base::input::KeyEventArgs;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(DOCUMENT).unwrap();
    let id = select_named(&workspace, &["First"])[0];
    workspace
        .root
        .get_control::<TextBox>("PropertySearch")
        .set_text(Some("Width"));
    let window = Window::new();
    window.set_width(1280.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let editor = workspace
        .property_grid()
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<NumericUpDown>())
        .find(|editor| {
            editor
                .value()
                .is_some_and(|value| value.to_string() == "80")
        })
        .expect("realized typed Width editor");
    let mut enter = KeyEventArgs::new();
    enter.key = Key::Enter;
    enter.set_routed_event(Some(InputElement::key_down_event()));
    editor.set_text(Some("155"));
    editor.raise_event(&enter);
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Width"),
        Some("155")
    );
    window.update_layout();
    let refreshed = workspace
        .property_grid()
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<NumericUpDown>())
        .find(|editor| {
            editor
                .value()
                .is_some_and(|value| value.to_string() == "155")
        })
        .expect("replacement row renders its new authored value");
    assert!(
        !editor.ptr_eq(&refreshed),
        "an old editor carries a revision-bound callback"
    );
    enter.set_handled(false);
    editor.set_text(Some("999"));
    editor.raise_event(&enter);
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Width"),
        Some("155")
    );
    // The native numeric control cannot parse Auto as a decimal, but the XAML Width editor
    // must still accept it rather than swallowing Enter inside its class handler.
    window.update_layout();
    let current = workspace
        .property_grid()
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<NumericUpDown>())
        .find(|editor| {
            editor
                .value()
                .is_some_and(|value| value.to_string() == "155")
        })
        .expect("current Width editor");
    enter.set_handled(false);
    current.set_text(Some("Auto"));
    current.raise_event(&enter);
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(id)
            .unwrap()
            .attribute("Width"),
        Some("Auto")
    );
    window.close();
}

#[test]
fn inspector_property_elements_reveal_source_reset_atomically_and_keep_metadata_collapsed() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let source = r#"<Button xmlns="https://github.com/ferroui" Name="Target"><Button.Background><SolidColorBrush Color="Red" /></Button.Background><Button.Styles><Style Selector="Button"><Setter Property="Opacity" Value="0.8" /></Style></Button.Styles></Button>"#;
    let workspace = DesignerWorkspace::new(source).unwrap();
    select_named(&workspace, &["Target"]);
    let background = property(&workspace, "Background");
    assert_eq!(background.origin, "Property element");
    assert!(background.can_reset);
    let style_row = || {
        workspace
            .inspectors
            .outline
            .paths
            .borrow()
            .iter()
            .find(|(row, _)| row.entry.label == "Button.Styles")
            .unwrap()
            .0
            .clone()
    };
    assert!(
        !style_row().expanded.get(),
        "metadata should not obscure the visual hierarchy"
    );
    let search = workspace.root.get_control::<TextBox>("OutlineSearch");
    search.set_text(Some("Setter"));
    assert!(
        style_row().expanded.get(),
        "search reveals matching descendants"
    );
    search.set_text(Some(""));
    assert!(
        !style_row().expanded.get(),
        "clearing search restores expansion state"
    );
    (background.reveal.as_ref().unwrap())();
    assert_eq!(
        workspace.view_mode(),
        super::super::shell::WorkspaceViewMode::Split
    );
    let selected = workspace.session.borrow().selection()[0];
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(selected)
            .unwrap()
            .name,
        "Button.Background"
    );
    assert_eq!(
        workspace.session.borrow().document().source(),
        source,
        "revealing a graph does not replace it"
    );
    select_named(&workspace, &["Target"]);
    (property(&workspace, "Background")
        .context
        .as_ref()
        .unwrap()
        .apply)(None);
    assert!(!workspace
        .session
        .borrow()
        .document()
        .source()
        .contains("Button.Background"));
    assert!(workspace
        .session
        .borrow()
        .document()
        .source()
        .contains("Button.Styles"));
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), source);
}

#[test]
fn inspector_tooltip_samples_animation_without_replacing_the_editor_or_authored_draft() {
    use ferroui_base::interactivity::CancelRoutedEventArgs;

    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(
        r#"<Canvas xmlns="https://github.com/ferroui"><Button Name="Target" Opacity="0.2" Width="100" Height="40" /></Canvas>"#,
    ).unwrap();
    let target = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    {
        let mut session = workspace.session.borrow_mut();
        session
            .record_keyframe(target, "Opacity", AnimationValue::Number(0.2))
            .unwrap();
        session.seek(3.0).unwrap();
        session
            .record_keyframe(target, "Opacity", AnimationValue::Number(0.8))
            .unwrap();
        session.seek(0.0).unwrap();
    }
    select_named(&workspace, &["Target"]);
    workspace
        .root
        .get_control::<TextBox>("PropertySearch")
        .set_text(Some("Opacity"));
    Dispatcher::ui_thread().run_jobs(None);
    let window = Window::new();
    window.set_width(1280.0);
    window.set_height(800.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();

    let tooltip = |control: &Control| {
        ToolTip::get_tip(control)
            .and_then(|tip| tip.downcast_ref::<String>().cloned())
            .unwrap_or_default()
    };
    let cell = workspace
        .property_grid()
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<Control>())
        .find(|control| tooltip(control).starts_with("Opacity ·"))
        .expect("realized Opacity value cell");
    let editor = cell
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<NumericUpDown>())
        .next()
        .expect("realized authored numeric editor");
    let row = property(&workspace, "Opacity");
    editor.set_text(Some("0.37"));
    workspace.session.borrow_mut().seek(1.5).unwrap();
    workspace.refresh(false);
    let opening = CancelRoutedEventArgs::with_event(ToolTip::tool_tip_opening_event());
    cell.raise_event(&opening);
    assert!(!opening.cancel());
    let tip = tooltip(&cell);
    assert!(tip.contains("Effective value: 0.5"), "{tip}");
    assert!(tip.contains("Runtime origin: Animation"), "{tip}");
    assert!(tip.contains("Authored value: Local value"), "{tip}");
    assert!(Rc::ptr_eq(&row, &property(&workspace, "Opacity")));
    assert!(cell
        .get_visual_descendants()
        .filter_map(|visual| visual.cast::<NumericUpDown>())
        .any(|current| current.ptr_eq(&editor)));
    assert_eq!(editor.text().as_deref(), Some("0.37"));
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Opacity"),
        Some("0.2")
    );

    // Viewers can inspect a running preview; edit permission does not govern diagnostics.
    workspace.session.borrow_mut().set_read_only(true);
    workspace.session.borrow_mut().seek(3.0).unwrap();
    workspace.refresh(false);
    let opening = CancelRoutedEventArgs::with_event(ToolTip::tool_tip_opening_event());
    cell.raise_event(&opening);
    assert!(!opening.cancel());
    assert!(tooltip(&cell).contains("Effective value: 0.8"));
    assert_eq!(editor.text().as_deref(), Some("0.37"));

    workspace.session.borrow_mut().select([]);
    let stale = CancelRoutedEventArgs::with_event(ToolTip::tool_tip_opening_event());
    cell.raise_event(&stale);
    assert!(
        stale.cancel(),
        "an old cell must not describe a different selection"
    );
    window.close();
}
