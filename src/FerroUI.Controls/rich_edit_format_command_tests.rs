use crate::documents::{
    CharacterFormat, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind, ParagraphFormat,
    ParagraphSnapshot, TextGravity,
};
use crate::rich_text::{EditError, EditorSelection};
use crate::{DisabledFormattingAccelerators, RichEditBox};
use ferroui_base::media::{
    FontStyle, FontWeight, TextDecorationCollection, TextDecorationLocation, TextDecorations,
};
use ferroui_base::threading::Dispatcher;
use std::cell::RefCell;
use std::rc::Rc;

fn run(text: &str, format: CharacterFormat) -> InlineSnapshot {
    InlineSnapshot {
        format,
        kind: InlineSnapshotKind::Run(text.to_owned()),
    }
}

fn mixed_editor() -> ferroui_base::Ref<RichEditBox> {
    let editor = RichEditBox::new();
    editor
        .rich_document()
        .restore(&DocumentSnapshot {
            paragraphs: vec![ParagraphSnapshot {
                format: ParagraphFormat::default(),
                inlines: vec![
                    run(
                        "ab",
                        CharacterFormat {
                            font_weight: Some(FontWeight::Bold),
                            font_style: Some(FontStyle::Italic),
                            text_decorations: Some(Some(TextDecorationCollection::from_items(
                                vec![
                                    TextDecorations::underline().get(0),
                                    TextDecorations::strikethrough().get(0),
                                ],
                            ))),
                            ..CharacterFormat::default()
                        },
                    ),
                    run(
                        "cd",
                        CharacterFormat {
                            font_style: Some(FontStyle::Italic),
                            text_decorations: Some(Some(TextDecorations::overline())),
                            ..CharacterFormat::default()
                        },
                    ),
                ],
            }],
            ..Default::default()
        })
        .unwrap();
    editor.select(4, 0);
    editor
}

fn decorations(editor: &RichEditBox, offset: usize) -> Vec<TextDecorationLocation> {
    // The probes address the start of each run, so inspect its forward side.
    // character_format_at is a caret-format query with backward affinity.
    let element = editor
        .rich_document()
        .element_at(offset, TextGravity::Forward)
        .expect("the probed text run");
    CharacterFormat::effective(&element)
        .text_decorations
        .flatten()
        .map_or_else(Vec::new, |items| {
            items
                .list()
                .snapshot()
                .iter()
                .map(|item| item.location())
                .collect()
        })
}

#[test]
fn rich_format_queries_report_mixed_attributes_independently() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = mixed_editor();
    assert_eq!(editor.bold_state(), None);
    assert_eq!(editor.italic_state(), Some(true));
    assert_eq!(editor.underline_state(), None);
    assert!(!editor.is_bold());
    assert!(editor.is_italic());
    editor.select(0, 2);
    assert_eq!(editor.bold_state(), Some(true));
    assert_eq!(editor.underline_state(), Some(true));
}

#[test]
fn rich_bold_command_makes_mixed_selection_uniform_and_preserves_direction_and_history() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = mixed_editor();
    let before = editor.rich_document().snapshot();
    editor.toggle_bold().unwrap();
    assert!(editor.is_bold());
    assert!(editor.is_italic());
    assert_eq!(editor.editor().selection(), EditorSelection::new(4, 0));
    assert!(editor.undo().unwrap());
    assert_eq!(editor.rich_document().snapshot(), before);
    assert_eq!(editor.bold_state(), None);
    assert!(!editor.can_undo());
    assert!(editor.redo().unwrap());
    assert!(editor.is_bold());
    editor.toggle_bold().unwrap();
    assert_eq!(editor.bold_state(), Some(false));
}

#[test]
fn rich_underline_command_preserves_each_runs_unrelated_decorations() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = mixed_editor();
    let before = editor.rich_document().snapshot();
    assert!(decorations(&editor, 2).contains(&TextDecorationLocation::Overline));
    editor.toggle_underline().unwrap();
    assert_eq!(editor.underline_state(), Some(true));
    assert!(decorations(&editor, 0).contains(&TextDecorationLocation::Strikethrough));
    assert!(decorations(&editor, 2).contains(&TextDecorationLocation::Overline));
    assert!(!decorations(&editor, 2).contains(&TextDecorationLocation::Strikethrough));
    let underlined = editor.rich_document().snapshot();
    editor.toggle_underline().unwrap();
    assert_eq!(editor.underline_state(), Some(false));
    assert_eq!(
        decorations(&editor, 0),
        vec![TextDecorationLocation::Strikethrough]
    );
    assert_eq!(
        decorations(&editor, 2),
        vec![TextDecorationLocation::Overline]
    );
    assert_eq!(editor.editor().selection(), EditorSelection::new(4, 0));
    assert!(editor.undo().unwrap());
    assert_eq!(editor.rich_document().snapshot(), underlined);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.rich_document().snapshot(), before);
    assert!(!editor.can_undo());
}

#[test]
fn rich_format_commands_update_collapsed_typing_format() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.toggle_bold().unwrap();
    editor.toggle_italic().unwrap();
    editor.toggle_underline().unwrap();
    assert_eq!(editor.bold_state(), Some(true));
    assert_eq!(editor.italic_state(), Some(true));
    assert_eq!(editor.underline_state(), Some(true));
    editor.insert_text("😀").unwrap();
    assert_eq!(editor.rich_document().text_length(), 2);
    assert_eq!(
        editor
            .rich_document()
            .character_format_at(0)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(
        editor.rich_document().character_format_at(0).unwrap().font_style,
        Some(FontStyle::Italic)
    );
    assert_eq!(
        decorations(&editor, 0),
        vec![TextDecorationLocation::Underline]
    );
}

#[test]
fn rich_format_commands_honor_read_only_and_keep_accelerator_policy_separate() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = mixed_editor();
    let before = editor.rich_document().snapshot();
    editor.set_is_read_only(true);
    assert!(!editor.can_format());
    assert!(matches!(editor.toggle_bold(), Err(EditError::ReadOnly)));
    assert!(matches!(editor.toggle_italic(), Err(EditError::ReadOnly)));
    assert!(matches!(
        editor.toggle_underline(),
        Err(EditError::ReadOnly)
    ));
    assert_eq!(editor.rich_document().snapshot(), before);
    assert!(!editor.can_undo());
    editor.set_is_read_only(false);
    editor.set_disabled_formatting_accelerators(DisabledFormattingAccelerators::BOLD);
    editor.toggle_bold().unwrap();
    assert!(editor.is_bold());
    editor.set_is_enabled(false);
    assert!(!editor.can_format());
}

#[test]
fn rich_format_commands_reject_reentrant_document_notifications() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = mixed_editor();
    let result = Rc::new(RefCell::new(None));
    let observed = result.clone();
    let weak = editor.downgrade();
    let subscription = editor.rich_document().changed(move |_| {
        if let Some(editor) = weak.upgrade() {
            *observed.borrow_mut() = Some(editor.toggle_italic());
        }
    });
    editor.toggle_bold().unwrap();
    assert!(matches!(
        *result.borrow(),
        Some(Err(EditError::ReentrantEdit))
    ));
    assert!(editor.is_italic());
    subscription.dispose();
}

#[test]
fn rich_format_command_cancels_preedit_before_querying_the_accepted_selection() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = mixed_editor();
    editor.set_preedit_text(Some("日本語"), Some(3)).unwrap();
    editor.toggle_bold().unwrap();
    assert!(!editor.editor().is_composing());
    assert_eq!(editor.text(), "abcd");
    assert!(editor.is_bold());
    assert!(editor.undo().unwrap());
    assert_eq!(editor.bold_state(), None);
}
