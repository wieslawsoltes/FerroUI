use super::*;
use crate::tests::{laid_out_editor, test_scope};
use crate::TextViewPosition;
use ferroui_edit_core::document::IUndoableOperation;

fn wrapped_editor(text: &str) -> (Ref<TextEditor>, TextViewPosition) {
    let editor = laid_out_editor(text);
    editor.set_word_wrap(true);
    let area = editor.text_area();
    area.measure(Size::new(120.0, 480.0));
    area.arrange(Rect::new(0.0, 0.0, 120.0, 480.0));
    let boundary = area
        .position_at_point(Point::new(10_000.0, 2.0), false)
        .unwrap();
    assert!(boundary.is_at_end_of_line);
    assert!(boundary.column < text.len());
    editor.document().clear_undo_history().unwrap();
    (editor, boundary)
}

fn press_key(area: &TextArea, key: Key, modifiers: KeyModifiers) {
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.key_modifiers = modifiers;
    event.set_routed_event(Some(InputElement::key_down_event()));
    area.raise_event(&event);
    assert!(event.handled());
}

#[test]
fn undo_restores_the_trailing_row_and_redo_maps_the_caret_like_the_original_edit() {
    let _scope = test_scope();
    let (editor, trailing) = wrapped_editor(&"abcdefghijk".repeat(8));
    let area = editor.text_area();
    editor.set_caret_position(trailing).unwrap();
    let rectangle = area.cursor_rectangle();
    let before = editor.text();
    assert!(area.perform_text_input("!").unwrap());
    let edited = (
        editor.text(),
        editor.caret_position(),
        area.cursor_rectangle(),
    );

    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), before);
    assert_eq!(editor.caret_position(), trailing);
    assert_eq!(area.cursor_rectangle(), rectangle);
    assert!(!editor.undo().unwrap());
    assert_eq!(editor.caret_position(), trailing);

    assert!(editor.redo().unwrap());
    assert_eq!(
        (
            editor.text(),
            editor.caret_position(),
            area.cursor_rectangle()
        ),
        edited
    );
    assert!(editor.document().undo().unwrap());
    assert_eq!(editor.caret_position(), trailing);
    assert_eq!(area.cursor_rectangle(), rectangle);
}

#[test]
fn undo_restores_the_box_caret_and_the_first_row_edit_projection() {
    let _scope = test_scope();
    let text = format!("{}\nnext", "abcdefghij".repeat(7));
    let (editor, _) = wrapped_editor(&text);
    let area = editor.text_area();
    editor.set_caret_offset(1).unwrap();
    let extend_box = KeyModifiers::ALT | KeyModifiers::SHIFT;
    press_key(&area, Key::Down, extend_box);
    let selection = editor.selection();
    let caret = editor.caret_position();
    let rectangle = area.cursor_rectangle();
    assert!(selection.is_rectangular());
    assert!(editor.caret_offset() > selection.primary().caret);
    assert!(rectangle.y > 0.0);

    assert!(area.perform_text_input("!").unwrap());
    assert_eq!(editor.text(), format!("a!{}", &text[1..]));
    press_key(&area, Key::Z, KeyModifiers::CONTROL);
    assert_eq!(editor.text(), text);
    assert_eq!(editor.selection(), selection);
    assert_eq!(editor.caret_position(), caret);
    assert_eq!(area.cursor_rectangle(), rectangle);

    press_key(&area, Key::Down, extend_box);
    assert_eq!(editor.selection(), selection);
    assert!(area.cursor_rectangle().y > rectangle.y);
}

#[test]
fn continued_undo_picks_the_oldest_view_state_even_if_it_has_default_affinity() {
    let _scope = test_scope();
    let text = "abcdefghijk".repeat(8);
    let (editor, trailing) = wrapped_editor(&text);
    let mut leading = trailing;
    leading.is_at_end_of_line = false;
    editor.set_caret_position(leading).unwrap();
    let document = editor.document();
    document.insert(document.text_length(), "1").unwrap();
    editor.set_caret_position(trailing).unwrap();
    let group = document.begin_continued_update().unwrap();
    document.insert(document.text_length(), "2").unwrap();
    group.finish().unwrap();
    assert_eq!(document.undo_count(), 1);

    assert!(document.undo().unwrap());
    assert_eq!(editor.text(), text);
    assert_eq!(editor.caret_position(), leading);
}

#[test]
fn a_shared_stack_restores_caret_affinity_in_each_edited_document() {
    let _scope = test_scope();
    let text = "abcdefghijk".repeat(8);
    let (first, first_caret) = wrapped_editor(&text);
    let (second, second_caret) = wrapped_editor(&text);
    let stack = first.document().undo_stack();
    second.document().set_undo_stack(&stack).unwrap();
    first.set_caret_position(first_caret).unwrap();
    second.set_caret_position(second_caret).unwrap();

    let group = stack.start_undo_group().unwrap();
    assert!(first.text_area().perform_text_input("!").unwrap());
    assert!(second.text_area().perform_text_input("?").unwrap());
    group.finish().unwrap();
    let edited_first = (first.text(), first.caret_position());
    let edited_second = (second.text(), second.caret_position());
    assert_eq!(stack.undo_count(), 1);

    assert!(second.undo().unwrap());
    assert_eq!(first.text(), text);
    assert_eq!(second.text(), text);
    assert_eq!(first.caret_position(), first_caret);
    assert_eq!(second.caret_position(), second_caret);
    assert!(first.redo().unwrap());
    assert_eq!((first.text(), first.caret_position()), edited_first);
    assert_eq!((second.text(), second.caret_position()), edited_second);
}

#[test]
fn shared_document_undo_restores_view_state_only_for_the_original_attachments() {
    let _scope = test_scope();
    let text = "abcdefghijk".repeat(8);
    let (first, trailing) = wrapped_editor(&text);
    let (second, _) = wrapped_editor(&text);
    let document = first.document();
    second.set_document(document.clone());
    let mut leading = trailing;
    leading.is_at_end_of_line = false;
    first.set_caret_position(trailing).unwrap();
    second.set_caret_position(leading).unwrap();

    document.insert(0, "!").unwrap();
    assert!(document.undo().unwrap());
    assert_eq!(first.caret_position(), trailing);
    assert_eq!(second.caret_position(), leading);

    document.insert(document.text_length(), "!").unwrap();
    first.set_document(TextDocument::from_text("replacement"));
    first.set_document(document.clone());
    first.set_caret_position(leading).unwrap();
    assert!(document.undo().unwrap());
    assert_eq!(first.caret_position(), leading);
    assert_eq!(second.caret_position(), leading);
}

struct ApplicationAction(Rc<Cell<bool>>);

impl IUndoableOperation for ApplicationAction {
    fn undo(&self) {
        self.0.set(false);
    }
    fn redo(&self) {
        self.0.set(true);
    }
}

#[test]
fn application_history_without_text_changes_restores_affinity_on_redo() {
    let _scope = test_scope();
    let (editor, trailing) = wrapped_editor(&"abcdefghijk".repeat(8));
    editor.set_caret_position(trailing).unwrap();
    let applied = Rc::new(Cell::new(true));
    editor
        .document()
        .undo_stack()
        .push(Rc::new(ApplicationAction(applied.clone())))
        .unwrap();
    assert!(editor.undo().unwrap());
    assert!(!applied.get());
    let mut leading = trailing;
    leading.is_at_end_of_line = false;
    editor.set_caret_position(leading).unwrap();
    assert!(editor.redo().unwrap());
    assert!(applied.get());
    assert_eq!(editor.caret_position(), trailing);
}
