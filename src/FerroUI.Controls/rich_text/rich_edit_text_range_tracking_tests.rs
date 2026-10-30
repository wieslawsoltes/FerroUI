//! Endpoint evidence is replayed literally. Additional tests cover FerroUI
//! transactions, UTF-16, callbacks and private mutation-origin isolation; those
//! framework policies are not additional native observations.
use crate::documents::{
    DocumentError, DocumentSnapshot, DocumentTextChange, Inline, Paragraph, RichTextDocument, Run,
    TextGravity,
};
use crate::rich_text::{
    EditError, EditKind, EditorSelection, RangeGravity, RichEditTextDocument, RichEditTextRange,
    TextGetOptions,
};
use crate::test_support::test_scope;
use ferroui_base::Ref;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[path = "rich_edit_text_range_tracking_native_cases.rs"]
mod native_cases;

type NativeState = (usize, usize, i32);
struct NativeCase {
    name: &'static str,
    requested_range: (usize, usize),
    requested_gravity: i32,
    primary_before: NativeState,
    duplicate_before: NativeState,
    edit: (usize, usize),
    text: &'static str,
    text_length: usize,
    hresult: u32,
    primary_after: NativeState,
    duplicate_after: NativeState,
    action_after: (usize, usize),
    story: &'static str,
}

fn services(text: &str) -> RichEditTextDocument {
    RichEditTextDocument::new(RichTextDocument::from_text(text))
}

fn native_state(range: &RichEditTextRange) -> NativeState {
    (
        range.start_position(),
        range.end_position(),
        range.gravity() as i32,
    )
}

fn replay(cases: &[NativeCase]) {
    for case in cases {
        // Each measured fixture starts in a fresh control with this same story.
        let document = services("abcdef");
        let primary = document
            .get_range(case.requested_range.0, case.requested_range.1)
            .unwrap();
        primary.set_gravity_value(case.requested_gravity).unwrap();
        let duplicate = primary.get_clone().unwrap();
        assert_eq!(
            native_state(&primary),
            case.primary_before,
            "{} before",
            case.name
        );
        assert_eq!(
            native_state(&duplicate),
            case.duplicate_before,
            "{} duplicate before",
            case.name
        );
        assert_eq!(case.text.encode_utf16().count(), case.text_length);
        let action = document.get_range(case.edit.0, case.edit.1).unwrap();
        assert_eq!(case.hresult, 0, "{} native HRESULT", case.name);
        action
            .set_text(case.text)
            .unwrap_or_else(|error| panic!("{}: {error}", case.name));
        assert_eq!(
            native_state(&primary),
            case.primary_after,
            "{} primary",
            case.name
        );
        assert_eq!(
            native_state(&duplicate),
            case.duplicate_after,
            "{} duplicate",
            case.name
        );
        assert_eq!(
            (action.start_position(), action.end_position()),
            case.action_after,
            "{} acted range",
            case.name
        );
        assert_eq!(
            document.get_text(TextGetOptions::NONE).unwrap(),
            case.story,
            "{} story",
            case.name
        );
    }
}

#[test]
fn rich_range_tracking_replays_115_original_native_edits_and_230_passive_pairs() {
    let _scope = test_scope();
    assert_eq!(native_cases::ORIGINAL.len(), 115);
    replay(native_cases::ORIGINAL);
}

#[test]
fn rich_range_tracking_replays_15_additional_native_edits_and_30_passive_pairs() {
    let _scope = test_scope();
    assert_eq!(native_cases::ADDITIONAL.len(), 15);
    replay(native_cases::ADDITIONAL);
}

#[test]
fn rich_range_tracking_operation_span_is_private_and_precedes_public_notifications() {
    let _scope = test_scope();
    let document = services("abcdef");
    let observed = Rc::new(RefCell::new(Vec::new()));
    let passive_slot = Rc::new(RefCell::new(None::<RichEditTextRange>));
    let captured = observed.clone();
    let slot = passive_slot.clone();
    // Register the public observer before the range itself. Subscription order
    // must not expose the old coordinates to a public document observer.
    let subscription = document.document().changed(move |args| {
        captured.borrow_mut().push((
            slot.borrow().as_ref().unwrap().range(),
            args.changes.clone(),
        ));
    });
    let passive = document.get_range(1, 4).unwrap();
    *passive_slot.borrow_mut() = Some(passive.clone());
    let pointer = document
        .document()
        .create_pointer(1, TextGravity::Backward)
        .unwrap();
    let end = document
        .document()
        .create_pointer(4, TextGravity::Forward)
        .unwrap();
    document.get_range(0, 2).unwrap().set_text("abX").unwrap();
    assert_eq!(passive.range(), 3..5);
    assert_eq!(pointer.offset(), 1);
    assert_eq!(end.offset(), 5);
    assert_eq!(
        *observed.borrow(),
        vec![(
            3..5,
            vec![DocumentTextChange {
                range: 2..2,
                inserted_length: 1
            }]
        )]
    );
    subscription.dispose();
}

#[test]
fn rich_range_tracking_same_visible_diff_keeps_distinct_origins_and_operation_spans() {
    let _scope = test_scope();
    for origin in 0..6 {
        let document = services("abcdef");
        let passive = document.get_range(1, 4).unwrap();
        match origin {
            0 => document.get_range(0, 2).unwrap().set_text("abX").unwrap(),
            1 => document.get_range(2, 2).unwrap().set_text("X").unwrap(),
            2 => document.document().replace_text(0..2, "abX").unwrap(),
            3 | 4 => {
                let editor = document.editor();
                editor.set_selection(EditorSelection::new(0, 2)).unwrap();
                editor
                    .insert_text(
                        "abX",
                        if origin == 3 {
                            EditKind::Programmatic
                        } else {
                            EditKind::Typing
                        },
                    )
                    .unwrap();
            }
            5 => document
                .get_range(0, 2)
                .unwrap()
                .set_snapshot(&DocumentSnapshot::from_text("abX"))
                .unwrap(),
            _ => unreachable!(),
        }
        assert_eq!(document.text(), "abXcdef");
        assert_eq!(
            passive.range(),
            if origin == 0 { 3..5 } else { 1..5 },
            "origin {origin}"
        );
    }
}

#[test]
fn rich_range_tracking_duplicate_identity_and_formatting_gravity_remain_independent() {
    let _scope = test_scope();
    let document = services("abcdef");
    let primary = document.get_range(1, 4).unwrap();
    primary.set_gravity(RangeGravity::Inward);
    let shared = primary.clone();
    let duplicate = primary.get_clone().unwrap();
    assert_eq!(duplicate.gravity(), RangeGravity::UIBehavior);
    duplicate.set_range(2, 2).unwrap();
    document.get_range(0, 4).unwrap().set_text("XY").unwrap();
    assert_eq!(primary.range(), 0..0);
    assert_eq!(shared.range(), 0..0);
    assert_eq!(duplicate.range(), 0..0);
    assert_eq!(primary.gravity(), RangeGravity::Inward);
    duplicate.set_range(1, 2).unwrap();
    assert_eq!(shared.range(), 0..0);
    shared.set_range(3, 3).unwrap();
    assert_eq!(primary.range(), 3..3);
    assert_eq!(duplicate.range(), 1..2);
}

#[test]
fn rich_range_tracking_same_text_has_no_semantic_event_and_cannot_leak_origin() {
    let _scope = test_scope();
    let document = services("abcdef");
    let exact = document.get_range(1, 4).unwrap();
    let overlap = document.get_range(2, 2).unwrap();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let captured = changes.clone();
    let subscription = document
        .document()
        .changed(move |args| captured.borrow_mut().extend(args.changes.clone()));
    document.get_range(1, 4).unwrap().set_text("bcd").unwrap();
    assert_eq!(exact.range(), 1..4);
    // Only the exact pair is observed natively. Keeping an unmeasured overlapping
    // caret unchanged for a text no-op preserves the existing framework policy.
    assert_eq!(overlap.range(), 2..2);
    assert!(changes.borrow().is_empty());
    document.document().replace_text(0..4, "XY").unwrap();
    assert_eq!(exact.range(), 0..2);
    assert_eq!(overlap.range(), 2..2);
    subscription.dispose();
}

#[test]
fn rich_range_tracking_uses_accepted_scalar_safe_text_and_structural_normalization() {
    let _scope = test_scope();
    let document = services("a😀bcdef");
    document.editor().set_max_length(8);
    let passive = document.get_range(1, 5).unwrap();
    let actor = document.get_range(0, 3).unwrap();
    actor.set_text("Q😀Z").unwrap();
    assert_eq!(document.text(), "Q😀bcdef");
    assert_eq!(actor.range(), 0..3);
    assert_eq!(passive.range(), 3..5);
    assert_eq!(passive.text().unwrap(), "bc");
    document.document().validate_range(passive.range()).unwrap();

    let document = services("abcdef");
    document.editor().set_max_length(7);
    let passive = document.get_range(1, 4).unwrap();
    document
        .get_range(0, 2)
        .unwrap()
        .set_text("ab\r\nX")
        .unwrap();
    assert_eq!(document.text(), "ab\ncdef");
    assert_eq!(passive.range(), 3..5);
    assert_eq!(passive.text().unwrap(), "cd");

    // A rejected insertion at a full document creates no operation provenance.
    document.editor().set_max_length(7);
    document.get_range(4, 4).unwrap().set_text("😀").unwrap();
    assert_eq!(document.text(), "ab\ncdef");
    assert_eq!(passive.range(), 3..5);
    document.document().replace_text(3..3, "Q").unwrap();
    assert_eq!(passive.range(), 3..6);
}

#[test]
fn rich_range_tracking_updates_each_edit_inside_one_notification_batch() {
    let _scope = test_scope();
    let document = services("abcdef");
    let passive = document.get_range(1, 4).unwrap();
    let events = Rc::new(Cell::new(0));
    let captured = events.clone();
    let subscription = document
        .document()
        .changed(move |_| captured.set(captured.get() + 1));
    let batch = document.document().begin_edit();
    document.get_range(0, 2).unwrap().set_text("abX").unwrap();
    assert_eq!(passive.range(), 3..5);
    let created_later = document.get_range(3, 3).unwrap();
    document.get_range(3, 3).unwrap().set_text("Q").unwrap();
    assert_eq!(passive.range(), 3..6);
    assert_eq!(created_later.range(), 3..3);
    assert_eq!(events.get(), 0);
    drop(batch);
    assert_eq!(events.get(), 1);
    assert_eq!(created_later.range(), 3..3);
    subscription.dispose();
}

fn cancel() -> EditError {
    DocumentError::InvalidFormat("cancel range tracking transaction").into()
}

#[test]
fn rich_range_tracking_failed_group_restores_collapsed_pairs_before_publication() {
    let _scope = test_scope();
    let document = services("abcdef");
    let passive = document.get_range(1, 4).unwrap();
    let duplicate = passive.get_clone().unwrap();
    let before = document.document().snapshot();
    let observed = Rc::new(RefCell::new(Vec::new()));
    let captured = observed.clone();
    let read_range = passive.clone();
    let subscription = document
        .document()
        .changed(move |_| captured.borrow_mut().push(read_range.range()));
    let result: Result<(), EditError> = document.edit_group(|document| {
        document.get_range(0, 6)?.set_text("Z")?;
        assert_eq!(passive.range(), 0..0);
        let nested: Result<(), EditError> = document.edit_group(|document| {
            document.get_range(0, 1)?.set_text("XY")?;
            assert_eq!(passive.range(), 0..0);
            Err(cancel())
        });
        assert!(nested.is_err());
        assert_eq!(document.text(), "Z");
        assert_eq!(passive.range(), 0..0);
        Err(cancel())
    });
    assert!(result.is_err());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(passive.range(), 1..4);
    assert_eq!(duplicate.range(), 1..4);
    assert!(observed.borrow().iter().all(|range| *range == (1..4)));
    assert!(!document.can_undo());
    subscription.dispose();
}

#[test]
fn rich_range_tracking_unchanged_semantic_pair_still_participates_in_rollback() {
    let _scope = test_scope();
    let document = services("abcdef");
    let passive = document.get_range(0, 3).unwrap();
    let result: Result<(), EditError> = document.edit_group(|document| {
        document.get_range(4, 4)?.set_text("X")?;
        assert_eq!(passive.range(), 0..3);
        let editor = document.editor();
        editor.set_selection(EditorSelection::new(0, 7))?;
        editor.insert_text("Z", EditKind::Programmatic)?;
        assert_eq!(passive.range(), 0..1);
        Err(cancel())
    });
    assert!(result.is_err());
    assert_eq!(document.text(), "abcdef");
    assert_eq!(passive.range(), 0..3);
    assert!(!document.can_undo());
}

#[test]
fn rich_range_tracking_rollback_preserves_new_endpoint_identity_and_new_ranges() {
    let _scope = test_scope();
    let document = services("abcdef");
    let passive = document.get_range(1, 4).unwrap();
    let created = RefCell::new(None);
    let result: Result<(), EditError> = document.edit_group(|document| {
        document.get_range(0, 6)?.set_text("Z")?;
        passive.set_range(1, 1)?;
        passive.set_gravity(RangeGravity::Forward);
        created.replace(Some(document.get_range(0, 1)?));
        Err(cancel())
    });
    assert!(result.is_err());
    assert_eq!(document.text(), "abcdef");
    // New identities follow the existing snapshot-restore pointer policy. The
    // abandoned pre-group identity must not overwrite an explicit newer setter.
    assert_eq!(passive.range(), 6..6);
    assert_eq!(passive.gravity(), RangeGravity::Forward);
    assert_eq!(created.borrow().as_ref().unwrap().range(), 0..6);
    assert!(!document.can_undo());
}

#[test]
fn rich_range_tracking_successful_undo_uses_existing_restore_policy_and_one_history_unit() {
    let _scope = test_scope();
    let document = services("abcdef");
    let passive = document.get_range(1, 4).unwrap();
    let before = document.document().snapshot();
    document.get_range(0, 6).unwrap().set_text("Z").unwrap();
    let after = document.document().snapshot();
    assert_eq!(passive.range(), 0..0);
    passive.set_gravity(RangeGravity::Backward);
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(passive.range(), 0..6);
    assert_eq!(passive.gravity(), RangeGravity::Backward);
    assert!(!document.can_undo());
    assert!(document.redo().unwrap());
    assert_eq!(document.document().snapshot(), after);
    assert_eq!(passive.range(), 0..1);
    assert!(!document.can_redo());
}

#[repr(C)]
struct ReadCallbackRun {
    base: Run,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}
ferroui_base::ferro_class!(ReadCallbackRun: Run);
ferroui_base::ferro_impl_classes!(ReadCallbackRun:
    ferroui_base::FerroObjectImpl,
    ferroui_base::StyledElementImpl,
    crate::documents::TextElementImpl
);
impl crate::documents::InlineImpl for ReadCallbackRun {
    fn append_text(this: &Self, output: &mut String) {
        use crate::documents::InlineImplExt;
        Self::parent_append_text(this, output);
        if this.text().as_deref() == Some("abcXY") {
            let callback = this.callback.borrow_mut().take();
            if let Some(callback) = callback {
                callback();
            }
        }
    }
}

#[test]
fn rich_range_tracking_text_read_callback_cannot_replay_into_a_new_range_identity() {
    let _scope = test_scope();
    for replace_existing in [false, true] {
        let run: Ref<ReadCallbackRun> = ferroui_base::instantiate(ReadCallbackRun {
            base: Run::construct(),
            callback: RefCell::new(None),
        });
        run.set_text(Some("abcdef"));
        let paragraph = Paragraph::new();
        paragraph.add_child(run.clone());
        let model = RichTextDocument::new();
        model.blocks().add(paragraph);
        let document = RichEditTextDocument::new(model);
        let passive = document.get_range(1, 4).unwrap();
        let created = Rc::new(RefCell::new(None));
        let captured = created.clone();
        let callback_document = document.clone();
        let callback_range = passive.clone();
        run.callback.replace(Some(Box::new(move || {
            if replace_existing {
                callback_range.set_range(4, 4).unwrap();
            } else {
                captured.replace(Some(callback_document.get_range(4, 4).unwrap()));
            }
        })));
        document.get_range(3, 6).unwrap().set_text("XY").unwrap();
        assert!(run.callback.borrow().is_none());
        if replace_existing {
            assert_eq!(passive.range(), 4..4);
        } else {
            assert_eq!(passive.range(), 1..3);
            assert_eq!(created.borrow().as_ref().unwrap().range(), 4..4);
        }
    }
}

#[test]
fn rich_range_tracking_callback_owned_model_and_endpoints_survive_failed_publication() {
    let _scope = test_scope();
    let document = services("abcdef");
    let passive = document.get_range(1, 4).unwrap();
    let callback_document = document.document();
    let callback_range = passive.clone();
    let armed = Rc::new(Cell::new(true));
    let captured = armed.clone();
    let subscription = document.document().changed(move |_| {
        if captured.replace(false) {
            assert_eq!(callback_range.range(), 0..0);
            callback_range.set_range(0, 1).unwrap();
            callback_document.replace_text(2..2, "Z").unwrap();
        }
    });
    let result = document.get_range(0, 4).unwrap().set_text("XY");
    assert!(matches!(result, Err(EditError::ReentrantEdit)));
    assert!(!armed.get());
    assert_eq!(document.text(), "XYZef");
    assert_eq!(passive.range(), 0..1);
    assert!(!document.can_undo());
    subscription.dispose();
    // The explicit SetText hint was consumed by its own splice, so this later
    // ordinary replacement must use the existing independent-pointer mapping.
    document.document().replace_text(0..2, "Q").unwrap();
    assert_eq!(passive.range(), 0..1);
}

#[test]
fn rich_range_tracking_private_subscriptions_do_not_retain_ranges_or_documents() {
    let _scope = test_scope();
    let weak = {
        let document = services("abcdef");
        let passive = document.get_range(1, 4).unwrap();
        let data = Rc::downgrade(&passive.data);
        let weak_document = document.document().downgrade();
        drop(passive);
        assert!(data.upgrade().is_none());
        document.get_range(0, 4).unwrap().set_text("XY").unwrap();
        weak_document
    };
    assert!(weak.upgrade().is_none());
}

#[test]
fn rich_range_tracking_empty_rich_snapshot_keeps_its_original_transaction_origin() {
    let _scope = test_scope();
    for rich in [false, true] {
        let document = services("abcdef");
        let passive = document.get_range(1, 4).unwrap();
        let before = document.document().snapshot();
        let result: Result<(), EditError> = document.edit_group(|document| {
            let actor = document.get_range(0, 6)?;
            if rich {
                actor.set_snapshot(&DocumentSnapshot::default())?;
            } else {
                actor.set_text("")?;
            }
            assert_eq!(document.text(), "");
            assert_eq!(passive.range(), 0..0);
            assert_eq!(actor.range(), 0..0);
            Err(cancel())
        });
        assert!(result.is_err());
        assert_eq!(document.document().snapshot(), before);
        // Deletions alone give the same endpoint values. A failed transaction
        // exposes whether the empty-rich fallback wrongly acquired the paired
        // checkpoint policy: rich restoration retains its prior pointer map.
        assert_eq!(passive.range(), if rich { 0..6 } else { 1..4 });
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_range_tracking_nested_text_read_write_cannot_replay_stale_outer_text() {
    let _scope = test_scope();
    let run: Ref<ReadCallbackRun> = ferroui_base::instantiate(ReadCallbackRun {
        base: Run::construct(),
        callback: RefCell::new(None),
    });
    run.set_text(Some("abcdef"));
    let paragraph = Paragraph::new();
    paragraph.add_child(run.clone());
    let model = RichTextDocument::new();
    model.blocks().add(paragraph);
    let document = RichEditTextDocument::new(model.clone());
    let passive = document.get_range(1, 4).unwrap();
    let pointer = model.create_pointer(4, TextGravity::Forward).unwrap();
    let callback_document = model.clone();
    run.callback.replace(Some(Box::new(move || {
        callback_document.replace_text(0..5, "Q").unwrap();
    })));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let captured = changes.clone();
    let subscription =
        model.changed(move |args| captured.borrow_mut().extend(args.changes.clone()));
    let result = document.get_range(3, 6).unwrap().set_text("XY");
    assert!(matches!(result, Err(EditError::ReentrantEdit)));
    assert!(run.callback.borrow().is_none());
    assert_eq!(document.text(), "Q");
    assert_eq!(passive.range(), 0..1);
    assert_eq!(pointer.offset(), 1);
    model.validate_range(passive.range()).unwrap();
    model
        .validate_range(pointer.offset()..pointer.offset())
        .unwrap();
    assert_eq!(
        *changes.borrow(),
        vec![DocumentTextChange {
            range: 0..6,
            inserted_length: 1
        }]
    );
    assert!(!document.can_undo());
    subscription.dispose();
    // The superseded outer observation must not remain in observed_text either.
    model.replace_text(1..1, "R").unwrap();
    assert_eq!(document.text(), "QR");
    assert_eq!(passive.range(), 0..2);
    assert_eq!(pointer.offset(), 2);
}

#[repr(C)]
struct PropertyCallbackRun {
    base: Run,
    before_parent: bool,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}
ferroui_base::ferro_class!(PropertyCallbackRun: Run);
ferroui_base::ferro_impl_classes!(PropertyCallbackRun:
    ferroui_base::StyledElementImpl,
    crate::documents::TextElementImpl,
    crate::documents::InlineImpl
);
impl ferroui_base::FerroObjectImpl for PropertyCallbackRun {
    fn on_property_changed(this: &Self, change: &ferroui_base::FerroPropertyChangedEventArgs<'_>) {
        use ferroui_base::FerroObjectImplExt;
        let callback =
            if change.property().name() == "Text" && this.text().as_deref() == Some("abcXY") {
                this.callback.borrow_mut().take()
            } else {
                None
            };
        if this.before_parent {
            if let Some(callback) = callback {
                callback();
            }
            Self::parent_on_property_changed(this, change);
        } else {
            Self::parent_on_property_changed(this, change);
            if let Some(callback) = callback {
                callback();
            }
        }
    }
}

#[test]
fn rich_range_tracking_property_callback_writes_before_or_after_invalidation_own_their_content() {
    let _scope = test_scope();
    for before_parent in [true, false] {
        let run: Ref<PropertyCallbackRun> = ferroui_base::instantiate(PropertyCallbackRun {
            base: Run::construct(),
            before_parent,
            callback: RefCell::new(None),
        });
        run.set_text(Some("abcdef"));
        let paragraph = Paragraph::new();
        paragraph.add_child(run.clone());
        let model = RichTextDocument::new();
        model.blocks().add(paragraph);
        let document = RichEditTextDocument::new(model.clone());
        let passive = document.get_range(1, 4).unwrap();
        let pointer = model.create_pointer(4, TextGravity::Forward).unwrap();
        let callback_document = model.clone();
        run.callback.replace(Some(Box::new(move || {
            callback_document.replace_text(0..5, "Q").unwrap();
        })));
        let result = document.get_range(3, 6).unwrap().set_text("XY");
        assert!(
            matches!(result, Err(EditError::ReentrantEdit)),
            "before parent {before_parent}: {result:?}"
        );
        assert!(run.callback.borrow().is_none());
        assert_eq!(document.text(), "Q");
        assert_eq!(passive.range(), 0..1);
        assert_eq!(pointer.offset(), 1);
        assert!(!document.can_undo());
        model.replace_text(1..1, "R").unwrap();
        assert_eq!(passive.range(), 0..2);
        assert_eq!(pointer.offset(), 2);
    }
}

#[repr(C)]
struct ReconcileCallbackInline {
    base: Inline,
    reads_until_callback: Cell<usize>,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}
ferroui_base::ferro_class!(ReconcileCallbackInline: Inline);
ferroui_base::ferro_impl_classes!(ReconcileCallbackInline:
    ferroui_base::FerroObjectImpl,
    ferroui_base::StyledElementImpl,
    crate::documents::TextElementImpl
);
impl crate::documents::InlineImpl for ReconcileCallbackInline {
    fn append_text(this: &Self, _output: &mut String) {
        let remaining = this.reads_until_callback.get();
        if remaining > 0 {
            this.reads_until_callback.set(remaining - 1);
            if remaining == 1 {
                let callback = this.callback.borrow_mut().take();
                if let Some(callback) = callback {
                    callback();
                }
            }
        }
    }
}

#[test]
fn rich_range_tracking_field_reconciliation_callbacks_observe_current_coordinates() {
    let _scope = test_scope();
    let document = services("abcdef");
    document
        .field_story()
        .get_range_from_visible(0..1)
        .unwrap()
        .set_link(Some("\"https://example.test/\""))
        .unwrap();
    let model = document.document();
    let run = model
        .element_at(2, TextGravity::Forward)
        .unwrap()
        .cast::<Run>()
        .unwrap();
    assert_eq!(run.text().as_deref(), Some("bcdef"));
    let inline: Ref<ReconcileCallbackInline> = ferroui_base::instantiate(ReconcileCallbackInline {
        base: Inline::construct(),
        reads_until_callback: Cell::new(0),
        callback: RefCell::new(None),
    });
    model.paragraphs()[0].add_child(inline.clone());
    let passive = document.get_range(3, 4).unwrap();
    let pointer = model.create_pointer(4, TextGravity::Forward).unwrap();
    let callback_range = passive.clone();
    let callback_pointer = pointer.clone();
    let callback_run = run.clone();
    inline.callback.replace(Some(Box::new(move || {
        assert_eq!(callback_range.range(), 3..5);
        assert_eq!(callback_pointer.offset(), 5);
        callback_run.set_text(Some("Q"));
    })));
    // One read accepts the new text; the next is semantic field reconciliation.
    // Its application callback must see that accepted replacement already mapped.
    inline.reads_until_callback.set(2);
    run.set_text(Some("bcXdef"));
    assert!(inline.callback.borrow().is_none());
    assert_eq!(document.text(), "aQ");
    assert_eq!(passive.range(), 1..2);
    assert_eq!(pointer.offset(), 2);
    model.snapshot().validate().unwrap();
}
