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

// S8 (#439): undo units are deltas of the blocks an edit replaced. Each edit of the editor, its
// undo and its redo give what the reference gives, whose units hold the whole document (the
// path before S8 and F1c: a document with `set_full_rebuild` records snapshots of the document).

/// An editor of the incremental document and one of the reference.
fn editors(build: impl Fn() -> RichTextDocument) -> [(Probe, RichTextEditor); 2] {
    let (incremental, reference) = probes(build);
    [incremental, reference].map(|probe| {
        let editor = RichTextEditor::new(probe.document.clone());
        (probe, editor)
    })
}

/// Runs `step` through the editors of both documents, then its undo, its redo and its undo
/// again, and compares the documents (snapshot, text, change events, pointers) and the editors
/// (selection, history) after each.
fn check_editor(
    context: &str,
    step: impl Fn(&RichTextEditor) -> Result<bool, crate::rich_text::EditError>,
) {
    let editors = editors(rich_document);
    let original = editors[0].0.document.snapshot();
    editor_step(context, &editors, &step);
    let edited = editors[0].0.document.snapshot();
    editor_step(&format!("{context}: undo"), &editors, |editor| editor.undo());
    assert_eq!(editors[0].0.document.snapshot(), original, "{context}: undo");
    editor_step(&format!("{context}: redo"), &editors, |editor| editor.redo());
    assert_eq!(editors[0].0.document.snapshot(), edited, "{context}: redo");
    editor_step(&format!("{context}: undo again"), &editors, |editor| editor.undo());
    assert_eq!(editors[0].0.document.snapshot(), original, "{context}: undo again");
}

#[test]
fn editor_breaks_and_typing_at_every_offset_undo_and_redo_like_the_snapshot_path() {
    let _scope = test_scope();
    let length = rich_document().text_length();
    for offset in 0..=length {
        check_editor(&format!("typed break at {offset}"), |editor| {
            editor.set_selection(EditorSelection::caret(offset))?;
            editor.insert_text("\n", EditKind::Typing)
        });
        check_editor(&format!("pasted breaks at {offset}"), |editor| {
            editor.set_selection(EditorSelection::caret(offset))?;
            editor.insert_text("x\n\ny", EditKind::Paste)
        });
        check_editor(&format!("typed character at {offset}"), |editor| {
            editor.set_selection(EditorSelection::caret(offset))?;
            editor.insert_text("z", EditKind::Typing)
        });
    }
}

#[test]
fn editor_merges_of_paragraphs_undo_and_redo_like_the_snapshot_path() {
    let _scope = test_scope();
    for mark in paragraph_marks(&rich_document()) {
        check_editor(&format!("backspace over {mark}"), |editor| {
            editor.set_selection(EditorSelection::caret(mark + 1))?;
            editor.delete_backward(false)
        });
        check_editor(&format!("delete of {mark}"), |editor| {
            editor.set_selection(EditorSelection::caret(mark))?;
            editor.delete_forward(false)
        });
        check_editor(&format!("typed over {mark}"), |editor| {
            editor.set_selection(EditorSelection::new(mark.saturating_sub(2), mark + 2))?;
            editor.insert_text("joined", EditKind::Typing)
        });
    }
}

#[test]
fn editor_replacements_across_paragraphs_undo_and_redo_like_the_snapshot_path() {
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
        check_editor(&format!("typed text over {start}..{end}"), |editor| {
            editor.set_selection(EditorSelection::new(start, end))?;
            editor.insert_text("z", EditKind::Typing)
        });
        check_editor(&format!("text over {start}..{end}"), |editor| {
            editor.set_selection(EditorSelection::new(start, end))?;
            editor.insert_text("x\ny", EditKind::Programmatic)
        });
        check_editor(&format!("deletion of {start}..{end}"), |editor| {
            editor.set_selection(EditorSelection::new(start, end))?;
            editor.delete_selection()
        });
        check_editor(&format!("fragment over {start}..{end}"), |editor| {
            editor.set_selection(EditorSelection::new(start, end))?;
            editor.insert_fragment(&DocumentSnapshot::from_text("p\nq\nr"), EditKind::Paste)
        });
        check_editor(&format!("rich fragment over {start}..{end}"), |editor| {
            let fragment = editor.document().slice(0..14)?;
            editor.set_selection(EditorSelection::new(start, end))?;
            editor.insert_fragment(&fragment, EditKind::Paste)
        });
    }
}

#[test]
fn editor_format_changes_undo_and_redo_like_the_snapshot_path() {
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
        check_editor(&format!("character format of {start}..{end}"), |editor| {
            editor.set_selection(EditorSelection::new(start, end))?;
            editor.apply_format(&bold).map(|()| true)
        });
        check_editor(&format!("paragraph format of {start}..{end}"), |editor| {
            editor.set_selection(EditorSelection::new(start, end))?;
            editor.apply_paragraph_format(&right).map(|()| true)
        });
    }
}

#[test]
fn editor_typing_groups_and_groups_undo_and_redo_like_the_snapshot_path() {
    let _scope = test_scope();
    let editors = editors(rich_document);
    let original = editors[0].0.document.snapshot();
    // A typing group: a character, a break, characters after it and Backspaces over the break
    // and into the paragraph before.
    editor_step("caret", &editors, |editor| {
        editor.set_selection(EditorSelection::caret(18))?;
        Ok(true)
    });
    for (index, character) in ["a", "\n", "b", "c"].into_iter().enumerate() {
        editor_step(&format!("typing {index}"), &editors, |editor| {
            editor.insert_text(character, EditKind::Typing)
        });
    }
    for index in 0..5 {
        editor_step(&format!("backspace {index}"), &editors, |editor| {
            editor.delete_backward(false)
        });
    }
    // An edit group over several paragraphs.
    editor_step("group", &editors, |editor| {
        editor.edit_group(|editor| {
            editor.set_selection(EditorSelection::caret(3))?;
            editor.insert_text("\n", EditKind::Typing)?;
            editor.set_selection(EditorSelection::new(30, 35))?;
            editor.apply_format(&CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..Default::default()
            })?;
            Ok(true)
        })
    });
    let edited = editors[0].0.document.snapshot();
    for index in 0..8 {
        editor_step(&format!("undo {index}"), &editors, |editor| editor.undo());
    }
    assert_eq!(editors[0].0.document.snapshot(), original);
    for index in 0..8 {
        editor_step(&format!("redo {index}"), &editors, |editor| editor.redo());
    }
    assert_eq!(editors[0].0.document.snapshot(), edited);
}

/// A document of `paragraphs` paragraphs "paragraph 0", "paragraph 1" and so on.
fn long_document(paragraphs: usize) -> RichTextDocument {
    let text: Vec<String> = (0..paragraphs)
        .map(|index| format!("paragraph {index}"))
        .collect();
    RichTextDocument::from_text(&text.join("\n"))
}

/// The UTF-16 offset of the start of a paragraph of `long_document` (its text is ASCII).
fn paragraph_start(document: &RichTextDocument, index: usize) -> usize {
    document
        .text()
        .split('\n')
        .take(index)
        .map(|paragraph| paragraph.len() + 1)
        .sum()
}

#[test]
fn an_undo_unit_of_typing_in_a_long_document_holds_the_edited_paragraph() {
    let _scope = test_scope();
    let document = long_document(1000);
    let editor = RichTextEditor::new(document.clone());
    let before = document.snapshot();
    let blocks = document.blocks().snapshot().to_vec();
    // The end of "paragraph 500".
    let caret = paragraph_start(&document, 501) - 1;
    editor.set_selection(EditorSelection::caret(caret)).unwrap();
    for character in ["a", "b", "c"] {
        assert!(editor.insert_text(character, EditKind::Typing).unwrap());
    }
    assert_eq!(
        editor.undo_unit_blocks(),
        vec![(500, 1, 1, false)],
        "one typing unit of the edited paragraph"
    );
    let edited = document.snapshot();
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), before);
    assert_eq!(editor.selection(), EditorSelection::caret(caret));
    let undone = document.blocks().snapshot().to_vec();
    assert_eq!(undone[..500], blocks[..500], "the paragraphs before keep their elements");
    assert_eq!(undone[501..], blocks[501..], "the paragraphs after keep their elements");
    assert!(editor.redo().unwrap());
    assert_eq!(document.snapshot(), edited);
    assert_eq!(editor.selection(), EditorSelection::caret(caret + 3));
    // A paragraph break in the middle of the paragraph, and its undo.
    editor.set_selection(EditorSelection::caret(caret - 2)).unwrap();
    assert!(editor.insert_text("\n", EditKind::Typing).unwrap());
    let units = editor.undo_unit_blocks();
    assert_eq!(units.len(), 2);
    assert!(units[1].1 <= 1 && units[1].2 <= 2 && !units[1].3, "{units:?}");
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), edited);
    let undone = document.blocks().snapshot().to_vec();
    assert_eq!(undone[..500], blocks[..500]);
    assert_eq!(undone[501..], blocks[501..]);
}

#[test]
fn a_typing_group_across_a_paragraph_break_holds_the_union_of_its_paragraphs() {
    let _scope = test_scope();
    let document = long_document(1000);
    let editor = RichTextEditor::new(document.clone());
    let before = document.snapshot();
    // A character typed at the start of "paragraph 500" and two Backspaces, the second over
    // the mark of "paragraph 499": one typing unit (#243 review S1).
    let start = paragraph_start(&document, 500);
    editor.set_selection(EditorSelection::caret(start)).unwrap();
    assert!(editor.insert_text("x", EditKind::Typing).unwrap());
    assert!(editor.delete_backward(false).unwrap());
    assert!(editor.delete_backward(false).unwrap());
    assert_eq!(document.paragraphs().len(), 999);
    assert_eq!(
        editor.undo_unit_blocks(),
        vec![(499, 2, 1, false)],
        "the unit holds the two paragraphs before and the merged one after"
    );
    let merged = document.snapshot();
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), before);
    assert_eq!(editor.selection(), EditorSelection::caret(start));
    assert!(!editor.can_undo());
    assert!(editor.redo().unwrap());
    assert_eq!(document.snapshot(), merged);
    assert_eq!(editor.selection(), EditorSelection::caret(start - 1));
    // A break typed after it starts a unit of its own, which typing after the break joins.
    assert!(editor.insert_text("\n", EditKind::Typing).unwrap());
    assert!(editor.insert_text("y", EditKind::Typing).unwrap());
    let units = editor.undo_unit_blocks();
    assert_eq!(units.len(), 2, "{units:?}");
    assert!(units[1].0 >= 499, "{units:?}");
    assert!(units[1].1 <= 1 && units[1].2 <= 2 && !units[1].3, "{units:?}");
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), merged);
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), before);
}

#[test]
fn an_external_change_still_clears_the_history_of_delta_units() {
    let _scope = test_scope();
    let document = long_document(10);
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::caret(3)).unwrap();
    assert!(editor.insert_text("x", EditKind::Typing).unwrap());
    assert!(editor.can_undo());
    // An edit of the document outside the editor invalidates the history: an undo cannot put
    // back blocks over the application's change.
    document.replace_text(0..0, "app ").unwrap();
    assert!(!editor.can_undo());
    assert!(!editor.undo().unwrap());
    assert!(document.text().starts_with("app parxagraph 0\n"));
    // So does a write to an element of the tree.
    editor.set_selection(EditorSelection::caret(0)).unwrap();
    assert!(editor.insert_text("y", EditKind::Typing).unwrap());
    assert!(editor.can_undo());
    let run = document.paragraphs()[5].inlines().snapshot()[0]
        .cast::<Run>()
        .unwrap();
    run.set_text(Some("changed"));
    assert!(!editor.can_undo());
    assert!(!editor.undo().unwrap());
    assert!(document.text().starts_with("yapp parxagraph 0\n"));
    // A later edit records a unit again, which undoes only that edit.
    let changed = document.snapshot();
    editor.set_selection(EditorSelection::caret(1)).unwrap();
    assert!(editor.insert_text("z", EditKind::Typing).unwrap());
    assert_eq!(editor.undo_unit_blocks(), vec![(0, 1, 1, false)]);
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), changed);
    assert!(!editor.can_undo());
}

#[test]
fn a_write_around_the_document_during_an_edit_keeps_the_edit_and_clears_the_history() {
    // A change of an element of the tree that is not an edit of the document (here from a
    // handler of its changes, while an edit is recorded) is one the journal of the edit cannot
    // follow: the edit and the change stay, and the history is cleared, as for a change outside
    // the editor (S8).
    let _scope = test_scope();
    let document = long_document(3);
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::caret(0)).unwrap();
    assert!(editor.insert_text("a", EditKind::Typing).unwrap());
    editor.checkpoint();
    let run = document.paragraphs()[2].inlines().snapshot()[0]
        .cast::<Run>()
        .unwrap();
    let armed = Rc::new(Cell::new(true));
    let trigger = armed.clone();
    let _subscription = document.changed(move |_| {
        if trigger.replace(false) {
            run.set_text(Some("changed"));
        }
    });
    assert!(editor.insert_text("b", EditKind::Typing).unwrap());
    assert!(!armed.get());
    assert_eq!(document.text(), "abparagraph 0\nparagraph 1\nchanged");
    assert!(!editor.can_undo());
}
