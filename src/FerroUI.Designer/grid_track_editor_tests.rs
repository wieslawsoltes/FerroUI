use super::*;
use ferroui_base::input::KeyEventArgs;
use ferroui_controls::testing::{TestServices, UnitTestApplication};

fn key(editor: &TextBox, key: Key) {
    let mut event = KeyEventArgs::new();
    event.set_routed_event(Some(InputElement::key_down_event()));
    event.key = key;
    editor.raise_event(&event);
}

#[test]
fn track_configuration_and_selection_are_view_state_and_lengths_commit_once() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = GridTrackEditor::new(Axis::Vertical);
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    editor.on_change(move |change| {
        observed.borrow_mut().push(change);
        Ok(())
    });
    editor.configure(vec!["Auto".into(), "2*".into(), "48".into()], 1, true);
    assert!(changes.borrow().is_empty());
    assert_eq!(editor.length_editor().text().as_deref(), Some("2"));
    editor.track_selector().set_selected_index(2);
    assert!(changes.borrow().is_empty());
    assert_eq!(editor.unit_selector().selected_index(), 1);
    editor.length_editor().set_text(Some("64"));
    assert!(changes.borrow().is_empty());
    key(&editor.length_editor(), Key::Enter);
    assert_eq!(
        *changes.borrow(),
        vec![GridTrackChange::Set {
            index: 2,
            length: GridLength::Pixels(64.0)
        }]
    );
    editor.configure(vec!["Auto".into(), "2*".into(), "64".into()], 2, true);
    editor.length_editor().set_text(Some("999"));
    key(&editor.length_editor(), Key::Escape);
    assert_eq!(editor.length_editor().text().as_deref(), Some("64"));
    assert_eq!(changes.borrow().len(), 1);
    editor.length_editor().set_text(Some("not a number"));
    key(&editor.length_editor(), Key::Enter);
    assert_eq!(changes.borrow().len(), 1);
    assert!(editor.error.is_visible());
}

#[test]
fn bound_tracks_remain_visible_and_host_errors_leave_the_draft_available() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    let editor = GridTrackEditor::new(Axis::Horizontal);
    editor.configure(vec!["{Binding Width}".into(), "*".into()], 0, true);
    assert!(editor.length_editor().is_read_only());
    assert!(!editor.unit_selector().is_enabled());
    editor.track_selector().set_selected_index(1);
    editor.on_change(|_| Err("Document changed".into()));
    editor.length_editor().set_text(Some("3"));
    key(&editor.length_editor(), Key::Enter);
    assert!(editor.error.is_visible());
    assert_eq!(editor.length_editor().text().as_deref(), Some("3"));
    assert_eq!(editor.error.text().as_deref(), Some("Document changed"));
    editor.configure(vec!["*".into()], 0, false);
    assert!(editor.length_editor().is_read_only());
    assert!(!editor.add.is_enabled());
    assert!(!editor.remove.is_enabled());
}
