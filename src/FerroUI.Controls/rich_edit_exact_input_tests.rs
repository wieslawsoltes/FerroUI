//! CoreText Range/Text/NewSelection regression contracts through the actual
//! input manager, rich control, document and raw/routed acknowledgement path.
use super::*;
use ferroui_base::input::text_input::{TextInputMethodEdit, TextSelection};

fn edit(start: i32, end: i32, text: &str, anchor: i32, active: i32) -> TextInputMethodEdit {
    TextInputMethodEdit::new(
        TextSelection::new(start, end),
        text,
        TextSelection::new(anchor, active),
    )
}

#[test]
fn rich_exact_input_replaces_requested_range_independently_of_selection_and_undoes_once() {
    let f = Fixture::new(true);
    f.owner.set_text("a😀bcd").unwrap();
    f.owner.select(5, 6);
    f.owner.editor().clear_history();
    let before = f.owner.document().snapshot();
    assert!(f
        .connection()
        .apply_edit(&edit(1, 3, "XY", 3, 1), 1)
        .unwrap());
    assert_eq!(f.owner.text(), "aXYbcd");
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (3, 1));
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (5, 6));
    assert!(!f.owner.undo().unwrap());
    assert!(f.owner.redo().unwrap());
    assert_eq!(
        f.connection().edit_state().unwrap().selection,
        TextSelection::new(3, 1)
    );
}

#[test]
fn rich_exact_input_empty_replacement_deletes_real_range() {
    let f = Fixture::new(true);
    assert!(f.connection().apply_edit(&edit(0, 4, "", 0, 0), 0).unwrap());
    assert_eq!(f.owner.text(), "");
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
}

#[test]
fn rich_exact_input_invalid_utf16_and_policy_changes_leave_text_selection_and_history_intact() {
    let f = Fixture::new(true);
    f.owner.set_text("a😀b").unwrap();
    f.owner.select(1, 3);
    f.owner.editor().clear_history();
    let before = f.owner.document().snapshot();
    let version = f.owner.document().revision();
    for request in [
        edit(-1, 0, "x", 0, 0),
        edit(1, 2, "x", 1, 1),
        edit(0, 0, "", 2, 2),
        edit(0, 10, "", 0, 0),
    ] {
        assert_eq!(
            f.connection().apply_edit(&request, 0),
            Err(TextInputMethodError::InvalidRange)
        );
    }
    f.owner.set_accepts_return(false);
    assert_eq!(
        f.connection().apply_edit(&edit(1, 3, "x\ny", 3, 3), 0),
        Err(TextInputMethodError::Rejected)
    );
    f.owner.set_accepts_return(true);
    assert_eq!(
        f.connection().apply_edit(&edit(1, 3, "\r\n", 3, 3), 0),
        Err(TextInputMethodError::Rejected)
    );
    f.owner.set_max_length(4);
    assert_eq!(
        f.connection().apply_edit(&edit(1, 3, "long", 5, 5), 0),
        Err(TextInputMethodError::Rejected)
    );
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(f.owner.document().revision(), version);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (1, 3));
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_exact_composition_uses_current_utf16_deltas_and_retains_suffix_when_caret_moves_inside() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    let session = f.connection().begin_composition().unwrap();
    assert_eq!(f.owner.document().snapshot(), before);
    session.apply_edit(&edit(1, 3, "😀XYZ", 3, 3)).unwrap();
    assert_eq!(f.owner.text(), "a😀XYZd");
    assert_eq!(f.owner.editor().active_composition_range(), Some(1..6));
    assert_eq!(f.connection().edit_state().unwrap().text, "a😀XYZd");
    // Replaces an interior suffix range in visible UTF-16 coordinates. The
    // resulting caret is deliberately before the end of the composition.
    session.apply_edit(&edit(4, 5, "q", 3, 3)).unwrap();
    assert_eq!(f.owner.text(), "a😀XqZd");
    assert!(session.is_current());
    assert!(session.complete(2).unwrap());
    assert_eq!(f.owner.text(), "a😀XqZd");
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (3, 3));
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.undo().unwrap());
    assert!(f.owner.redo().unwrap());
    assert_eq!(f.owner.text(), "a😀XqZd");
}

#[test]
fn rich_exact_composition_extends_across_original_selection_and_preserves_cancel_snapshot() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 2, "X", 2, 2)).unwrap();
    session.apply_edit(&edit(0, 1, "YZ", 3, 3)).unwrap();
    session.apply_edit(&edit(4, 5, "Q", 4, 4)).unwrap();
    assert_eq!(f.owner.text(), "YZXcQ");
    assert_eq!(f.owner.editor().active_composition_range(), Some(0..5));
    assert!(!f.owner.can_undo());
    assert!(session.cancel().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (1, 3));
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_exact_composition_empty_deletion_commits_without_resurrecting_original_selection() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "", 1, 1)).unwrap();
    assert_eq!(f.owner.text(), "ad");
    assert!(session.complete(0).unwrap());
    assert_eq!(f.owner.text(), "ad");
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
    assert!(f.owner.redo().unwrap());
    assert_eq!(f.owner.text(), "ad");
}

#[test]
fn rich_exact_composition_selection_update_keeps_full_preview_and_does_not_cancel() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "hello", 6, 6)).unwrap();
    session.select_exact(TextSelection::new(5, 2)).unwrap();
    assert!(session.is_current());
    assert_eq!(f.owner.text(), "ahellod");
    assert!(f.owner.editor().is_composing());
    assert!(session.complete(0).unwrap());
    assert_eq!(f.owner.text(), "ahellod");
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (5, 2));
}

#[test]
fn rich_exact_composition_external_visible_selection_rejects_late_native_requests() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "XYZ", 4, 4)).unwrap();
    let committed = f.owner.text_input_method_client().selection();
    f.owner.select(0, 0);
    let before = f.owner.document().snapshot();
    assert_eq!(f.owner.text_input_method_client().selection(), committed);
    assert!(!session.is_current());
    assert_eq!(
        session.apply_edit(&edit(1, 2, "Q", 2, 2)),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(
        session.select_exact(TextSelection::new(4, 4)),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(session.complete(0), Err(TextInputMethodError::StateChanged));
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (0, 0));
}

#[test]
fn rich_exact_composition_canceled_visible_selection_keeps_the_native_session_current() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "XYZ", 4, 4)).unwrap();
    f.owner.selection_changing(|_, args| args.set_cancel(true));
    f.owner.select(0, 0);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (4, 4));
    assert!(session.is_current());
    assert!(session.complete(0).unwrap());
    assert_eq!(f.owner.text(), "aXYZd");
}

#[test]
fn rich_exact_composition_directed_selection_changes_only_publish_after_commit() {
    let f = Fixture::new(true);
    let input = f.owner.text_input_method_client();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let recorded = seen.clone();
    let weak_input = Rc::downgrade(&input);
    let subscription = input.selection_changed(Rc::new(move || {
        recorded
            .borrow_mut()
            .push(weak_input.upgrade().unwrap().selection());
    }));
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "😀XYZ", 3, 3)).unwrap();
    session.select_exact(TextSelection::new(6, 1)).unwrap();
    session.select_exact(TextSelection::new(5, 3)).unwrap();
    assert_eq!(input.selection(), TextSelection::new(1, 3));
    assert_eq!(
        f.connection().edit_state().unwrap().selection,
        TextSelection::new(5, 3)
    );
    assert!(session.is_current());
    assert!(seen.borrow().is_empty());
    assert!(session.complete(0).unwrap());
    assert_eq!(f.owner.text(), "a😀XYZd");
    assert_eq!(&*seen.borrow(), &[TextSelection::new(5, 3)]);
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
    assert_eq!(
        &*seen.borrow(),
        &[TextSelection::new(5, 3), TextSelection::new(1, 3)]
    );
    subscription.dispose();
}

#[test]
fn rich_exact_composition_external_selection_round_trip_invalidates_native_requests() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "😀XYZ", 6, 3)).unwrap();
    let input = f.owner.text_input_method_client();
    let committed = input.selection();
    let snapshot = f.owner.document().snapshot();
    let count = Rc::new(Cell::new(0));
    let recorded = count.clone();
    let subscription = input.selection_changed(Rc::new(move || {
        recorded.set(recorded.get() + 1);
    }));

    f.owner.select(3, 6);
    f.owner.select(6, 3);

    assert_eq!(input.selection(), committed);
    assert_eq!(count.get(), 0);
    assert!(!session.is_current());
    assert_eq!(
        session.select_exact(TextSelection::new(3, 3)),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(session.complete(0), Err(TextInputMethodError::StateChanged));
    assert_eq!(f.owner.document().snapshot(), snapshot);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (6, 3));
    subscription.dispose();
}

#[test]
fn rich_exact_composition_rejects_external_mutation_before_document_batch_notification() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "XYZ", 4, 4)).unwrap();
    let document = f.owner.document();
    let revision = document.revision();
    let batch = document.begin_edit();
    document.replace_text(0..0, "external").unwrap();
    let before = document.snapshot();
    assert_eq!(
        document.revision(),
        revision,
        "public notification remains deferred"
    );
    assert_eq!(
        session.apply_edit(&edit(0, 0, "stale", 0, 0)),
        Err(TextInputMethodError::StateChanged)
    );
    assert_eq!(document.snapshot(), before);
    assert!(!session.is_current());
    drop(batch);
    assert_eq!(document.snapshot(), before);
}

#[test]
fn rich_exact_composition_rejects_unpublished_formatting_even_when_text_and_selection_match() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "XYZ", 4, 4)).unwrap();
    let document = f.owner.document();
    let revision = document.revision();
    let text = document.text();
    let selection = f.owner.editor().selection();
    let batch = document.begin_edit();
    document
        .apply_format(
            0..1,
            &CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..CharacterFormat::default()
            },
        )
        .unwrap();
    let before = document.snapshot();
    assert_eq!(document.revision(), revision);
    assert_eq!(document.text(), text);
    assert_eq!(f.owner.editor().selection(), selection);
    assert_eq!(session.complete(0), Err(TextInputMethodError::StateChanged));
    assert_eq!(document.snapshot(), before);
    drop(batch);
    assert_eq!(document.snapshot(), before);
}

#[test]
fn rich_exact_input_raw_and_routed_requests_reject_a_deferred_document_mutation() {
    for raw in [true, false] {
        let f = Fixture::new(true);
        let document = f.owner.document();
        let revision = document.revision();
        let batch = document.begin_edit();
        let raw_subscription = raw.then(|| {
            let document = document.clone();
            InputManager::instance()
                .unwrap()
                .pre_process()
                .subscribe_fn(move |event| {
                    if event
                        .downcast_ref::<ferroui_base::input::raw::RawTextInputEventArgs>()
                        .is_some()
                    {
                        document.replace_text(0..0, "external").unwrap();
                    }
                })
        });
        let routed_subscription = (!raw).then(|| {
            let document = document.clone();
            f.owner.add_handler_with(
                InputElement::text_input_event(),
                move |_, _: &TextInputEventArgs| {
                    document.replace_text(0..0, "external").unwrap();
                },
                RoutingStrategies::TUNNEL,
                false,
            )
        });
        assert_eq!(
            f.connection().apply_edit(&edit(1, 3, "stale", 0, 0), 0),
            Err(TextInputMethodError::StateChanged)
        );
        assert_eq!(document.revision(), revision);
        assert_eq!(document.text(), "externalabcd");
        assert!(!f.owner.can_undo());
        if let Some(subscription) = raw_subscription {
            subscription.dispose();
        }
        if let Some(subscription) = routed_subscription {
            f.owner
                .remove_handler(InputElement::text_input_event(), subscription);
        }
        drop(batch);
        assert_eq!(f.owner.text(), "externalabcd");
    }
}

#[test]
fn rich_exact_composition_refreshes_accepted_state_while_document_notifications_are_deferred() {
    let f = Fixture::new(true);
    let document = f.owner.document();
    let revision = document.revision();
    let batch = document.begin_edit();
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "XYZ", 4, 4)).unwrap();
    assert!(session.is_current());
    session.apply_edit(&edit(2, 3, "Q", 3, 3)).unwrap();
    assert!(session.is_current());
    session.select_exact(TextSelection::new(4, 2)).unwrap();
    assert!(session.is_current());
    assert_eq!(document.revision(), revision);
    assert_eq!(document.text(), "aXQZd");
    assert!(session.complete(0).unwrap());
    assert!(!session.is_current());
    assert_eq!(document.revision(), revision);
    assert_eq!(document.text(), "aXQZd");
    assert_eq!(
        f.owner.editor().selection(),
        crate::rich_text::EditorSelection::new(4, 2)
    );
    drop(batch);
    assert_eq!(f.owner.text(), "aXQZd");
}

#[test]
fn rich_exact_composition_intercepted_completion_preserves_preview_for_retry() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    session.apply_edit(&edit(1, 3, "preview", 3, 3)).unwrap();
    let preview = f.owner.document().snapshot();
    let token = f.owner.add_handler_with(
        InputElement::text_input_event(),
        |_, event: &TextInputEventArgs| event.set_handled(true),
        RoutingStrategies::TUNNEL,
        false,
    );
    assert_eq!(session.complete(1), Err(TextInputMethodError::Intercepted));
    assert_eq!(f.owner.document().snapshot(), preview);
    assert!(session.is_current());
    f.owner
        .remove_handler(InputElement::text_input_event(), token);
    assert!(session.complete(2).unwrap());
    assert_eq!(f.owner.text(), "apreviewd");
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.text(), "abcd");
}

#[test]
fn rich_exact_input_selection_cancellation_rolls_back_structured_edit_without_history() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    f.owner.selection_changing(|_, args| args.set_cancel(true));
    assert_eq!(
        f.connection().apply_edit(&edit(0, 1, "XY", 5, 5), 0),
        Err(TextInputMethodError::Rejected)
    );
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!((f.owner.selection_start(), f.owner.selection_end()), (1, 3));
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_exact_composition_reentrant_callback_keeps_application_edit_and_rejects_native_success() {
    let f = Fixture::new(true);
    let session = f.connection().begin_composition().unwrap();
    let owner = f.owner.downgrade();
    f.owner.add_handler(
        RichEditBox::text_composition_changed_event(),
        move |_, _| {
            owner.upgrade().unwrap().set_text("external").unwrap();
        },
    );
    assert_eq!(
        session.apply_edit(&edit(1, 3, "preview", 3, 3)),
        Err(TextInputMethodError::ReentrantEdit)
    );
    assert_eq!(f.owner.text(), "external");
    assert!(!session.is_current());
    assert!(session.complete(0).is_err());
    assert_eq!(f.owner.text(), "external");
}

#[test]
fn rich_exact_input_identical_text_updates_selection_without_destroying_redo_or_formatting() {
    let f = Fixture::new(true);
    f.owner.insert_text("x").unwrap();
    f.owner.undo().unwrap();
    let before = f.owner.document().snapshot();
    assert!(!f
        .connection()
        .apply_edit(&edit(1, 3, "bc", 0, 0), 0)
        .unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(f.owner.can_redo());
}
