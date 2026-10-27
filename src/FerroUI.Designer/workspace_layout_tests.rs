use super::*;
use ferroui_base::input::{Key, KeyEventArgs};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_designer_tools::GridLength;

const SOURCE: &str = r#"<Grid xmlns="https://github.com/ferroui" Name="Layout" RowDefinitions="Auto,*,40" ColumnDefinitions="*,2*"><Button Name="Target" Grid.Row="1" Grid.RowSpan="2" /></Grid>"#;

fn enter(editor: &GridTrackEditor, value: &str) {
    editor.length_editor().set_text(Some(value));
    let mut event = KeyEventArgs::new();
    event.set_routed_event(Some(InputElement::key_down_event()));
    event.key = Key::Enter;
    editor.length_editor().raise_event(&event);
}

#[test]
fn grid_structure_updates_live_preview_and_cells_in_one_undo_even_when_recording() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let session = workspace.session();
    let root = session.borrow().document().ast().root();
    session.borrow_mut().set_recording(true);
    session
        .borrow_mut()
        .edit_grid_tracks(
            root,
            Axis::Vertical,
            GridTrackChange::Insert {
                index: 2,
                length: GridLength::Pixels(24.0),
            },
        )
        .unwrap();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    assert_eq!(
        session
            .borrow()
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Grid.RowSpan"),
        Some("3")
    );
    assert!(session.borrow().can_manipulate());
    assert!(session.borrow().timeline().tracks.is_empty());
    let visual = session
        .borrow()
        .preview
        .visual(root)
        .unwrap()
        .cast::<Grid>()
        .unwrap();
    assert_eq!(visual.row_definitions().count(), 4);
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), SOURCE);
}

#[test]
fn contextual_grid_editor_preserves_selection_and_rejects_retired_widgets() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let target = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    workspace.session.borrow_mut().select([target]);
    workspace.refresh(true);
    let editors = workspace.grid_track_editors();
    assert_eq!(editors.len(), 2);
    editors[0].track_selector().set_selected_index(1);
    enter(&editors[0], "3");
    let root = workspace.session.borrow().document().ast().root();
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(root)
            .unwrap()
            .attribute("RowDefinitions"),
        Some("Auto,3*,40")
    );
    assert_eq!(workspace.session.borrow().selection(), &[target]);
    assert_eq!(workspace.grid_track_editors()[0].selected_track(), 1);
    let accepted = workspace.session.borrow().document().source().to_owned();
    enter(&editors[0], "9");
    assert_eq!(workspace.session.borrow().document().source(), accepted);
    let current = workspace.grid_track_editors();
    workspace.session.borrow_mut().set_read_only(true);
    enter(&current[0], "8");
    assert_eq!(workspace.session.borrow().document().source(), accepted);
    workspace.session.borrow_mut().set_read_only(false);
    workspace.session.borrow_mut().undo().unwrap();
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
}

#[test]
fn layout_editors_are_selection_and_ast_identity_guarded_and_do_not_author_on_refresh() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let root = workspace.session.borrow().document().ast().root();
    workspace.session.borrow_mut().select([root]);
    workspace.refresh(true);
    let previous = workspace.grid_track_editors();
    assert!(!workspace.session.borrow().can_undo());
    let target = workspace
        .session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    workspace.session.borrow_mut().select([target]);
    enter(&previous[1], "5");
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
    workspace.refresh(true);
    let previous = workspace.grid_track_editors();
    let uri = workspace.session.borrow().document_uri().to_owned();
    workspace
        .session
        .borrow_mut()
        .set_document_uri(format!("{uri}.retired"))
        .unwrap();
    let mut replacement = DesignerSession::with_uri(SOURCE, ContentControl::new(), uri).unwrap();
    replacement.select([target]);
    *workspace.session.borrow_mut() = replacement;
    enter(&previous[1], "7");
    assert_eq!(workspace.session.borrow().document().source(), SOURCE);
}

#[test]
fn grid_edits_materialize_inherited_cells_without_rewriting_styles() {
    const SOURCE: &str = r#"<Grid xmlns="https://github.com/ferroui" RowDefinitions="*,*"><Grid.Styles><Style Selector="Button"><Setter Property="Grid.Row" Value="1" /></Style></Grid.Styles><Button Name="Styled" /></Grid>"#;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let window = ferroui_controls::Window::new();
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    window.update_layout();
    let session = workspace.session();
    let root = session.borrow().document().ast().root();
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Styled")
        .unwrap();
    let child = session
        .borrow()
        .preview
        .visual(target)
        .unwrap()
        .cast::<Control>()
        .unwrap();
    assert_eq!(Grid::get_row(&child), 1);
    session
        .borrow_mut()
        .edit_grid_tracks(
            root,
            Axis::Vertical,
            GridTrackChange::Insert {
                index: 1,
                length: GridLength::Auto,
            },
        )
        .unwrap();
    assert_eq!(
        session
            .borrow()
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("Grid.Row"),
        Some("2")
    );
    assert!(
        session
            .borrow()
            .document()
            .source()
            .contains("Property=\"Grid.Row\" Value=\"1\"")
    );
    assert!(session.borrow().can_manipulate());
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), SOURCE);
    window.close();
}

#[test]
fn grid_edits_keep_native_attached_names_under_a_project_default_namespace() {
    const SOURCE: &str = r#"<f:Grid xmlns:f="https://github.com/ferroui" xmlns="urn:project" RowDefinitions="*,*"><f:Button Name="Target" /></f:Grid>"#;
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let session = workspace.session();
    let root = session.borrow().document().ast().root();
    session
        .borrow_mut()
        .edit_grid_tracks(
            root,
            Axis::Vertical,
            GridTrackChange::Insert {
                index: 0,
                length: GridLength::Auto,
            },
        )
        .unwrap();
    assert!(
        session.borrow().can_manipulate(),
        "{:?}",
        session.borrow().preview_status()
    );
    let target = session
        .borrow()
        .document()
        .ast()
        .find_name("Target")
        .unwrap();
    assert_eq!(
        session
            .borrow()
            .document()
            .ast()
            .node(target)
            .unwrap()
            .attribute("f:Grid.Row"),
        Some("1")
    );
    let child = session
        .borrow()
        .preview
        .visual(target)
        .unwrap()
        .cast::<Control>()
        .unwrap();
    assert_eq!(Grid::get_row(&child), 1);
    session.borrow_mut().undo().unwrap();
    assert_eq!(session.borrow().document().source(), SOURCE);
}

#[test]
fn grid_length_commit_retains_keyboard_focus_for_consecutive_edits() {
    use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
    let _app = UnitTestApplication::start(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)),
    );
    let workspace = DesignerWorkspace::new(SOURCE).unwrap();
    let window = ferroui_controls::Window::new();
    window.set_width(1280.0);
    window.set_height(820.0);
    window.set_content(Some(Control::boxed(workspace.control())));
    window.show();
    let root = workspace.session.borrow().document().ast().root();
    workspace.session.borrow_mut().select([root]);
    workspace.refresh(true);
    window.update_layout();
    let editor = workspace.grid_track_editors()[1].clone();
    editor.length_editor().focus();
    assert!(editor.length_editor().is_focused());
    enter(&editor, "3");
    Dispatcher::ui_thread().run_jobs(None);
    let current = workspace.grid_track_editors()[1].clone();
    assert!(current.length_editor().is_focused());
    enter(&current, "4");
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(
        workspace
            .session
            .borrow()
            .document()
            .ast()
            .node(root)
            .unwrap()
            .attribute("ColumnDefinitions"),
        Some("4*,2*")
    );
    window.close();
}
