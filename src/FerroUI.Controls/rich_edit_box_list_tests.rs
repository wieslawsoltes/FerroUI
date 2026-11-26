//! The keyboard list editing of RichEditBox (#1257): Enter, Backspace, Tab and Shift+Tab.
use crate::documents::{MarkerType, ParagraphFormat};
use crate::rich_edit_box_tests::rich_edit_box_template;
use crate::RichEditBox;
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};

fn press(editor: &RichEditBox, key: Key, modifiers: KeyModifiers) {
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.key_modifiers = modifiers;
    event.set_routed_event(Some(InputElement::key_down_event()));
    editor.raise_event(&event);
    assert!(event.handled());
}

fn list_editor(text: &str) -> (ferroui_base::Ref<RichEditBox>, ferroui_base::Ref<crate::test_support::TestRoot>) {
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let root = crate::test_support::TestRoot::with_child(editor.clone());
    editor.set_text(text).unwrap();
    editor.select(0, text.encode_utf16().count());
    editor
        .apply_paragraph_format(&ParagraphFormat { list_type: Some(MarkerType::Bullet), ..Default::default() })
        .unwrap();
    (editor, root)
}

fn types(editor: &RichEditBox) -> Vec<MarkerType> {
    editor.rich_document().paragraphs().iter().map(|paragraph| paragraph.list_type()).collect()
}

fn levels(editor: &RichEditBox) -> Vec<i32> {
    editor.rich_document().paragraphs().iter().map(|paragraph| paragraph.list_level_index()).collect()
}

#[test]
fn rich_edit_enter_continues_a_list_and_ends_it_in_an_empty_item() {
    let _scope = crate::test_support::test_scope();
    let (editor, _root) = list_editor("one");
    editor.select(3, 3);
    press(&editor, Key::Enter, KeyModifiers::empty());
    assert_eq!(editor.rich_document().text(), "one\n");
    assert_eq!(types(&editor), [MarkerType::Bullet, MarkerType::Bullet]);
    press(&editor, Key::Enter, KeyModifiers::empty());
    assert_eq!(editor.rich_document().text(), "one\n");
    assert_eq!(types(&editor), [MarkerType::Bullet, MarkerType::None]);
    editor.undo().unwrap();
    assert_eq!(types(&editor), [MarkerType::Bullet, MarkerType::Bullet]);
}

#[test]
fn rich_edit_backspace_at_the_start_of_a_list_item_removes_its_marker() {
    let _scope = crate::test_support::test_scope();
    let (editor, _root) = list_editor("one\ntwo");
    editor.select(4, 4);
    press(&editor, Key::Back, KeyModifiers::empty());
    assert_eq!(editor.rich_document().text(), "one\ntwo");
    assert_eq!(types(&editor), [MarkerType::Bullet, MarkerType::None]);
    press(&editor, Key::Back, KeyModifiers::empty());
    assert_eq!(editor.rich_document().text(), "onetwo");
}

#[test]
fn rich_edit_tab_indents_and_shift_tab_outdents_a_list_when_tabs_are_accepted() {
    let _scope = crate::test_support::test_scope();
    let (editor, _root) = list_editor("one\ntwo");
    editor.set_accepts_tab(true);
    editor.select(4, 4);
    press(&editor, Key::Tab, KeyModifiers::empty());
    assert_eq!(levels(&editor), [0, 2]);
    assert_eq!(editor.rich_document().text(), "one\ntwo");
    press(&editor, Key::Tab, KeyModifiers::SHIFT);
    assert_eq!(levels(&editor), [0, 1]);
    // Inside a list item's text Tab types a tab.
    editor.select(5, 5);
    press(&editor, Key::Tab, KeyModifiers::empty());
    assert_eq!(editor.rich_document().text(), "one\nt\two");
    // The same command without a key, on the selection of both items.
    editor.select(0, 9);
    assert!(editor.change_list_level(2).unwrap());
    assert_eq!(levels(&editor), [3, 3]);
}
