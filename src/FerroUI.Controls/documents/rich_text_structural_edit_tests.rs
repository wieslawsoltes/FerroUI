//! F1c (#439): structural edits without a rebuild of the document. A paragraph break, a merge,
//! a replacement across paragraphs, a character or paragraph format and a restore (undo) read
//! and rebuild the paragraphs they change; their result is the result of the path through a
//! snapshot of the document that rebuilds every block (`set_full_rebuild`, the path before F1c).

use super::*;
use crate::rich_text::{EditKind, EditorSelection, RichTextEditor};
use crate::test_support::test_scope;
use ferroui_base::media::{FontWeight, TextAlignment};

/// Five paragraphs: plain text, a bold span in a centred paragraph, an empty paragraph, a soft
/// line break, a hyperlink and a surrogate pair, and a paragraph with a font size.
fn rich_document() -> RichTextDocument {
    let document = RichTextDocument::new();
    document.blocks().add(Paragraph::with_text("alpha beta"));
    let mixed = Paragraph::new();
    mixed.add_child_text("one ");
    let bold = Bold::new();
    bold.add_child_text("two");
    mixed.inlines().add(bold);
    mixed.add_child_text(" three");
    mixed.set_text_alignment(TextAlignment::Center);
    document.blocks().add(mixed);
    document.blocks().add(Paragraph::new());
    let linked = Paragraph::new();
    linked.add_child_text("four");
    linked.inlines().add(LineBreak::new());
    linked
        .inlines()
        .add(Hyperlink::with_text("five", Some("https://example.test/")));
    linked.add_child_text("\u{1F600}six");
    document.blocks().add(linked);
    let styled = Paragraph::with_text("seven eight");
    styled.set_font_size(20.0);
    document.blocks().add(styled);
    document
}

/// A document with what observes it: every change event and a pointer of each gravity at
/// every offset.
struct Probe {
    document: RichTextDocument,
    events: Rc<RefCell<Vec<DocumentChangedEventArgs>>>,
    pointers: Vec<TextPointer>,
    _subscription: Subscription,
}

fn probe(document: RichTextDocument) -> Probe {
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let subscription = document.changed(move |change| sink.borrow_mut().push(change.clone()));
    let pointers = (0..=document.text_length())
        .flat_map(|offset| {
            [TextGravity::Forward, TextGravity::Backward]
                .map(|gravity| document.create_pointer(offset, gravity))
        })
        .filter_map(Result::ok)
        .collect();
    Probe {
        document,
        events,
        pointers,
        _subscription: subscription,
    }
}

/// The incremental document (F1c) and the reference that rebuilds every block.
fn probes(build: impl Fn() -> RichTextDocument) -> (Probe, Probe) {
    let incremental = probe(build());
    let reference = probe(build());
    reference.document.set_full_rebuild(true);
    (incremental, reference)
}

fn assert_same(incremental: &Probe, reference: &Probe, context: &str) {
    let document = &incremental.document;
    assert_eq!(
        document.snapshot(),
        reference.document.snapshot(),
        "{context}"
    );
    assert_eq!(document.text(), reference.document.text(), "{context}");
    assert_eq!(
        document.text(),
        document.blocks().text(),
        "{context}: observed text"
    );
    assert_eq!(
        document.text_length(),
        reference.document.text_length(),
        "{context}"
    );
    assert_eq!(
        *incremental.events.borrow(),
        *reference.events.borrow(),
        "{context}: change events"
    );
    let offsets =
        |probe: &Probe| -> Vec<usize> { probe.pointers.iter().map(TextPointer::offset).collect() };
    assert_eq!(
        offsets(incremental),
        offsets(reference),
        "{context}: pointers"
    );
}

/// Runs `edit` on both documents and compares the results.
fn check(context: &str, edit: impl Fn(&RichTextDocument) -> Result<(), DocumentError>) {
    let (incremental, reference) = probes(rich_document);
    let expected = edit(&reference.document);
    assert_eq!(edit(&incremental.document), expected, "{context}");
    assert_same(&incremental, &reference, context);
}

fn paragraph_marks(document: &RichTextDocument) -> Vec<usize> {
    document
        .text()
        .encode_utf16()
        .enumerate()
        .filter(|(_, unit)| *unit == u16::from(b'\n'))
        .map(|(offset, _)| offset)
        .collect()
}

#[test]
fn a_paragraph_break_at_every_offset_matches_the_snapshot_path() {
    let _scope = test_scope();
    let length = rich_document().text_length();
    for offset in 0..=length {
        check(&format!("break at {offset}"), |document| {
            document.replace_text(offset..offset, "\n")
        });
        check(&format!("typed break at {offset}"), |document| {
            document.replace_typed_text(offset..offset, "\n")
        });
        check(&format!("two breaks at {offset}"), |document| {
            document.replace_text(offset..offset, "x\n\ny")
        });
    }
}

#[test]
fn a_merge_of_paragraphs_matches_the_snapshot_path() {
    let _scope = test_scope();
    for mark in paragraph_marks(&rich_document()) {
        check(&format!("backspace over {mark}"), |document| {
            document.replace_typed_text(mark..mark + 1, "")
        });
        check(&format!("delete of {mark}"), |document| {
            document.replace_text(mark..mark + 1, "")
        });
        check(&format!("replacement of {mark}"), |document| {
            document.replace_text(mark.saturating_sub(2)..mark + 2, "joined")
        });
    }
}

#[test]
fn replacements_across_paragraphs_match_the_snapshot_path() {
    let _scope = test_scope();
    let length = rich_document().text_length();
    for (start, end) in [
        (0, length),
        (2, length - 2),
        (5, 20),
        (11, 23),
        (22, 24),
        (3, 12),
    ] {
        check(&format!("text over {start}..{end}"), |document| {
            document.replace_text(start..end, "x\ny")
        });
        check(&format!("typed text over {start}..{end}"), |document| {
            document.replace_typed_text(start..end, "z")
        });
        check(&format!("fragment over {start}..{end}"), |document| {
            document.replace_range(start..end, &DocumentSnapshot::from_text("p\nq\nr"))
        });
        check(&format!("rich fragment over {start}..{end}"), |document| {
            let fragment = document.slice(0..14)?;
            document.replace_range(start..end, &fragment)
        });
    }
}

#[test]
fn format_changes_match_the_snapshot_path() {
    let _scope = test_scope();
    let length = rich_document().text_length();
    let bold = CharacterFormat {
        font_weight: Some(FontWeight::Bold),
        ..Default::default()
    };
    let right = ParagraphFormat {
        text_alignment: Some(TextAlignment::Right),
        ..Default::default()
    };
    for (start, end) in [
        (0, length),
        (3, 15),
        (10, 11),
        (11, 25),
        (24, 25),
        (15, 18),
        (6, 6),
    ] {
        check(&format!("character format of {start}..{end}"), |document| {
            document.apply_format(start..end, &bold)
        });
        check(&format!("paragraph format of {start}..{end}"), |document| {
            document.apply_paragraph_format(start..end, &right)
        });
    }
}

#[test]
fn edits_in_one_batch_and_restores_match_the_snapshot_path() {
    let _scope = test_scope();
    let (incremental, reference) = probes(rich_document);
    let original = rich_document().snapshot();
    for probe in [&incremental, &reference] {
        let document = &probe.document;
        {
            let _batch = document.begin_edit();
            document.replace_text(14..14, "\n").unwrap();
            document.replace_text(15..15, "typed").unwrap();
            document
                .apply_format(
                    2..30,
                    &CharacterFormat {
                        font_size: Some(30.0),
                        ..Default::default()
                    },
                )
                .unwrap();
        }
        let edited = document.snapshot();
        document.restore(&original).unwrap();
        assert_eq!(document.snapshot(), original);
        document.restore(&edited).unwrap();
        document.replace_typed_text(10..12, "").unwrap();
    }
    assert_same(&incremental, &reference, "batch and restores");
}

#[test]
fn incremental_edits_keep_the_paragraphs_they_do_not_change() {
    let _scope = test_scope();
    let document = rich_document();
    let original = document.snapshot();
    let before = document.blocks().snapshot().to_vec();
    // "alpha beta\none |two three": the break splits the second paragraph.
    document.replace_text(15..15, "\n").unwrap();
    let split = document.blocks().snapshot().to_vec();
    assert_eq!(split.len(), before.len() + 1);
    assert_eq!(split[0], before[0]);
    assert_eq!(split[1], before[1], "the first part keeps the paragraph");
    assert_eq!(split[3..], before[2..]);
    // An undo through a snapshot rebuilds what changed: the original paragraphs are back.
    document.restore(&original).unwrap();
    assert_eq!(document.blocks().snapshot().to_vec(), before);
    // A merge and a format change of one paragraph leave the others.
    document.replace_typed_text(10..11, "").unwrap();
    let merged = document.blocks().snapshot().to_vec();
    assert_eq!(merged[0], before[0]);
    assert_eq!(merged[1..], before[2..]);
    document
        .apply_format(
            0..3,
            &CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..Default::default()
            },
        )
        .unwrap();
    document
        .apply_paragraph_format(
            30..30,
            &ParagraphFormat {
                text_alignment: Some(TextAlignment::Right),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(document.blocks().snapshot().to_vec(), merged);
}

#[test]
fn a_break_in_a_paragraph_with_a_control_keeps_the_control_and_rejects_a_duplicate() {
    let _scope = test_scope();
    let document = RichTextDocument::from_text("first");
    let paragraph = Paragraph::new();
    let control = crate::Button::new();
    let container = InlineUIContainer::with_child(control.clone());
    paragraph.add_child_text("ab");
    paragraph.inlines().add(container.clone());
    paragraph.add_child_text("cd");
    document.blocks().add(paragraph);
    document.replace_text(9..9, "\n").unwrap();
    assert_eq!(document.text(), "first\nab\u{fffc}\ncd");
    let child = document.paragraphs()[1]
        .inlines()
        .snapshot()
        .iter()
        .find_map(|inline| inline.cast::<InlineUIContainer>())
        .and_then(|container| container.child());
    assert_eq!(child, Some(control.clone().upcast::<Control>()));
    // A copy of the control into another paragraph is a duplicate, as through a snapshot.
    let fragment = document.slice(6..9).unwrap();
    let before = document.snapshot();
    assert_eq!(
        document.replace_range(0..0, &fragment),
        Err(DocumentError::DuplicateEmbeddedControl)
    );
    assert_eq!(document.snapshot(), before);
}

/// Runs the same editor step on both documents and compares them and the editors.
fn editor_step(
    context: &str,
    editors: &[(Probe, RichTextEditor); 2],
    step: impl Fn(&RichTextEditor) -> Result<bool, crate::rich_text::EditError>,
) {
    let [(incremental, incremental_editor), (reference, reference_editor)] = editors;
    let expected = format!("{:?}", step(reference_editor));
    assert_eq!(
        format!("{:?}", step(incremental_editor)),
        expected,
        "{context}"
    );
    assert_same(incremental, reference, context);
    assert_eq!(
        incremental_editor.selection(),
        reference_editor.selection(),
        "{context}"
    );
    assert_eq!(
        incremental_editor.can_undo(),
        reference_editor.can_undo(),
        "{context}"
    );
    assert_eq!(
        incremental_editor.can_redo(),
        reference_editor.can_redo(),
        "{context}"
    );
}

#[test]
fn editor_structural_edits_undo_and_redo_like_the_snapshot_path() {
    let _scope = test_scope();
    let (incremental, reference) = probes(rich_document);
    let original = incremental.document.snapshot();
    let editors = [incremental, reference].map(|probe| {
        let editor = RichTextEditor::new(probe.document.clone());
        (probe, editor)
    });
    let bold = CharacterFormat {
        font_weight: Some(FontWeight::Bold),
        ..Default::default()
    };
    let right = ParagraphFormat {
        text_alignment: Some(TextAlignment::Right),
        ..Default::default()
    };
    let steps: Vec<(
        &str,
        Box<dyn Fn(&RichTextEditor) -> Result<bool, crate::rich_text::EditError>>,
    )> = vec![
        (
            "break",
            Box::new(|editor: &RichTextEditor| {
                editor.set_selection(EditorSelection::caret(15))?;
                editor.insert_text("\n", EditKind::Typing)
            }),
        ),
        (
            "typing after the break",
            Box::new(|editor: &RichTextEditor| editor.insert_text("x", EditKind::Typing)),
        ),
        (
            "backspace over the break",
            Box::new(|editor: &RichTextEditor| {
                editor.set_selection(EditorSelection::caret(16))?;
                editor.delete_backward(false)
            }),
        ),
        (
            "delete of a mark",
            Box::new(|editor: &RichTextEditor| {
                editor.set_selection(EditorSelection::caret(10))?;
                editor.delete_forward(false)
            }),
        ),
        (
            "character format across paragraphs",
            Box::new(move |editor: &RichTextEditor| {
                editor.set_selection(EditorSelection::new(5, 30))?;
                editor.apply_format(&bold).map(|()| true)
            }),
        ),
        (
            "paragraph format",
            Box::new(move |editor: &RichTextEditor| {
                editor.apply_paragraph_format(&right).map(|()| true)
            }),
        ),
        (
            "typed break over a selection",
            Box::new(|editor: &RichTextEditor| {
                editor.set_selection(EditorSelection::new(3, 25))?;
                editor.insert_text("\n", EditKind::Typing)
            }),
        ),
    ];
    for (context, step) in &steps {
        editor_step(context, &editors, step);
    }
    let edited = editors[0].0.document.snapshot();
    for index in 0..steps.len() {
        editor_step(&format!("undo {index}"), &editors, |editor| editor.undo());
    }
    assert_eq!(editors[0].0.document.snapshot(), original);
    for index in 0..steps.len() {
        editor_step(&format!("redo {index}"), &editors, |editor| editor.redo());
    }
    assert_eq!(editors[0].0.document.snapshot(), edited);
}

#[test]
fn a_paragraph_break_and_its_undo_restore_the_document_and_selection() {
    let _scope = test_scope();
    let document = rich_document();
    let editor = RichTextEditor::new(document.clone());
    let before = document.snapshot();
    editor.set_selection(EditorSelection::caret(4)).unwrap();
    assert!(editor.insert_text("\n", EditKind::Typing).unwrap());
    assert_eq!(document.text().split('\n').next(), Some("alph"));
    assert_eq!(editor.selection(), EditorSelection::caret(5));
    let after = document.snapshot();
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), before);
    assert_eq!(editor.selection(), EditorSelection::caret(4));
    assert!(editor.redo().unwrap());
    assert_eq!(document.snapshot(), after);
    assert_eq!(editor.selection(), EditorSelection::caret(5));
}

#[test]
fn breaks_and_merges_of_a_paragraph_keep_its_inlines_and_inheritance_children_bounded() {
    // #439: every break and merge of a paragraph added empty runs to it (the run a replacement
    // carries its format in, and the one each part kept), so its inlines, and the inheritance
    // children of the paragraph that each removal and addition walks, grew with every edit.
    let _scope = test_scope();
    let view = crate::RichTextBlock::new();
    for text in ["first paragraph", "second paragraph", "third paragraph"] {
        view.blocks().add(Paragraph::with_text(text));
    }
    let document = view.rich_document();
    let editor = RichTextEditor::new(document.clone());
    let text = document.text();
    // "first paragraph\nsecond| paragraph\nthird paragraph", as the editing benchmark does.
    let cycle = || {
        editor.set_selection(EditorSelection::caret(22)).unwrap();
        assert!(editor.insert_text("\n", EditKind::Typing).unwrap());
        assert!(editor.delete_backward(false).unwrap());
    };
    cycle();
    let settled = document.snapshot();
    for _ in 0..200 {
        cycle();
        assert_eq!(document.text(), text);
    }
    assert_eq!(
        document.snapshot(),
        settled,
        "a break and its merge come back to one image"
    );
    let paragraphs = document.paragraphs();
    assert_eq!(paragraphs.len(), 3);
    for paragraph in &paragraphs {
        let inlines = paragraph.inlines().count();
        assert!(inlines <= 3, "{inlines} inlines");
        let entries = paragraph.inheritance_child_entries();
        assert!(
            entries <= 2 * inlines + 2,
            "{entries} inheritance children of {inlines} inlines"
        );
    }
    let entries = view.inheritance_child_entries();
    assert!(entries <= 16, "{entries} inheritance children of the view");
    // The same through the path before F1c, which shares how the paragraphs are made.
    let reference = RichTextDocument::from_text(&text);
    reference.set_full_rebuild(true);
    let editor = RichTextEditor::new(reference.clone());
    for _ in 0..200 {
        editor.set_selection(EditorSelection::caret(22)).unwrap();
        editor.insert_text("\n", EditKind::Typing).unwrap();
        editor.delete_backward(false).unwrap();
    }
    for paragraph in reference.paragraphs() {
        assert!(paragraph.inlines().count() <= 3);
        assert!(paragraph.inheritance_child_entries() <= 8);
    }
}
