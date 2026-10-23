//! Framework transaction and structured-content tests supplement the separately
//! recorded native TypeText alignment and endpoint observations.
use super::*;
use crate::documents::{Block, LineBreak, Paragraph, Run};
use crate::rich_text::RichEditTextDocument;
use crate::test_support::test_scope;
use crate::RichEditBox;
use ferroui_base::media::{FlowDirection, FontStyle, FontWeight, TextAlignment};
use std::cell::RefCell;

fn model(text: &str) -> RichTextDocument {
    let document = RichTextDocument::from_text(text);
    let paragraphs = document.paragraphs();
    paragraphs[0].set_text_alignment(TextAlignment::Left);
    paragraphs
        .last()
        .unwrap()
        .set_text_alignment(TextAlignment::Right);
    document
}

#[test]
fn rich_selection_paragraph_typing_protects_structural_mark_in_both_utf16_directions() {
    let _scope = test_scope();
    for (anchor, active, expected) in [
        (3, 4, "a😀X\nz"),
        (4, 3, "a😀X\nz"),
        (1, 4, "aX\nz"),
        (4, 1, "aX\nz"),
        (0, 4, "X\nz"),
        (4, 0, "X\nz"),
    ] {
        let document = model("a😀\nz");
        let editor = RichTextEditor::new(document.clone());
        let selection = EditorSelection::new(anchor, active);
        editor.set_selection(selection).unwrap();
        let before = document.snapshot();
        assert!(editor.type_text("X").unwrap());
        assert_eq!(document.text(), expected);
        assert_eq!(
            document.paragraphs()[0].text_alignment(),
            TextAlignment::Left
        );
        assert_eq!(
            document.paragraphs()[1].text_alignment(),
            TextAlignment::Right
        );
        assert_eq!(
            editor.selection(),
            EditorSelection::caret(anchor.min(active) + 1)
        );
        let after = document.snapshot();
        assert!(editor.undo().unwrap());
        assert_eq!(document.snapshot(), before);
        assert_eq!(editor.selection(), selection);
        assert!(!editor.can_undo());
        assert!(editor.redo().unwrap());
        assert_eq!(document.snapshot(), after);
    }
}

#[test]
fn rich_selection_paragraph_typing_retains_inherited_alignment_without_baking_a_default() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("ab\ncd");
    let paragraphs = document.paragraphs();
    for paragraph in &paragraphs {
        paragraph.set_flow_direction(FlowDirection::LeftToRight);
    }
    paragraphs[0].set_text_alignment(TextAlignment::Right);
    paragraphs[0].set_left_indent(14.0);
    paragraphs[1].set_left_indent(28.0);
    assert_eq!(
        paragraphs[1].get_base_value(Block::text_alignment_property()),
        None
    );
    let before = document.snapshot();
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(1, 4)).unwrap();
    assert!(editor.type_text("X").unwrap());
    let paragraph = document.paragraphs()[0].clone();
    assert_eq!(
        paragraph.get_base_value(Block::text_alignment_property()),
        None
    );
    // TextBlock's inherited default is logical Start. TextLineImpl resolves it
    // to physical Left for this explicitly LTR fixture, without storing Left
    // in the paragraph's local formatting or snapshot.
    assert_eq!(paragraph.text_alignment(), TextAlignment::Start);
    assert_eq!(paragraph.flow_direction(), FlowDirection::LeftToRight);
    assert_eq!(
        document.snapshot().paragraphs[0].format.text_alignment,
        None
    );
    // Native alignment evidence does not silently redefine unmeasured indents.
    assert_eq!(paragraph.left_indent(), 14.0);
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), before);
    assert!(editor.redo().unwrap());
    assert_eq!(
        document.paragraphs()[0].get_base_value(Block::text_alignment_property()),
        None
    );
}

#[test]
fn rich_selection_paragraph_typing_applies_alignment_to_every_inserted_paragraph() {
    let _scope = test_scope();
    for text in ["\r", "\n", "\r\n", "Q\r\nR"] {
        let document = model("ab\ncd\nef");
        let editor = RichTextEditor::new(document.clone());
        editor.set_selection(EditorSelection::new(7, 1)).unwrap();
        assert!(editor.type_text(text).unwrap());
        assert_eq!(document.text(), format!("a{}f", text.replace('\r', "\n")));
        for paragraph in document.paragraphs() {
            assert_eq!(paragraph.text_alignment(), TextAlignment::Right);
        }
        assert!(editor.undo().unwrap());
        assert_eq!(document.text(), "ab\ncd\nef");
        assert!(!editor.can_undo());
    }
}

#[test]
fn rich_selection_paragraph_typing_limit_counts_the_retained_mark_before_accepting_input() {
    let _scope = test_scope();
    let document = model("ab\ncd");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(2, 3)).unwrap();
    editor.set_max_length(5);
    let before = document.snapshot();
    assert!(!editor.type_text("X").unwrap());
    assert_eq!(document.snapshot(), before);
    assert_eq!(editor.selection(), EditorSelection::new(2, 3));
    assert!(!editor.can_undo());
    editor.set_max_length(6);
    assert!(!editor.type_text("😀").unwrap());
    assert_eq!(document.snapshot(), before);
    assert!(editor.type_text("X😀").unwrap());
    assert_eq!(document.text(), "abX\ncd");
    assert_eq!(editor.selection(), EditorSelection::caret(3));
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), before);
    assert_eq!(editor.selection(), EditorSelection::new(2, 3));
}

#[test]
fn rich_selection_paragraph_typing_limit_does_not_split_a_replacement_surrogate() {
    let _scope = test_scope();
    let document = model("a😀\nz");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(4, 1)).unwrap();
    editor.set_max_length(5);
    assert!(editor.type_text("😎Z").unwrap());
    assert_eq!(document.text(), "a😎\nz");
    assert_eq!(editor.selection(), EditorSelection::caret(3));
    assert!(editor.undo().unwrap());
    assert_eq!(editor.selection(), EditorSelection::new(4, 1));
}

#[test]
fn rich_selection_paragraph_typing_overtype_composes_with_retained_mark_and_history() {
    let _scope = test_scope();
    // Native observations independently establish selected-replacement typing
    // and OVERTYPE stopping at a paragraph mark. Their combination is a model
    // regression, not a separately measured native selected-mark fixture.
    for (anchor, active, limit, typed, expected, caret, changed) in [
        (3, 4, 0, "X😀", "a😀X😀\n😎z", 6, true),
        (4, 3, 0, "X😀", "a😀X😀\n😎z", 6, true),
        (1, 4, 0, "X😀", "aX😀\n😎z", 4, true),
        (4, 1, 0, "X😀", "aX😀\n😎z", 4, true),
        (3, 4, 8, "X😀", "a😀X\n😎z", 4, true),
        (4, 3, 8, "😀", "a😀\n😎z", 0, false),
    ] {
        let document = model("a😀\n😎z");
        let editor = RichTextEditor::new(document.clone());
        editor
            .set_selection_options(SelectionOptions::OVERTYPE | SelectionOptions::AT_END_OF_LINE)
            .unwrap();
        let selection = EditorSelection::new(anchor, active);
        editor.set_selection(selection).unwrap();
        editor.set_max_length(limit);
        let before = document.snapshot();
        assert_eq!(editor.type_text(typed).unwrap(), changed);
        assert_eq!(document.text(), expected);
        assert_eq!(
            editor.selection(),
            if changed {
                EditorSelection::caret(caret)
            } else {
                selection
            }
        );
        assert_eq!(
            editor
                .selection_options()
                .contains(SelectionOptions::AT_END_OF_LINE),
            !changed
        );
        assert!(editor
            .selection_options()
            .contains(SelectionOptions::OVERTYPE));
        assert_eq!(
            document.paragraphs()[0].text_alignment(),
            TextAlignment::Left
        );
        assert_eq!(
            document.paragraphs()[1].text_alignment(),
            TextAlignment::Right
        );
        let after = document.snapshot();
        assert_eq!(editor.undo().unwrap(), changed);
        assert_eq!(document.snapshot(), before);
        assert_eq!(editor.selection(), selection);
        assert!(!editor.can_undo());
        assert_eq!(editor.redo().unwrap(), changed);
        assert_eq!(document.snapshot(), after);
    }
}

#[test]
fn rich_selection_paragraph_typing_does_not_protect_inline_literal_or_soft_breaks() {
    let _scope = test_scope();
    for soft in [false, true] {
        let document = RichTextDocument::new();
        let paragraph = Paragraph::new();
        let end = if soft {
            paragraph.inlines().add(Run::with_text(Some("ab")));
            paragraph.inlines().add(LineBreak::new());
            let end = paragraph.text_length();
            paragraph.inlines().add(Run::with_text(Some("cd")));
            end
        } else {
            paragraph.inlines().add(Run::with_text(Some("ab\ncd")));
            3
        };
        document.blocks().add(paragraph);
        let editor = RichTextEditor::new(document.clone());
        editor.set_selection(EditorSelection::new(2, end)).unwrap();
        assert!(editor.type_text("X").unwrap());
        assert_eq!(document.text(), "abXcd");
        assert_eq!(document.paragraphs().len(), 1);
    }
}

#[test]
fn rich_selection_paragraph_typing_keeps_bulk_delete_and_composition_policies_separate() {
    let _scope = test_scope();
    for kind in [
        EditKind::Programmatic,
        EditKind::Paste,
        EditKind::Composition,
    ] {
        let document = model("ab\ncd");
        let editor = RichTextEditor::new(document.clone());
        editor.set_selection(EditorSelection::new(2, 3)).unwrap();
        assert!(editor.insert_text("X", kind).unwrap());
        assert_eq!(document.text(), "abXcd");
        assert_eq!(
            document.paragraphs()[0].text_alignment(),
            TextAlignment::Left
        );
    }
    let document = model("ab\ncd");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(2, 3)).unwrap();
    assert!(editor.delete_selection().unwrap());
    assert_eq!(document.text(), "abcd");
}

#[test]
fn rich_selection_paragraph_typing_publishes_complete_alignment_and_caret_once() {
    let _scope = test_scope();
    let document = model("ab\ncd");
    let editor = Rc::new(RichTextEditor::new(document.clone()));
    editor.set_selection(EditorSelection::new(4, 1)).unwrap();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let read = observed.clone();
    let weak = Rc::downgrade(&editor);
    document.changed(move |_| {
        let editor = weak.upgrade().unwrap();
        read.borrow_mut().push((
            editor.document().text(),
            editor.document().paragraphs()[0].text_alignment(),
            editor.selection(),
        ));
    });
    assert!(editor.type_text("😀").unwrap());
    assert_eq!(
        &*observed.borrow(),
        &[(
            "a😀d".to_owned(),
            TextAlignment::Right,
            EditorSelection::caret(3),
        )]
    );
}

#[test]
fn rich_selection_paragraph_typing_failed_group_restores_exact_document_and_history() {
    let _scope = test_scope();
    let document = model("ab\ncd");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(4, 1)).unwrap();
    let before = document.snapshot();
    let result: Result<(), EditError> = editor.edit_group(|editor| {
        editor.type_text("X")?;
        assert_eq!(
            editor.document().paragraphs()[0].text_alignment(),
            TextAlignment::Right
        );
        Err(DocumentError::InvalidFormat("Reject the outer transaction.").into())
    });
    assert!(result.is_err());
    assert_eq!(document.snapshot(), before);
    assert_eq!(editor.selection(), EditorSelection::new(4, 1));
    assert!(!editor.can_undo());
    assert!(!editor.can_redo());
}

#[test]
fn rich_selection_paragraph_typing_retains_explicit_typing_and_independent_range_modes() {
    let _scope = test_scope();
    let document = model("ab\ncd");
    let services = RichEditTextDocument::new(document.clone());
    let selection = services.selection();
    selection.set_range(4, 1).unwrap();
    selection.set_gravity(RangeGravity::Backward);
    let independent = services.get_range(1, 4).unwrap();
    independent.set_gravity(RangeGravity::Forward);
    assert!(services
        .editor()
        .edit_group(|editor| {
            editor.replace_range_typing_format(&CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            })?;
            editor.type_text("X")
        })
        .unwrap());
    let format = document.character_format_at(2).unwrap();
    assert_eq!(format.font_weight, Some(FontWeight::Bold));
    assert_eq!(format.font_style, Some(FontStyle::Italic));
    assert_eq!(selection.gravity(), RangeGravity::Backward);
    assert_eq!(independent.gravity(), RangeGravity::Forward);
    assert_eq!(
        document.paragraphs()[0].text_alignment(),
        TextAlignment::Right
    );
}

#[test]
fn rich_selection_paragraph_typing_is_used_by_the_real_control_and_shared_history() {
    let _scope = test_scope();
    let owner = RichEditBox::new();
    owner.set_document(model("ab\ncd"));
    owner.select(3, 2);
    let services = owner.text_document();
    services.clear_undo_redo_history();
    assert!(owner.insert_text("X").unwrap());
    assert_eq!(owner.text(), "abX\ncd");
    assert_eq!(services.selection().selection(), EditorSelection::caret(3));
    assert_eq!(
        owner.document().paragraphs()[0].text_alignment(),
        TextAlignment::Left
    );
    assert!(services.undo().unwrap());
    assert_eq!(owner.text(), "ab\ncd");
    assert_eq!(services.selection().selection(), EditorSelection::new(3, 2));
    assert!(!services.can_undo());
    assert!(services.redo().unwrap());
    assert_eq!(owner.text(), "abX\ncd");
}
