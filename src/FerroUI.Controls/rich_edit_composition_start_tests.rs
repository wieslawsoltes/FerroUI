//! EN_STARTCOMPOSITION has a lifecycle before any text exists. Its null native
//! range is independent of the committed selection captured for replacement.
use super::*;
use crate::rich_text::RichTextCompositionChange;

#[test]
fn rich_native_empty_composition_start_and_cancel_preserve_content_selection_and_redo() {
    let f = Fixture::new(true);
    f.owner.insert_text("temporary").unwrap();
    f.owner.undo().unwrap();
    let document = f.owner.document();
    let before = document.snapshot();
    let revision = document.revision();
    let mutation = document.mutation_version();
    let observed = observe(&f.owner);
    let model_changes = Rc::new(Cell::new(0));
    let count = model_changes.clone();
    let model_subscription = document.changed(move |_| count.set(count.get() + 1));
    let selection_changes = Rc::new(Cell::new(0));
    let count = selection_changes.clone();
    let selection_subscription = f
        .owner
        .text_input_method_client()
        .selection_changed(Rc::new(move || count.set(count.get() + 1)));
    let causes = Rc::new(RefCell::new(Vec::new()));
    let recorded = causes.clone();
    let cause_subscription = f
        .owner
        .editor()
        .composition_changed(move |cause| recorded.borrow_mut().push(cause.clone()));

    let session = f.connection().begin_composition().unwrap();
    assert!(session.is_current());
    assert!(f.owner.editor().is_composing());
    assert_eq!(f.owner.editor().active_composition_range(), None);
    assert_eq!(document.snapshot(), before);
    assert_eq!(document.revision(), revision);
    assert_eq!(document.mutation_version(), mutation);
    assert_eq!(f.owner.editor().selection(), EditorSelection::new(1, 3));
    assert_eq!(
        f.owner.text_input_method_client().surrounding_text(),
        "abcd"
    );
    assert_eq!(
        f.owner.text_input_method_client().selection(),
        TextSelection::new(1, 3)
    );
    assert!(f.owner.can_redo());
    assert!(!f.owner.can_undo());
    assert!(causes.borrow().is_empty(), "starting is not a text edit");
    assert_eq!(
        &*observed.borrow(),
        &[ObservedComposition {
            stage: "start",
            text: String::new(),
            start: 0,
            length: 0,
            selection: EditorSelection::new(1, 3),
            committed: false,
        }]
    );

    assert!(session.cancel().unwrap());
    assert!(!session.is_current());
    assert!(!f.owner.editor().is_composing());
    assert_eq!(document.snapshot(), before);
    assert_eq!(document.revision(), revision);
    assert_eq!(document.mutation_version(), mutation);
    assert_eq!(model_changes.get(), 0);
    assert_eq!(selection_changes.get(), 0);
    assert!(f.owner.can_redo());
    assert!(!f.owner.can_undo());
    assert_eq!(
        &*causes.borrow(),
        &[RichTextCompositionChange::Finalized {
            text: String::new(),
        }]
    );
    assert_eq!(
        observed.borrow().last(),
        Some(&ObservedComposition {
            stage: "end",
            text: String::new(),
            start: 0,
            length: 0,
            selection: EditorSelection::new(1, 3),
            committed: false,
        })
    );
    assert_eq!(session.cancel(), Err(TextInputMethodError::StaleConnection));
    assert!(!f.owner.cancel_composition().unwrap());
    assert_eq!(observed.borrow().len(), 2);
    model_subscription.dispose();
    selection_subscription.dispose();
    cause_subscription.dispose();
}

#[test]
fn rich_native_empty_completion_and_empty_commit_close_once_without_undo_or_deletion() {
    for commit_text in [false, true] {
        let f = Fixture::new(true);
        f.owner.insert_text("temporary").unwrap();
        f.owner.undo().unwrap();
        f.owner.select(3, 1);
        let before = f.owner.document().snapshot();
        let mutation = f.owner.document().mutation_version();
        let observed = observe(&f.owner);
        let session = f.connection().begin_composition().unwrap();
        let changed = if commit_text {
            session.commit("", 0).unwrap()
        } else {
            session.complete(0).unwrap()
        };
        assert!(!changed);
        assert_eq!(f.owner.document().snapshot(), before);
        assert_eq!(f.owner.document().mutation_version(), mutation);
        assert_eq!(f.owner.editor().selection(), EditorSelection::new(3, 1));
        assert!(!f.owner.editor().is_composing());
        assert!(!f.owner.can_undo());
        assert!(f.owner.can_redo());
        assert_eq!(observed.borrow().len(), 2);
        assert_eq!(
            observed.borrow().last(),
            Some(&ObservedComposition {
                stage: "end",
                text: String::new(),
                start: 0,
                length: 0,
                selection: EditorSelection::new(3, 1),
                committed: true,
            })
        );
        assert_eq!(
            session.complete(0),
            Err(TextInputMethodError::StaleConnection)
        );
        assert!(!f.owner.complete_input_method_composition().unwrap());
        assert_eq!(observed.borrow().len(), 2);
    }
}

#[test]
fn rich_native_first_exact_replacement_establishes_its_range_independently_of_selection() {
    let f = Fixture::new(true);
    f.owner.select(3, 1);
    let before = f.owner.document().snapshot();
    let observed = observe(&f.owner);
    let session = f.connection().begin_composition().unwrap();
    assert_eq!(f.owner.editor().active_composition_range(), None);
    session.apply_edit(&edit(3, 4, "😀", 5, 5)).unwrap();
    assert_eq!(f.owner.text(), "abc😀");
    assert_eq!(f.owner.editor().active_composition_range(), Some(3..5));
    assert_eq!(
        f.owner.text_input_method_client().surrounding_text(),
        "abcd"
    );
    assert_eq!(observed.borrow().len(), 2);
    assert_eq!(
        observed.borrow().last(),
        Some(&ObservedComposition {
            stage: "change",
            text: "😀".into(),
            start: 3,
            length: 2,
            selection: EditorSelection::caret(5),
            committed: false,
        })
    );
    assert!(session.complete(0).unwrap());
    assert_eq!(observed.borrow().len(), 3);
    assert_eq!(
        (observed.borrow()[2].start, observed.borrow()[2].length),
        (3, 2)
    );
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(f.owner.editor().selection(), EditorSelection::new(3, 1));
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_legacy_first_preedit_still_starts_without_a_native_begin() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    let observed = observe(&f.owner);
    f.owner.set_preedit_text(Some("😀"), Some(2)).unwrap();
    assert_eq!(f.owner.text(), "a😀d");
    assert_eq!(observed.borrow().len(), 2);
    assert_eq!(
        observed.borrow()[0],
        ObservedComposition {
            stage: "start",
            text: "😀".into(),
            start: 1,
            length: 2,
            selection: EditorSelection::caret(3),
            committed: false,
        }
    );
    assert!(f.owner.commit_composition("😀").unwrap());
    assert_eq!(observed.borrow().len(), 3);
    assert_eq!(observed.borrow()[2].stage, "end");
    assert!(f.owner.undo().unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_native_empty_updates_do_not_invent_composition_text_changes() {
    for exact in [false, true] {
        let f = Fixture::new(true);
        f.owner.select(2, 2);
        let before = f.owner.document().snapshot();
        let mutation = f.owner.document().mutation_version();
        let observed = observe(&f.owner);
        let session = f.connection().begin_composition().unwrap();
        if exact {
            assert!(!session.apply_edit(&edit(2, 2, "", 2, 2)).unwrap());
        } else {
            session.update("", Some(0)).unwrap();
        }
        assert_eq!(f.owner.document().snapshot(), before);
        assert_eq!(f.owner.document().mutation_version(), mutation);
        assert_eq!(f.owner.editor().active_composition_range(), Some(2..2));
        assert_eq!(observed.borrow().len(), 1);
        assert!(session.cancel().unwrap());
        assert_eq!(observed.borrow().len(), 2);
        assert_eq!(
            (observed.borrow()[1].start, observed.borrow()[1].length),
            (2, 0)
        );
        assert!(!f.owner.can_undo());
    }
}

#[test]
fn rich_native_start_callback_can_cancel_without_adopting_the_ended_session() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    let observed = observe(&f.owner);
    let owner = f.owner.downgrade();
    f.owner.add_handler(
        RichEditBox::text_composition_started_event(),
        move |_, args| {
            assert_eq!((args.start_index(), args.length()), (0, 0));
            assert!(args.text.is_empty());
            assert!(owner.upgrade().unwrap().cancel_composition().unwrap());
        },
    );
    assert!(matches!(
        f.connection().begin_composition(),
        Err(TextInputMethodError::ReentrantEdit)
    ));
    assert_eq!(f.owner.document().snapshot(), before);
    assert!(!f.owner.editor().is_composing());
    assert!(!f.owner.can_undo());
    assert_eq!(observed.borrow().len(), 2);
    assert_eq!(observed.borrow()[0].stage, "start");
    assert_eq!(observed.borrow()[1].stage, "end");
}

#[test]
fn rich_native_reentrant_begin_from_start_or_end_preserves_the_newest_session() {
    for during_start in [false, true] {
        let f = Fixture::new(true);
        let before = f.owner.document().snapshot();
        let observed = observe(&f.owner);
        let connection = f.connection();
        let old = if during_start {
            None
        } else {
            Some(connection.begin_composition().unwrap())
        };
        let nested = Rc::new(RefCell::new(None));
        let saved = nested.clone();
        let replacement_connection = connection.clone();
        let once = Cell::new(false);
        f.owner.add_handler(
            if during_start {
                RichEditBox::text_composition_started_event()
            } else {
                RichEditBox::text_composition_ended_event()
            },
            move |_, _| {
                if !once.replace(true) {
                    *saved.borrow_mut() = Some(replacement_connection.begin_composition().unwrap());
                }
            },
        );
        assert!(matches!(
            connection.begin_composition(),
            Err(TextInputMethodError::ReentrantEdit)
        ));
        if let Some(old) = old {
            assert!(!old.is_current());
            assert_eq!(old.cancel(), Err(TextInputMethodError::StaleConnection));
        }
        let replacement = nested.borrow_mut().take().unwrap();
        assert!(replacement.is_current());
        assert!(f.owner.editor().is_composing());
        assert_eq!(f.owner.document().snapshot(), before);
        assert_eq!(
            observed
                .borrow()
                .iter()
                .map(|event| event.stage)
                .collect::<Vec<_>>(),
            ["start", "end", "start"]
        );
        assert!(replacement.cancel().unwrap());
        assert_eq!(
            observed
                .borrow()
                .iter()
                .map(|event| event.stage)
                .collect::<Vec<_>>(),
            ["start", "end", "start", "end"]
        );
        assert!(!f.owner.can_undo());
    }
}

#[test]
fn rich_native_start_focus_handoff_invalidates_the_original_semantic_owner() {
    let f = Fixture::new(true);
    let connection = f.connection();
    let before = f.owner.document().snapshot();
    let observed = observe(&f.owner);
    let outside = f.outside.downgrade();
    f.owner.add_handler(
        RichEditBox::text_composition_started_event(),
        move |_, _| {
            outside.upgrade().unwrap().focus();
        },
    );
    assert!(matches!(
        connection.begin_composition(),
        Err(TextInputMethodError::ReentrantEdit)
    ));
    assert!(f.outside.is_focused());
    assert!(!connection.is_current());
    assert!(!f.owner.editor().is_composing());
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(
        observed
            .borrow()
            .iter()
            .map(|event| event.stage)
            .collect::<Vec<_>>(),
        ["start", "end"]
    );
    assert!(!f.owner.can_undo());
}

#[test]
fn rich_native_restart_does_not_adopt_external_changes_during_cancellation() {
    let f = Fixture::new(true);
    let observed = observe(&f.owner);
    let connection = f.connection();
    let previous = connection.begin_composition().unwrap();
    previous.update("preview", None).unwrap();
    let owner = f.owner.downgrade();
    let once = Cell::new(false);
    f.owner
        .add_handler(RichEditBox::text_changing_event(), move |_, _| {
            if !once.replace(true) {
                owner.upgrade().unwrap().set_text("external").unwrap();
            }
        });
    assert!(matches!(
        connection.begin_composition(),
        Err(TextInputMethodError::ReentrantEdit)
    ));
    assert_eq!(f.owner.text(), "external");
    assert!(!f.owner.editor().is_composing());
    assert!(!previous.is_current());
    assert!(!f.owner.can_undo());
    assert_eq!(
        observed
            .borrow()
            .iter()
            .filter(|event| event.stage == "start")
            .count(),
        1,
        "the interrupted restart must not publish a newer session"
    );
}

#[test]
fn rich_native_intercepted_empty_completion_keeps_its_lifecycle_open_for_retry() {
    let f = Fixture::new(true);
    let before = f.owner.document().snapshot();
    let observed = observe(&f.owner);
    let session = f.connection().begin_composition().unwrap();
    let token = f.owner.add_handler_with(
        InputElement::text_input_event(),
        |_, event: &TextInputEventArgs| event.set_handled(true),
        RoutingStrategies::TUNNEL,
        false,
    );
    assert_eq!(session.complete(0), Err(TextInputMethodError::Intercepted));
    assert!(session.is_current());
    assert!(f.owner.editor().is_composing());
    assert_eq!(observed.borrow().len(), 1);
    f.owner
        .remove_handler(InputElement::text_input_event(), token);
    assert!(!session.complete(0).unwrap());
    assert_eq!(f.owner.document().snapshot(), before);
    assert_eq!(observed.borrow().len(), 2);
    assert!(!f.owner.can_undo());
}
