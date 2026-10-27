//! Native shell commands must preserve authoring state and expose usable workspace space.
use crate::{DesignerWorkspace, NavigationPane, STARTER_XAML, WorkspaceViewMode};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::{Rect, Size};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Button, Control, Grid};

fn click(workspace: &DesignerWorkspace, name: &str) {
    workspace
        .control()
        .get_control::<Button>(name)
        .raise_event(&RoutedEventArgs::with_event(Button::click_event()));
}

#[test]
fn workspace_modes_and_navigation_preserve_pending_text_and_document_identity() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(STARTER_XAML).unwrap();
    let session = workspace.session();
    let target = session.borrow().document().ast().find_name("Card").unwrap();
    session.borrow_mut().select([target]);
    workspace.source_editor().set_text(Some("<Canvas>"));
    let revision = session.borrow().document().revision();
    let generation = session.borrow().generation();
    assert_eq!(workspace.view_mode(), WorkspaceViewMode::Design);
    assert!(
        !workspace
            .control()
            .get_control::<Control>("SourcePane")
            .is_visible()
    );
    for (button, mode, canvas, code) in [
        ("SplitView", WorkspaceViewMode::Split, true, true),
        ("CodeView", WorkspaceViewMode::Xaml, false, true),
        ("DesignView", WorkspaceViewMode::Design, true, false),
    ] {
        click(&workspace, button);
        assert_eq!(workspace.view_mode(), mode);
        assert_eq!(
            workspace
                .control()
                .get_control::<Control>("ArtboardScroll")
                .is_visible(),
            canvas
        );
        assert_eq!(
            workspace
                .control()
                .get_control::<Control>("SourcePane")
                .is_visible(),
            code
        );
        assert!(
            workspace
                .control()
                .get_control::<Button>(button)
                .classes()
                .contains("active")
        );
    }
    for (button, pane, content) in [
        ("AssetsTab", NavigationPane::Assets, "AssetsPane"),
        ("ProjectTab", NavigationPane::Project, "ProjectPane"),
        ("LayersTab", NavigationPane::Layers, "LayersPane"),
    ] {
        click(&workspace, button);
        assert_eq!(workspace.navigation_pane(), pane);
        assert!(
            workspace
                .control()
                .get_control::<Control>(content)
                .is_visible()
        );
    }
    assert_eq!(
        workspace.source_editor().text().as_deref(),
        Some("<Canvas>")
    );
    assert_eq!(session.borrow().document().revision(), revision);
    assert_eq!(session.borrow().generation(), generation);
    assert_eq!(session.borrow().selection(), &[target]);
}

#[test]
fn fit_and_timeline_commands_use_the_actual_workspace_geometry() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(STARTER_XAML).unwrap();
    assert!(
        workspace.fit_artboard().is_err(),
        "Detached, unmeasured controls have no viewport"
    );
    let control = workspace.control();
    let size = Size::new(1600.0, 1040.0);
    control.measure(size);
    control.arrange(Rect::from_size(size));
    control.update_layout();
    workspace.set_zoom(4.0);
    click(&workspace, "ZoomFit");
    assert!(workspace.zoom() > 0.5 && workspace.zoom() < 2.0);
    let viewport = control
        .get_control::<Control>("ArtboardScroll")
        .bounds()
        .size();
    let (width, height) = workspace.artboard_size();
    assert!(width * workspace.zoom() <= viewport.width - 80.0 + 0.01);
    assert!(height * workspace.zoom() <= viewport.height - 80.0 + 0.01);
    let root = control.cast::<Grid>().unwrap();
    let timeline_height = root.row_definitions().get(4).actual_height();
    click(&workspace, "TimelineToggle");
    assert!(!workspace.timeline_visible());
    control.measure(size);
    control.arrange(Rect::from_size(size));
    assert_eq!(root.row_definitions().get(4).actual_height(), 0.0);
    click(&workspace, "TimelineToggle");
    assert!(workspace.timeline_visible());
    control.measure(size);
    control.arrange(Rect::from_size(size));
    assert_eq!(
        root.row_definitions().get(4).actual_height(),
        timeline_height
    );
    click(&workspace, "TimelineDetails");
    assert!(
        control
            .get_control::<Control>("AnimationDetails")
            .is_visible()
    );
    click(&workspace, "TimelineDetails");
    assert!(
        !control
            .get_control::<Control>("AnimationDetails")
            .is_visible()
    );
}

#[test]
fn canvas_focus_restores_resized_docks_timeline_and_pending_authoring_state() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(STARTER_XAML).unwrap();
    let root = workspace.control();
    let layout = root.get_control::<Grid>("MainLayout");
    layout
        .column_definitions()
        .get(0)
        .set_width(ferroui_controls::GridLength::from_pixels(280.0));
    layout
        .column_definitions()
        .get(4)
        .set_width(ferroui_controls::GridLength::from_pixels(360.0));
    let size = Size::new(1600.0, 1040.0);
    root.measure(size);
    root.arrange(Rect::from_size(size));
    root.update_layout();
    workspace.set_navigation_pane(NavigationPane::Assets);
    workspace.source_editor().set_text(Some("<Canvas>"));
    let revision = workspace.session().borrow().document().revision();

    click(&workspace, "FocusCanvas");
    assert!(workspace.canvas_focused());
    assert!(!workspace.timeline_visible());
    for name in [
        "NavigationDock",
        "InspectorDock",
        "NavigationDivider",
        "InspectorDivider",
    ] {
        assert!(!root.get_control::<Control>(name).is_visible());
    }
    click(&workspace, "FocusCanvas");
    root.measure(size);
    root.arrange(Rect::from_size(size));
    root.update_layout();
    assert!(!workspace.canvas_focused());
    assert!(workspace.timeline_visible());
    assert_eq!(layout.column_definitions().get(0).actual_width(), 280.0);
    assert_eq!(layout.column_definitions().get(4).actual_width(), 360.0);
    assert_eq!(workspace.navigation_pane(), NavigationPane::Assets);
    assert_eq!(
        workspace.source_editor().text().as_deref(),
        Some("<Canvas>")
    );
    assert_eq!(workspace.session().borrow().document().revision(), revision);

    workspace.set_timeline_visible(false);
    workspace.set_canvas_focused(true);
    workspace.set_canvas_focused(true);
    workspace.set_canvas_focused(false);
    assert!(
        !workspace.timeline_visible(),
        "a previously hidden timeline stays hidden"
    );
}

#[test]
fn read_only_source_input_restores_the_buffer_without_staging_or_changing_history() {
    use ferroui_base::input::{DragDropEffects, InputElement, TextInputEventArgs};
    use ferroui_controls_treedatagrid::TreeDataGridRowDragStartedEventArgs;
    use std::sync::Arc;

    const SOURCE: &str = r#"<StackPanel xmlns="https://github.com/ferroui"><Button Name="Target" Width="80"/></StackPanel>"#;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let session = workspace.session();
    let editor = workspace.source_editor();
    let ast = session.borrow().document().ast_snapshot();
    let revision = session.borrow().document().revision();
    session.borrow_mut().set_read_only(true);
    let generation = session.borrow().generation();
    assert!(
        !editor.is_read_only(),
        "simulate input before the next role-state UI refresh"
    );

    let mut input = TextInputEventArgs::new();
    input.set_routed_event(Some(InputElement::text_input_event()));
    input.text = Some("<Grid xmlns='https://github.com/ferroui'/>".into());
    editor.select_all();
    editor.raise_event(&input);
    assert_eq!(editor.text().as_deref(), Some(SOURCE));
    assert!(editor.is_read_only());
    // Host/programmatic replacements bypass the control's input permission as well.
    editor.set_text(Some("<Canvas"));
    assert_eq!(editor.text().as_deref(), Some(SOURCE));
    {
        let session = session.borrow();
        assert_eq!(session.document().source(), SOURCE);
        assert_eq!(session.document().revision(), revision);
        assert_eq!(session.generation(), generation);
        assert!(Arc::ptr_eq(&ast, &session.document().ast_snapshot()));
        assert!(!session.document().can_undo());
        assert!(!session.document().can_redo());
        assert!(!session.document().is_dirty());
    }

    session.borrow_mut().set_read_only(false);
    workspace.refresh(true);
    let grid = workspace.outline_grid();
    let model = grid.rows().unwrap().get(1).model();
    let drag = TreeDataGridRowDragStartedEventArgs::new(vec![model]);
    grid.raise_event(&drag);
    assert_eq!(
        drag.allowed_effects(),
        DragDropEffects::MOVE,
        "rejected input must not leave a pending source buffer blocking authoring"
    );

    let edited = SOURCE.replace("80", "120");
    editor.select_all();
    let mut input = TextInputEventArgs::new();
    input.set_routed_event(Some(InputElement::text_input_event()));
    input.text = Some(edited.clone());
    editor.raise_event(&input);
    assert_eq!(session.borrow().document().source(), edited);
    assert_eq!(session.borrow().document().revision(), revision + 1);
    assert!(session.borrow().document().can_undo());
    // A role change during debounce retains the accepted draft and its one history entry.
    let staged = session.borrow().document().ast_snapshot();
    session.borrow_mut().set_read_only(true);
    editor.set_text(Some("<Button/>"));
    assert_eq!(editor.text().as_deref(), Some(edited.as_str()));
    assert_eq!(session.borrow().document().revision(), revision + 1);
    assert!(Arc::ptr_eq(
        &staged,
        &session.borrow().document().ast_snapshot()
    ));
    session.borrow_mut().set_read_only(false);
    workspace.refresh(true);
    click(&workspace, "ApplySource");
    assert!(session.borrow().can_manipulate());
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), SOURCE);
    assert!(!session.borrow().document().can_undo());
}
