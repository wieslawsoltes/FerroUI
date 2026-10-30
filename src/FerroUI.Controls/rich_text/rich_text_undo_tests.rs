//! Undo grouping and history fixes of the landing of #243 (review findings M6, S1,
//! S6, S8, N3; `docs/porting/winui-rich-text.md`, R7).
use super::*;
use ferroui_base::input::text_input::{TextInputMethodEdit, TextSelection};
use ferroui_base::media::TextAlignment;
use ferroui_base::threading::Dispatcher;

fn editor(text: &str) -> RichTextEditor {
    let editor = RichTextEditor::new(RichTextDocument::from_text(text));
    let length = text.encode_utf16().count();
    editor.set_selection(EditorSelection::caret(length)).unwrap();
    editor
}

/// M6: one-character exact insertions at the caret (the Windows CoreText host and the
/// browser send every typed character this way) merge into one undo unit.
#[test]
fn exact_insertions_at_the_caret_are_one_undo_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = editor("");
    for (index, character) in ["a", "b", "c"].into_iter().enumerate() {
        let at = index as i32;
        let edit = TextInputMethodEdit::new(TextSelection::new(at, at), character, TextSelection::new(at + 1, at + 1));
        editor.apply_exact_input_edit(&edit, false).unwrap();
    }
    assert_eq!(editor.document().text(), "abc");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.document().text(), "");
}

/// M6: an exact replacement of a range stays its own unit.
#[test]
fn an_exact_range_replacement_is_its_own_undo_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = editor("");
    editor.insert_text("abc", EditKind::Typing).unwrap();
    let edit = TextInputMethodEdit::new(TextSelection::new(0, 3), "x", TextSelection::new(1, 1));
    editor.apply_exact_input_edit(&edit, false).unwrap();
    assert!(editor.undo().unwrap());
    assert_eq!(editor.document().text(), "abc");
}

/// S1: Backspace at a collapsed caret joins the typing before it, as RichEdit groups
/// typing "including characters deleted by using the BackSpace key".
#[test]
fn backspace_merges_with_typing() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = editor("");
    for character in ["h", "e", "l", "l", "o"] {
        editor.insert_text(character, EditKind::Typing).unwrap();
    }
    for _ in 0..3 {
        editor.delete_backward(false).unwrap();
    }
    assert_eq!(editor.document().text(), "he");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.document().text(), "");
    assert!(!editor.can_undo());
}

/// S1: Delete (forward) stays a break between typing units.
#[test]
fn forward_delete_is_its_own_undo_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = editor("");
    editor.insert_text("ab", EditKind::Typing).unwrap();
    editor.set_selection(EditorSelection::caret(0)).unwrap();
    editor.delete_forward(false).unwrap();
    assert_eq!(editor.document().text(), "b");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.document().text(), "ab");
}

/// S6: removing a paragraph mark with Backspace keeps the format of the trailing
/// paragraph, as typing over the mark does.
#[test]
fn backspace_over_a_paragraph_mark_keeps_the_trailing_paragraph_format() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = editor("one\ntwo");
    let document = editor.document();
    let left = ParagraphFormat { text_alignment: Some(TextAlignment::Left), ..ParagraphFormat::default() };
    let right = ParagraphFormat { text_alignment: Some(TextAlignment::Right), ..ParagraphFormat::default() };
    document.apply_paragraph_format(0..3, &left).unwrap();
    document.apply_paragraph_format(4..7, &right).unwrap();
    editor.set_selection(EditorSelection::caret(4)).unwrap();
    editor.delete_backward(false).unwrap();
    let snapshot = document.snapshot();
    assert_eq!(snapshot.paragraphs.len(), 1);
    assert_eq!(snapshot.paragraphs[0].format.text_alignment, Some(TextAlignment::Right));
}

/// N3: lowering the undo limit limits the redo stack too.
#[test]
fn the_undo_limit_limits_redo() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = editor("");
    for character in ["a", "b", "c"] {
        editor.insert_text(character, EditKind::Typing).unwrap();
        editor.checkpoint();
    }
    while editor.undo().unwrap() {}
    assert_eq!(editor.document().text(), "");
    editor.set_undo_limit(1);
    assert!(editor.redo().unwrap());
    assert!(!editor.redo().unwrap());
}
