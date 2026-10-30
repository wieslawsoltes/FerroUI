use super::*;
use crate::documents::{
    CharacterFormat, Hyperlink, InlineSnapshotKind, Paragraph, RichTextDocument, Run, Span,
};
use crate::rich_text::{EditorSelection, LetterCase};
use ferroui_base::media::{IBrush, ISolidColorBrush, ITransform};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Ref, RelativePoint};
use std::any::Any;
use std::cell::Cell;
use std::collections::HashSet;

#[path = "rich_text_link_native_cases.rs"]
mod native_cases;

fn services(text: &str) -> RichEditTextDocument {
    RichEditTextDocument::new(RichTextDocument::from_text(text))
}
fn offset(value: usize) -> FieldStoryOffset {
    FieldStoryOffset::new(value)
}
fn positions(range: &RichEditTextFieldRange) -> Range<usize> {
    let range = range.range().unwrap();
    range.start.get()..range.end.get()
}
fn link(
    document: &RichEditTextDocument,
    range: Range<usize>,
    target: &str,
) -> RichEditTextFieldRange {
    let range = document
        .field_story()
        .get_range_from_visible(range)
        .unwrap();
    assert!(range.set_link(Some(&format!("\"{target}\""))).unwrap());
    range
}
fn links(document: &RichTextDocument) -> Vec<Ref<Hyperlink>> {
    fn visit(collection: &crate::documents::InlineCollection, result: &mut Vec<Ref<Hyperlink>>) {
        for inline in collection.snapshot().iter() {
            if let Some(link) = inline.cast::<Hyperlink>() {
                result.push(link);
            }
            if let Some(span) = inline.cast::<Span>() {
                visit(&span.inlines(), result);
            }
        }
    }
    let mut result = Vec::new();
    for paragraph in document.paragraphs() {
        visit(&paragraph.inlines(), &mut result);
    }
    result
}
fn assert_native(
    document: &RichEditTextDocument,
    range: &RichEditTextFieldRange,
    expected: &native_cases::NativeLinkSnapshot,
) {
    assert_eq!(positions(range), expected.range.0..expected.range.1);
    assert_eq!(document.field_story().text().unwrap(), expected.story);
    assert_eq!(
        document.field_story().text_length().unwrap(),
        expected.story_utf16_length
    );
    assert_eq!(range.text().unwrap(), expected.text);
    assert_eq!(range.utf16().unwrap().len(), expected.text_utf16_length);
    document.document().snapshot().validate().unwrap();
}
fn native_failure(error: TextFieldError) -> u32 {
    match error {
        TextFieldError::InvalidArgument => 0x80070057,
        TextFieldError::ParagraphOnly => 0x8000ffff,
        error => panic!("Unexpected measured transition failure: {error:?}"),
    }
}

#[test]
fn rich_field_live_replays_all_137_measured_link_transitions_with_observed_before_ranges() {
    let _scope = Dispatcher::unit_test_scope();
    assert_eq!(
        native_cases::SOURCE_SHA256,
        "8e0fc9f9605aefd02ea20949d1b9fd3305f427d1fb493f600969bcda16623987"
    );
    assert_eq!(
        native_cases::MEASURED_COMMIT,
        "dc99ba00a692c51391fa18a767f7d16a4f6f2415"
    );
    assert_eq!(
        native_cases::PROBE_GIT_BLOB,
        "4accd7fdb21888f9fe4843f3c957420ab2cc38ab"
    );
    assert_eq!(native_cases::ENGINE_VERSION, "6.2.26100.33438");
    let mut document = services("");
    let mut current = "";
    let mut fixtures = HashSet::new();
    let mut step = 0;
    let mut getters_moved = 0;
    let mut failed_setters = 0;
    for case in native_cases::CASES {
        if current != case.fixture {
            current = case.fixture;
            assert!(fixtures.insert(current));
            document = services(case.initial_text);
            document
                .selection()
                .set_range(document.text_length(), 0)
                .unwrap();
            step = 0;
        }
        assert_eq!(case.step, step, "{} {}", case.fixture, case.role);
        step += 1;
        // Setup ranges are supplied from the immutable oracle. This does not
        // assert independent native live-pointer rebasing between commands.
        let range = document
            .field_story()
            .get_range(offset(case.before.range.0), offset(case.before.range.1))
            .unwrap();
        assert_native(&document, &range, &case.before);
        let before = document.document().snapshot();
        let selection = document.editor().selection();
        let revision = document.document().revision();
        let mut result = None;
        let hresult = match case.operation {
            "Get" => match range.get_link() {
                Ok(value) => {
                    if positions(&range) != (case.before.range.0..case.before.range.1) {
                        getters_moved += 1;
                    }
                    result = value;
                    if result.is_some() {
                        0
                    } else {
                        1
                    }
                }
                Err(error) => native_failure(error),
            },
            "Set" => match range.set_link(case.argument) {
                Ok(_) => 0,
                Err(error) => {
                    failed_setters += 1;
                    native_failure(error)
                }
            },
            _ => unreachable!(),
        };
        assert_eq!(hresult, case.hresult, "{}:{}", case.fixture, case.step);
        assert_eq!(
            result.as_deref(),
            case.result,
            "{}:{}",
            case.fixture,
            case.step
        );
        assert_native(&document, &range, &case.after);
        assert_eq!(document.editor().selection(), selection);
        assert_eq!(document.text(), before.text());
        assert_eq!(case.before.selection, case.after.selection);
        if hresult & 0x80000000 != 0 || case.operation == "Get" {
            assert_eq!(document.document().snapshot(), before);
            assert_eq!(document.document().revision(), revision);
        }
    }
    assert_eq!(
        (
            native_cases::CASES.len(),
            fixtures.len(),
            getters_moved,
            failed_setters
        ),
        (137, 59, 15, 12)
    );
}

#[test]
fn rich_field_live_projection_keeps_visible_selection_and_typed_boundaries_separate() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("a😀b\ncd");
    document.selection().set_range(6, 1).unwrap();
    let visible = document.get_range(1, 3).unwrap();
    let range = link(&document, 1..3, "x");
    assert_eq!(document.text(), "a😀b\ncd");
    assert_eq!(visible.range(), 1..3);
    assert_eq!(document.editor().selection(), EditorSelection::new(6, 1));
    assert_eq!(
        document.field_story().text().unwrap(),
        "aHYPERLINK \"x\"😀b\rcd\r"
    );
    assert_eq!(range.visible_range().unwrap(), Some(1..3));
    let story = document.field_story();
    assert_eq!(
        story
            .project_visible(1, FieldInstructionBoundary::Before)
            .unwrap()
            .get(),
        1
    );
    assert_eq!(
        story
            .project_visible(1, FieldInstructionBoundary::After)
            .unwrap()
            .get(),
        14
    );
    let hidden = story.get_range(offset(2), offset(4)).unwrap();
    assert_eq!(hidden.visible_range().unwrap(), None);
    assert_eq!(hidden.text().unwrap(), "YP");
    assert!(matches!(
        hidden.get_link(),
        Err(TextFieldError::Unsupported(_))
    ));
    assert_eq!(document.editor().selection(), EditorSelection::new(6, 1));
}

#[test]
fn rich_field_live_raw_surrogate_endpoint_is_readable_and_explicitly_snapshot_bound() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("a😀b");
    let raw = document
        .field_story()
        .get_range(offset(2), offset(3))
        .unwrap();
    assert_eq!(raw.utf16().unwrap(), vec![0xde00]);
    assert!(matches!(
        raw.text(),
        Err(TextFieldError::InvalidUtf16Boundary)
    ));
    assert!(matches!(
        raw.set_link(Some("\"x\"")),
        Err(TextFieldError::Unsupported(_))
    ));
    document.get_range(4, 4).unwrap().set_text("!").unwrap();
    assert!(matches!(raw.utf16(), Err(TextFieldError::StaleRange)));
}

#[test]
fn rich_field_live_history_restores_exact_fields_sentinel_selection_and_typing_patch() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("alpha beta");
    document.selection().set_range(8, 8).unwrap();
    document
        .editor()
        .apply_format(&CharacterFormat {
            font_size: Some(27.0),
            ..Default::default()
        })
        .unwrap();
    let typing = document.editor().typing_format();
    document.clear_undo_redo_history();
    let before = document.document().snapshot();
    let range = document.field_story().get_range_from_visible(0..5).unwrap();
    assert!(range
        .set_link(Some("\u{fddf}\"https://example.test\""))
        .unwrap());
    let created = document.document().snapshot();
    assert!(created.link_fields[0].has_sentinel);
    assert_eq!(created.link_fields[0].range, 0..5);
    assert_eq!(document.editor().typing_format(), typing);
    assert_eq!(document.editor().selection(), EditorSelection::caret(8));
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.editor().typing_format(), typing);
    assert!(document.redo().unwrap());
    assert_eq!(document.document().snapshot(), created);
    assert_eq!(document.editor().selection(), EditorSelection::caret(8));
    assert!(range.set_link(None).unwrap());
    assert!(document.document().snapshot().link_fields.is_empty());
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), created);
}

#[test]
fn rich_field_live_rejected_and_unchanged_commands_preserve_preedit_and_redo() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("alpha beta");
    let linked = link(&document, 0..5, "x");
    let current = document.document().snapshot();
    document.get_range(10, 10).unwrap().set_text("!").unwrap();
    document.undo().unwrap();
    document.selection().set_range(10, 10).unwrap();
    document.editor().set_preedit("preview", None).unwrap();
    let before = document.document().snapshot();
    let revision = document.document().revision();
    let selection = document.editor().selection();
    for argument in [Some("unquoted"), Some("\"bad\0target\"")] {
        assert!(matches!(
            linked.set_link(argument),
            Err(TextFieldError::InvalidArgument)
        ));
    }
    assert!(!linked.set_link(Some("\"x\"")).unwrap());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.document().revision(), revision);
    assert_eq!(document.editor().selection(), selection);
    assert!(document.editor().is_composing());
    assert!(document.can_redo());
    assert!(linked.set_link(Some("\"updated\"")).unwrap());
    assert!(!document.editor().is_composing());
    assert_eq!(document.text(), current.text());
    assert_eq!(
        document.document().snapshot().link_fields[0].target,
        "updated"
    );
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), current);
}

#[test]
fn rich_field_live_cross_paragraph_results_preserve_identity_and_separator_only_slices() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("ab\n\ncd");
    link(&document, 1..5, "cross");
    let snapshot = document.document().snapshot();
    let field = &snapshot.link_fields[0];
    assert_eq!(field.range, 1..5);
    assert_eq!(links(&document.document()).len(), 2);
    for link in links(&document.document()) {
        assert_eq!(link.field_id(), Some(field.id));
    }
    let separators = document.document().slice(2..4).unwrap();
    assert_eq!(separators.text(), "\n\n");
    assert_eq!(separators.link_fields[0].range, 0..2);
    assert_eq!(separators.link_fields[0].id, field.id);
    separators.validate().unwrap();
    let restored = RichTextDocument::new();
    restored.restore(&separators).unwrap();
    assert_eq!(restored.snapshot(), separators);
}

#[test]
fn rich_field_live_plain_edits_extend_shrink_and_delete_fields_without_stale_projections() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd ef");
    link(&document, 0..4, "x");
    let id = document.document().snapshot().link_fields[0].id;
    document.get_range(2, 2).unwrap().set_text("😀").unwrap();
    let inserted = document.document().snapshot();
    assert_eq!(inserted.text(), "ab😀cd ef");
    assert_eq!(inserted.link_fields[0].range, 0..6);
    assert_eq!(inserted.link_fields[0].id, id);
    document.get_range(1, 5).unwrap().set_text("").unwrap();
    let shortened = document.document().snapshot();
    assert_eq!(shortened.text(), "ad ef");
    assert_eq!(shortened.link_fields[0].range, 0..2);
    assert_eq!(shortened.link_fields[0].id, id);
    document.get_range(0, 2).unwrap().set_text("").unwrap();
    let deleted = document.document().snapshot();
    assert!(deleted.link_fields.is_empty());
    deleted.validate().unwrap();
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), shortened);
}

#[test]
fn rich_field_live_case_expansion_preserves_one_field_and_contextual_run_mapping() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("aßb\nΟΣ");
    link(&document, 0..6, "case");
    let before = document.document().snapshot();
    let id = before.link_fields[0].id;
    assert!(document
        .get_range(1, 2)
        .unwrap()
        .change_case(LetterCase::Upper)
        .unwrap());
    let upper = document.document().snapshot();
    assert_eq!(upper.text(), "aSSb\nΟΣ");
    assert_eq!(upper.link_fields.len(), 1);
    assert_eq!(upper.link_fields[0].id, id);
    assert_eq!(upper.link_fields[0].range, 0..7);
    assert!(document
        .get_range(5, 7)
        .unwrap()
        .change_case(LetterCase::Lower)
        .unwrap());
    assert_eq!(document.text(), "aSSb\nος");
    assert_eq!(document.document().snapshot().link_fields[0].id, id);
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), upper);
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), before);
}

#[test]
fn rich_field_live_character_paragraph_replacement_preserves_cross_paragraph_identity() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    link(&document, 0..4, "x");
    let id = document.document().snapshot().link_fields[0].id;
    assert!(document
        .get_range(1, 1)
        .unwrap()
        .set_character_scalar('\n')
        .unwrap());
    let snapshot = document.document().snapshot();
    assert_eq!(snapshot.text(), "a\ncd");
    assert_eq!(snapshot.link_fields.len(), 1);
    assert_eq!(snapshot.link_fields[0].id, id);
    assert_eq!(snapshot.link_fields[0].range, 0..4);
    snapshot.validate().unwrap();
}

#[test]
fn rich_field_live_rich_insertion_remaps_copied_ids_and_splits_destination_field() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    link(&document, 0..4, "original");
    let before = document.document().snapshot();
    let copied = document.document().slice(0..2).unwrap();
    document
        .get_range(2, 2)
        .unwrap()
        .set_snapshot(&copied)
        .unwrap();
    let after = document.document().snapshot();
    assert_eq!(after.text(), "ababcd");
    assert_eq!(after.link_fields.len(), 3);
    assert_eq!(
        after
            .link_fields
            .iter()
            .map(|field| field.range.clone())
            .collect::<Vec<_>>(),
        vec![0..2, 2..4, 4..6]
    );
    let ids: HashSet<_> = after.link_fields.iter().map(|field| field.id).collect();
    assert_eq!(ids.len(), 3);
    assert_eq!(after.link_fields[0].id, before.link_fields[0].id);
    after.validate().unwrap();
    document.undo().unwrap();
    assert_eq!(document.document().snapshot(), before);
    document.redo().unwrap();
    assert_eq!(document.document().snapshot(), after);
}

#[test]
fn rich_field_live_undo_keeps_allocation_floor_and_never_reuses_retired_identity() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    link(&document, 0..2, "first");
    let first = document.document().snapshot().link_fields[0].id;
    document.undo().unwrap();
    assert_eq!(document.document().snapshot().next_link_field_id, 1);
    link(&document, 2..4, "second");
    assert!(document.document().snapshot().link_fields[0].id > first);
}

#[test]
fn rich_field_live_provisional_instruction_cannot_alias_a_later_authoritative_id() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichEditTextDocument::new(RichTextDocument::new());
    let paragraph = Paragraph::new();
    paragraph
        .inlines()
        .add(Hyperlink::with_text("old", Some("legacy")));
    document.document().blocks().add(paragraph.clone());
    let interior = document
        .field_story()
        .get_range(offset(1), offset(4))
        .unwrap();
    assert_eq!(interior.text().unwrap(), "YPE");
    paragraph.inlines().clear();
    paragraph.inlines().add(Run::with_text(Some("new")));
    link(&document, 0..3, "unrelated");
    assert_eq!(document.document().snapshot().link_fields[0].id, 1);
    assert!(matches!(interior.text(), Err(TextFieldError::StaleRange)));
}

#[test]
fn rich_field_live_target_updates_reuse_interactive_nodes_and_undo_restores_handlers() {
    let _scope = Dispatcher::unit_test_scope();
    let model = RichTextDocument::new();
    let paragraph = Paragraph::new();
    let original = Hyperlink::with_text("click", Some("old"));
    paragraph.inlines().add(original.clone());
    model.blocks().add(paragraph);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let subscription = original.click(move |args| {
        args.set_handled(true);
        observed.set(observed.get() + 1);
    });
    let document = RichEditTextDocument::new(model);
    let range = document.field_story().get_range_from_visible(0..5).unwrap();
    range.set_link(Some("\"new\"")).unwrap();
    let updated = links(&document.document()).remove(0);
    assert!(Ref::ptr_eq(&updated, &original));
    assert_eq!(updated.navigate_uri().as_deref(), Some("new"));
    assert!(updated.navigate());
    assert_eq!(calls.get(), 1);
    range.set_link(None).unwrap();
    assert!(links(&document.document()).is_empty());
    document.undo().unwrap();
    let restored = links(&document.document()).remove(0);
    assert!(restored.navigate());
    assert_eq!(calls.get(), 2);
    subscription.dispose();
    assert!(restored.navigate());
    assert_eq!(calls.get(), 2);
}

#[test]
fn rich_field_live_getter_moves_only_acted_handle_and_duplicate_remains_independent() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("alpha beta");
    link(&document, 0..5, "x");
    document.selection().set_range(9, 7).unwrap();
    document.clear_undo_redo_history();
    let start = document
        .field_story()
        .project_visible(1, FieldInstructionBoundary::After)
        .unwrap();
    let end = document
        .field_story()
        .project_visible(3, FieldInstructionBoundary::After)
        .unwrap();
    let range = document.field_story().get_range(start, end).unwrap();
    let duplicate = range.duplicate();
    let revision = document.document().revision();
    assert_eq!(range.get_link().unwrap().as_deref(), Some("\"x\""));
    assert_eq!(positions(&range), 0..18);
    assert_eq!(positions(&duplicate), start.get()..end.get());
    assert_eq!(document.editor().selection(), EditorSelection::new(9, 7));
    assert_eq!(document.document().revision(), revision);
    assert!(!document.can_undo());
}

#[test]
fn rich_field_live_publication_installs_acted_endpoints_before_newer_callback_range() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("alpha beta");
    let range = document.field_story().get_range_from_visible(0..5).unwrap();
    let updated = range.clone();
    let called = Rc::new(Cell::new(false));
    let observed = called.clone();
    let _subscription = document.editor().changed(move || {
        if !observed.replace(true) {
            assert_eq!(positions(&updated), 0..18);
            updated.set_range(offset(0), offset(0)).unwrap();
        }
    });
    assert!(range.set_link(Some("\"x\"")).unwrap());
    assert!(called.get());
    assert_eq!(positions(&range), 0..0);
    assert_eq!(document.document().snapshot().link_fields[0].range, 0..5);
}

#[test]
fn rich_field_live_external_publication_write_survives_interrupted_link_and_clears_history() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("alpha beta");
    let range = document.field_story().get_range_from_visible(0..5).unwrap();
    let updated = range.clone();
    let model = document.document();
    let called = Rc::new(Cell::new(false));
    let observed = called.clone();
    let _subscription = document.editor().changed(move || {
        if !observed.replace(true) {
            model
                .replace_text(0..model.text_length(), "external")
                .unwrap();
            updated.set_range(offset(0), offset(0)).unwrap();
        }
    });
    assert!(matches!(
        range.set_link(Some("\"x\"")),
        Err(TextFieldError::Edit(EditError::ReentrantEdit))
    ));
    assert_eq!(document.text(), "external");
    assert_eq!(positions(&range), 0..0);
    document.document().snapshot().validate().unwrap();
    assert!(!document.can_undo());
}

struct CallbackBrush {
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}
impl IBrush for CallbackBrush {
    fn opacity(&self) -> f64 {
        1.0
    }
    fn transform(&self) -> Option<Rc<dyn ITransform>> {
        None
    }
    fn transform_origin(&self) -> RelativePoint {
        RelativePoint::CENTER
    }
    fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_solid_color_brush(&self) -> Option<&dyn ISolidColorBrush> {
        let callback = self.callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        None
    }
}

#[test]
fn rich_field_live_snapshot_callback_rejects_stale_document_or_range_without_borrow_panics() {
    let _scope = Dispatcher::unit_test_scope();
    for mutate_model in [false, true] {
        let document = services("alpha beta");
        let range = document.field_story().get_range_from_visible(0..5).unwrap();
        let brush = Rc::new(CallbackBrush {
            callback: RefCell::new(None),
        });
        document
            .document()
            .element_at(1, TextGravity::Forward)
            .unwrap()
            .set_foreground(Some(brush.clone()));
        let updated = range.clone();
        let model = document.document();
        *brush.callback.borrow_mut() = Some(Box::new(move || {
            if mutate_model {
                model
                    .replace_text(0..model.text_length(), "external")
                    .unwrap();
            } else {
                updated.set_range(offset(7), offset(7)).unwrap();
            }
        }));
        assert!(matches!(
            range.set_link(Some("\"x\"")),
            Err(TextFieldError::Edit(EditError::ReentrantEdit))
        ));
        assert!(document.document().snapshot().link_fields.is_empty());
        if mutate_model {
            assert_eq!(document.text(), "external");
        } else {
            assert_eq!(positions(&range), 7..7);
        }
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_field_live_external_semantic_target_updates_are_reconciled_without_merging_other_ids() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("ab cd");
    link(&document, 0..2, "first");
    link(&document, 3..5, "second");
    let before = document.document().snapshot();
    let projected = links(&document.document());
    projected[0].set_navigate_uri(Some("second".to_owned()));
    let after = document.document().snapshot();
    assert_eq!(after.link_fields.len(), 2);
    assert_eq!(after.link_fields[0].id, before.link_fields[0].id);
    assert_eq!(after.link_fields[1], before.link_fields[1]);
    assert_eq!(after.link_fields[0].target, "second");
    after.validate().unwrap();
}

#[test]
fn rich_field_live_common_validation_rejects_dangling_projection_before_touching_model() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("ab");
    link(&document, 0..2, "x");
    let before = document.document().snapshot();
    let mut invalid = before.clone();
    invalid.link_fields.clear();
    assert!(matches!(
        document.document().restore(&invalid),
        Err(DocumentError::InvalidLinkField(_))
    ));
    assert_eq!(document.document().snapshot(), before);
    let mut wrong_target = before.clone();
    wrong_target.link_fields[0].target = "different".to_owned();
    assert!(wrong_target.validate().is_err());
}

#[test]
fn rich_field_live_offset_map_retires_empty_projection_and_preserves_watermark() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("ab");
    link(&document, 0..2, "x");
    let mut snapshot = document.document().snapshot();
    let watermark = snapshot.next_link_field_id;
    let InlineSnapshotKind::Hyperlink { inlines, .. } = &mut snapshot.paragraphs[0].inlines[0].kind
    else {
        panic!("Projected link expected")
    };
    inlines.clear();
    snapshot
        .remap_link_field_offsets(&[Some(0), Some(0), Some(0)])
        .unwrap();
    assert!(snapshot.link_fields.is_empty());
    assert!(matches!(
        snapshot.paragraphs[0].inlines[0].kind,
        InlineSnapshotKind::Span(_)
    ));
    assert_eq!(snapshot.next_link_field_id, watermark);
}

#[test]
fn rich_field_live_unsupported_target_name_projection_keeps_shared_metadata_intact() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("ab");
    link(&document, 0..2, "x");
    links(&document.document())[0].set_target_name(Some("frame".to_owned()));
    let before = document.document().snapshot();
    assert_eq!(before.link_fields[0].target_name.as_deref(), Some("frame"));
    assert!(matches!(
        document.field_story().text(),
        Err(TextFieldError::Unsupported(_))
    ));
    assert_eq!(document.document().snapshot(), before);
}

#[test]
fn rich_field_live_structured_line_break_is_preserved_or_rejected_before_a_partial_wrap() {
    let _scope = Dispatcher::unit_test_scope();
    let mut snapshot = DocumentSnapshot::from_text("a");
    snapshot.paragraphs[0]
        .inlines
        .push(crate::documents::InlineSnapshot {
            format: CharacterFormat::default(),
            kind: InlineSnapshotKind::LineBreak,
        });
    snapshot.paragraphs[0]
        .inlines
        .push(crate::documents::InlineSnapshot::run("b"));
    let width = snapshot.text_length() - 2;
    let document = services("");
    document.document().restore(&snapshot).unwrap();
    if width == 2 {
        let partial = document.field_story().get_range_from_visible(0..2).unwrap();
        assert!(matches!(
            partial.set_link(Some("\"x\"")),
            Err(TextFieldError::Unsupported(_))
        ));
        assert_eq!(document.document().snapshot(), snapshot);
        assert!(!document.can_undo());
    }
    link(&document, 0..width + 2, "x");
    let linked = document.document().snapshot();
    let InlineSnapshotKind::Hyperlink { inlines, .. } = &linked.paragraphs[0].inlines[0].kind
    else {
        panic!("Expected a field projection")
    };
    assert!(inlines
        .iter()
        .any(|inline| matches!(inline.kind, InlineSnapshotKind::LineBreak)));
    assert_eq!(linked.text(), snapshot.text());
}

#[test]
fn rich_field_live_structured_transforms_reject_callback_owned_range_or_document() {
    let _scope = Dispatcher::unit_test_scope();
    for character in [false, true] {
        for mutate_model in [false, true] {
            let document = services("ab cd");
            link(&document, 0..2, "x");
            let range = document.get_range(0, 2).unwrap();
            let brush = Rc::new(CallbackBrush {
                callback: RefCell::new(None),
            });
            document
                .document()
                .element_at(1, TextGravity::Forward)
                .unwrap()
                .set_foreground(Some(brush.clone()));
            document.clear_undo_redo_history();
            let updated = range.clone();
            let model = document.document();
            *brush.callback.borrow_mut() = Some(Box::new(move || {
                if mutate_model {
                    model
                        .replace_text(0..model.text_length(), "external")
                        .unwrap();
                } else {
                    updated.set_range(4, 4).unwrap();
                }
            }));
            let result = if character {
                range.set_character_scalar('\n')
            } else {
                range.change_case(LetterCase::Upper)
            };
            assert!(matches!(result, Err(EditError::ReentrantEdit)));
            if mutate_model {
                assert_eq!(document.text(), "external");
            } else {
                assert_eq!(range.range(), 4..4);
                assert_eq!(document.text(), "ab cd");
            }
            document.document().snapshot().validate().unwrap();
            assert!(!document.can_undo());
        }
    }
}

#[test]
fn rich_field_live_external_restore_starts_one_current_history_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("seed");
    document.set_text("abc").unwrap();
    assert!(document.can_undo());
    document
        .document()
        .restore(&DocumentSnapshot::from_text("abcd"))
        .unwrap();
    document.selection().set_range(3, 1).unwrap();
    let before = document.document().snapshot();
    let selection = document.selection().selection();
    assert!(!document.can_undo());

    link(&document, 0..2, "current");
    let after = document.document().snapshot();
    assert_eq!(after.text(), before.text());
    assert_eq!(after.link_fields.len(), 1);
    assert_eq!(document.selection().selection(), selection);
    assert!(document.can_undo());
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.selection().selection(), selection);
    assert!(!document.can_undo());
    assert!(document.redo().unwrap());
    assert_eq!(document.document().snapshot(), after);
}
