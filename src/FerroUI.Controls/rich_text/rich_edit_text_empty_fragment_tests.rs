//! Empty rich fragments preserve paragraph state and independent range defaults.
use super::*;
use crate::documents::LineSpacingRule;
use crate::rich_text::LetterCase;
use crate::RichEditBox;
use std::cell::{Cell, RefCell};

fn empty_rich_fragment() -> DocumentSnapshot {
    DocumentSnapshot {
        paragraphs: vec![ParagraphSnapshot {
            format: ParagraphFormat {
                character: CharacterFormat {
                    font_weight: Some(FontWeight::Bold),
                    font_size: Some(24.0),
                    ..CharacterFormat::default()
                },
                text_alignment: Some(TextAlignment::Center),
                line_spacing_rule: Some(LineSpacingRule::Exactly),
                line_spacing: Some(30.0),
                ..ParagraphFormat::default()
            },
            inlines: Vec::new(),
        }],
        ..Default::default()
    }
}

#[test]
fn rich_empty_fragment_replaces_only_the_empty_target_paragraph() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("left\nold\nright");
    document.selection().set_range(14, 10).unwrap();
    let before = document.document().snapshot();
    let fragment = empty_rich_fragment();
    document
        .get_range(5, 8)
        .unwrap()
        .set_snapshot(&fragment)
        .unwrap();
    let after = document.document().snapshot();
    assert_eq!(document.text(), "left\n\nright");
    assert_eq!(after.paragraphs[0], before.paragraphs[0]);
    assert_eq!(after.paragraphs[1], fragment.paragraphs[0]);
    assert_eq!(after.paragraphs[2], before.paragraphs[2]);
    assert_eq!(
        document.selection().selection(),
        EditorSelection::new(11, 7)
    );
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(
        document.selection().selection(),
        EditorSelection::new(14, 10)
    );
}

#[test]
fn rich_empty_fragment_at_nonempty_paragraph_sets_only_independent_typing_defaults() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    document.selection().set_range(4, 3).unwrap();
    let before = document.document().snapshot();
    let revision = document.document().revision();
    let range = document.get_range(2, 2).unwrap();
    range.set_snapshot(&empty_rich_fragment()).unwrap();

    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.document().revision(), revision);
    assert_eq!(document.selection().selection(), EditorSelection::new(4, 3));
    assert!(!document.can_undo());
    assert_eq!(
        range.character_format().unwrap(),
        TextFormatValue::Uniform(
            document
                .document()
                .character_format_at(2)
                .unwrap()
                .overlay(&empty_rich_fragment().paragraphs[0].format.character)
        )
    );
    range.set_text("X").unwrap();
    assert_eq!(document.text(), "abXcd");
    assert_eq!(
        document
            .document()
            .character_format_at(3)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(
        document
            .document()
            .character_format_at(3)
            .unwrap()
            .font_size,
        Some(24.0)
    );
    assert_eq!(
        document.document().snapshot().paragraphs[0].format,
        before.paragraphs[0].format
    );
    assert_eq!(
        document
            .document()
            .character_format_at(1)
            .unwrap()
            .font_weight,
        Some(FontWeight::Normal)
    );
    assert_eq!(
        document
            .document()
            .character_format_at(5)
            .unwrap()
            .font_weight,
        Some(FontWeight::Normal)
    );
}

#[test]
fn rich_empty_fragment_failed_outer_group_restores_document_selection_and_history() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("word");
    document.selection().set_range(3, 1).unwrap();
    let before = document.document().snapshot();
    let result: Result<(), EditError> = document.edit_group(|document| {
        document
            .document_range()?
            .set_snapshot(&empty_rich_fragment())?;
        Err(DocumentError::InvalidFormat("rollback").into())
    });
    assert!(result.is_err());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.selection().selection(), EditorSelection::new(3, 1));
    assert!(!document.can_undo());
    assert!(!document.can_redo());
}

#[test]
fn rich_independent_range_invalid_paragraph_patch_preserves_composition_preview() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("committed");
    document.selection().set_range(0, 9).unwrap();
    document.editor().set_preedit("preview", Some(3)).unwrap();
    let before = document.document().snapshot();
    let selection = document.selection().selection();
    let revision = document.document().revision();
    let range = document.document_range().unwrap();
    assert!(matches!(
        range.set_paragraph_format(&ParagraphFormat {
            text_indent: Some(f64::INFINITY),
            ..ParagraphFormat::default()
        }),
        Err(EditError::Document(DocumentError::InvalidFormat(_)))
    ));
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.document().revision(), revision);
    assert_eq!(document.selection().selection(), selection);
    assert!(document.editor().is_composing());
    assert!(!document.can_undo());
}
