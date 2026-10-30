// Independent IME transaction regressions. TextEdit payload expectations follow
// Microsoft's TextEdit control-pattern guidance: changed preview substring, full
// accepted finalization, and an empty finalization for cancellation.
use super::*;
use ferroui_base::media::FontWeight;
use ferroui_base::threading::Dispatcher;

fn preview(text: &str) -> RichTextCompositionChange {
    RichTextCompositionChange::Preview {
        changed_text: text.to_owned(),
    }
}

fn finalized(text: &str) -> RichTextCompositionChange {
    RichTextCompositionChange::Finalized {
        text: text.to_owned(),
    }
}

fn collect_changes(
    editor: &RichTextEditor,
) -> (
    Rc<RefCell<Vec<RichTextCompositionChange>>>,
    Rc<dyn IDisposable>,
) {
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = changes.clone();
    let subscription = editor.composition_changed(move |change| {
        observed.borrow_mut().push(change.clone());
    });
    (changes, subscription)
}

#[test]
fn rich_composition_preview_reports_only_accepted_unicode_replacements() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichTextEditor::new(RichTextDocument::from_text("[]"));
    editor.set_selection(EditorSelection::caret(1)).unwrap();
    let (changes, _subscription) = collect_changes(&editor);
    editor.set_preedit("A😀e\u{301}Z", None).unwrap();
    editor.set_preedit("A😃e\u{301}Z", None).unwrap();
    editor.set_preedit("A😃e\u{300}Z", None).unwrap();
    editor.set_preedit("A😃e\u{300}", None).unwrap();
    editor.set_preedit("A😃e\u{300}", Some(0)).unwrap();
    assert_eq!(
        *changes.borrow(),
        vec![
            preview("A😀e\u{301}Z"),
            preview("😃"),
            preview("\u{300}"),
            preview("")
        ]
    );
    assert_eq!(editor.document().text(), "[A😃e\u{300}]");
    assert_eq!(editor.active_composition_range(), Some(1..6));
    assert_eq!(editor.selection(), EditorSelection::caret(1));
    assert!(!editor.can_undo());
}

#[test]
fn rich_composition_preview_uses_normalized_limited_text_and_current_cursor() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichTextEditor::new(RichTextDocument::from_text("LxR"));
    editor.set_selection(EditorSelection::new(1, 2)).unwrap();
    editor.set_max_length(6);
    let (changes, _subscription) = collect_changes(&editor);
    editor.set_preedit("a\r\n😀z", Some(3)).unwrap();
    assert_eq!(editor.document().text(), "La\n😀R");
    assert_eq!(editor.active_composition_range(), Some(1..5));
    assert_eq!(editor.selection(), EditorSelection::caret(3));
    assert_eq!(editor.surrounding_text(), "LxR");
    assert_eq!(editor.input_method_selection(), EditorSelection::new(1, 2));
    assert_eq!(*changes.borrow(), vec![preview("a\n😀")]);
}

#[test]
fn rich_composition_model_and_editor_callbacks_observe_the_complete_accepted_preview() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("qxz");
    let editor = Rc::new(RichTextEditor::new(document.clone()));
    editor.set_selection(EditorSelection::new(1, 2)).unwrap();
    let order = Rc::new(RefCell::new(Vec::new()));
    let observed = order.clone();
    let weak = Rc::downgrade(&editor);
    let _model = document.changed(move |_| {
        let editor = weak.upgrade().unwrap();
        assert_eq!(editor.document().text(), "q日😀z");
        assert_eq!(editor.active_composition_range(), Some(1..4));
        assert_eq!(editor.surrounding_text(), "qxz");
        assert_eq!(editor.input_method_selection(), EditorSelection::new(1, 2));
        observed.borrow_mut().push("model");
    });
    let observed = order.clone();
    let weak = Rc::downgrade(&editor);
    let _editor = editor.changed(move || {
        assert_eq!(
            weak.upgrade().unwrap().active_composition_range(),
            Some(1..4)
        );
        observed.borrow_mut().push("editor");
    });
    let observed = order.clone();
    let weak = Rc::downgrade(&editor);
    let _composition = editor.composition_changed(move |change| {
        assert_eq!(*change, preview("日😀"));
        assert_eq!(
            weak.upgrade().unwrap().active_composition_range(),
            Some(1..4)
        );
        observed.borrow_mut().push("composition");
    });
    editor.set_preedit("日😀", Some(1)).unwrap();
    assert_eq!(*order.borrow(), vec!["model", "editor", "composition"]);
}

#[test]
fn rich_composition_commit_publishes_one_accepted_finalization_and_one_undo_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abc");
    document
        .apply_format(
            1..2,
            &CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..Default::default()
            },
        )
        .unwrap();
    let original = document.snapshot();
    let editor = Rc::new(RichTextEditor::new(document.clone()));
    editor.set_selection(EditorSelection::new(1, 2)).unwrap();
    editor.set_preedit("日", None).unwrap();
    editor.set_preedit("日本", None).unwrap();
    editor.set_max_length(4);
    let (changes, _subscription) = collect_changes(&editor);
    let model_texts = Rc::new(RefCell::new(Vec::new()));
    let observed = model_texts.clone();
    let weak = Rc::downgrade(&editor);
    let model = document.changed(move |_| {
        let editor = weak.upgrade().unwrap();
        assert_eq!(editor.active_composition_range(), None);
        observed.borrow_mut().push(editor.document().text());
    });
    editor.set_selection_projection(Some(Rc::new(|_| EditorSelection::caret(0))));
    assert!(editor.commit_composition("😀z").unwrap());
    assert_eq!(document.text(), "a😀c");
    assert_eq!(editor.selection(), EditorSelection::caret(0));
    assert_eq!(*model_texts.borrow(), vec!["a😀c"]);
    assert_eq!(*changes.borrow(), vec![finalized("😀")]);
    assert!(!editor.is_composing());
    model.dispose();
    editor.set_selection_projection(None);
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), original);
    assert_eq!(editor.selection(), EditorSelection::new(1, 2));
    assert!(!editor.can_undo());
    assert!(editor.redo().unwrap());
    assert_eq!(document.text(), "a😀c");
    assert_eq!(editor.selection(), EditorSelection::caret(0));
    assert_eq!(*changes.borrow(), vec![finalized("😀")]);
}

#[test]
fn rich_composition_typing_commit_does_not_publish_a_cancellation_first() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichTextEditor::new(RichTextDocument::from_text("old"));
    editor.set_selection(EditorSelection::new(0, 3)).unwrap();
    editor.set_preedit("preview", None).unwrap();
    let (changes, _subscription) = collect_changes(&editor);
    assert!(editor.insert_text("final", EditKind::Typing).unwrap());
    assert_eq!(editor.document().text(), "final");
    assert_eq!(*changes.borrow(), vec![finalized("final")]);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.document().text(), "old");
    assert!(!editor.can_undo());
}

#[test]
fn rich_composition_explicit_cancel_is_empty_once_and_restores_directed_selection() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abcd");
    let original = document.snapshot();
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(3, 1)).unwrap();
    editor.set_preedit("候補", None).unwrap();
    let (changes, _subscription) = collect_changes(&editor);
    editor.set_is_read_only(true);
    assert!(editor.cancel_composition().unwrap());
    assert!(!editor.cancel_composition().unwrap());
    assert_eq!(document.snapshot(), original);
    assert_eq!(editor.selection(), EditorSelection::new(3, 1));
    assert_eq!(editor.active_composition_range(), None);
    assert_eq!(*changes.borrow(), vec![finalized("")]);
    assert!(!editor.can_undo());
}

#[test]
fn rich_composition_read_only_and_reentrant_errors_preserve_preview_without_false_events() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = Rc::new(RichTextEditor::new(RichTextDocument::from_text("base")));
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let weak = Rc::downgrade(&editor);
    let _subscription = editor.composition_changed(move |_| {
        let editor = weak.upgrade().unwrap();
        let before = editor.document().snapshot();
        assert!(matches!(
            editor.set_preedit("nested", None),
            Err(EditError::ReentrantEdit)
        ));
        assert!(matches!(
            editor.commit_composition("nested"),
            Err(EditError::ReentrantEdit)
        ));
        assert!(matches!(
            editor.cancel_composition(),
            Err(EditError::ReentrantEdit)
        ));
        assert_eq!(editor.document().snapshot(), before);
        observed.set(observed.get() + 1);
    });
    editor.set_preedit("日", None).unwrap();
    let before = editor.document().snapshot();
    let selection = editor.selection();
    editor.set_is_read_only(true);
    assert!(matches!(
        editor.set_preedit("other", None),
        Err(EditError::ReadOnly)
    ));
    assert!(matches!(
        editor.commit_composition("other"),
        Err(EditError::ReadOnly)
    ));
    assert_eq!(editor.document().snapshot(), before);
    assert_eq!(editor.selection(), selection);
    assert_eq!(editor.active_composition_range(), Some(0..1));
    assert_eq!(editor.surrounding_text(), "base");
    assert_eq!(calls.get(), 1);
    editor.set_is_read_only(false);
    editor.commit_composition("final").unwrap();
    assert_eq!(calls.get(), 2);
}

#[test]
fn rich_composition_empty_and_fully_rejected_commits_restore_the_committed_selection() {
    let _scope = Dispatcher::unit_test_scope();
    for text in ["", "😀"] {
        let document = RichTextDocument::from_text("abc");
        let original = document.snapshot();
        let editor = RichTextEditor::new(document.clone());
        editor.set_selection(EditorSelection::new(1, 2)).unwrap();
        editor.set_max_length(3);
        editor.set_preedit("x", None).unwrap();
        let (changes, _subscription) = collect_changes(&editor);
        assert!(!editor.commit_composition(text).unwrap());
        assert_eq!(document.snapshot(), original);
        assert_eq!(editor.selection(), EditorSelection::new(1, 2));
        assert_eq!(editor.active_composition_range(), None);
        assert_eq!(*changes.borrow(), vec![finalized("")]);
        assert!(!editor.can_undo());
    }
}

#[test]
fn rich_composition_ordinary_edits_and_commits_without_preview_do_not_fabricate_causes() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichTextEditor::new(RichTextDocument::new());
    let (changes, _subscription) = collect_changes(&editor);
    editor.insert_text("typed", EditKind::Typing).unwrap();
    editor.insert_text(" pasted", EditKind::Paste).unwrap();
    editor.commit_composition(" direct").unwrap();
    assert!(!editor.cancel_composition().unwrap());
    assert_eq!(editor.document().text(), "typed pasted direct");
    assert!(changes.borrow().is_empty());
    assert_eq!(editor.active_composition_range(), None);
}

#[test]
fn rich_composition_nested_groups_deliver_causes_in_order_after_the_group_settles() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = Rc::new(RichTextEditor::new(RichTextDocument::from_text("!")));
    let (changes, _subscription) = collect_changes(&editor);
    let in_group = Rc::new(Cell::new(true));
    let flag = in_group.clone();
    let weak = Rc::downgrade(&editor);
    let _observer = editor.composition_changed(move |_| {
        assert!(!flag.get());
        let editor = weak.upgrade().unwrap();
        assert_eq!(editor.document().text(), "a日本語!");
        assert_eq!(editor.active_composition_range(), None);
    });
    editor
        .edit_group(|editor| {
            editor.insert_text("a", EditKind::Programmatic)?;
            editor.edit_group(|editor| {
                editor.set_preedit("日", None)?;
                editor.set_preedit("日本", None)?;
                assert_eq!(editor.active_composition_range(), Some(1..3));
                assert!(changes.borrow().is_empty());
                editor.commit_composition("日本語")?;
                Ok(())
            })?;
            assert!(changes.borrow().is_empty());
            in_group.set(false);
            Ok(())
        })
        .unwrap();
    assert_eq!(
        *changes.borrow(),
        vec![preview("日"), preview("本"), finalized("日本語")]
    );
    assert!(editor.undo().unwrap());
    assert_eq!(editor.document().text(), "!");
    assert!(!editor.can_undo());
}

#[test]
fn rich_composition_preview_only_group_never_adds_an_undo_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichTextEditor::new(RichTextDocument::from_text("base"));
    let (changes, _subscription) = collect_changes(&editor);
    editor
        .edit_group(|editor| {
            editor.set_preedit("preview", None)?;
            assert!(changes.borrow().is_empty());
            Ok(())
        })
        .unwrap();
    assert_eq!(editor.active_composition_range(), Some(0..7));
    assert_eq!(*changes.borrow(), vec![preview("preview")]);
    assert!(!editor.can_undo());
    editor.cancel_composition().unwrap();
    assert_eq!(editor.document().text(), "base");
    assert!(!editor.can_undo());
}

#[test]
fn rich_composition_group_undo_snapshots_exclude_an_uncommitted_preview() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichTextEditor::new(RichTextDocument::new());
    editor.insert_text("A", EditKind::Typing).unwrap();
    editor
        .edit_group(|editor| {
            editor.insert_text("B", EditKind::Programmatic)?;
            editor.set_preedit("日", None)
        })
        .unwrap();
    assert_eq!(editor.document().text(), "AB日");
    assert_eq!(editor.surrounding_text(), "AB");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.document().text(), "A");
    assert!(editor.redo().unwrap());
    assert_eq!(editor.document().text(), "AB");
    assert_eq!(editor.active_composition_range(), None);
    assert!(editor.undo().unwrap());
    assert!(editor.undo().unwrap());
    assert_eq!(editor.document().text(), "");
}

#[test]
fn rich_composition_failed_groups_discard_causes_and_restore_composition_history_and_limits() {
    let _scope = Dispatcher::unit_test_scope();
    for commit in [false, true] {
        let editor = RichTextEditor::new(RichTextDocument::from_text("base"));
        editor.insert_text("!", EditKind::Typing).unwrap();
        let before = editor.document().snapshot();
        let selection = editor.selection();
        let (changes, _subscription) = collect_changes(&editor);
        let result: Result<(), EditError> = editor.edit_group(|editor| {
            editor.set_max_length(100);
            editor.set_preedit("日", None)?;
            if commit {
                editor.commit_composition("日本")?;
            }
            Err(EditError::ReadOnly)
        });
        assert!(matches!(result, Err(EditError::ReadOnly)));
        assert_eq!(editor.document().snapshot(), before);
        assert_eq!(editor.selection(), selection);
        assert_eq!(editor.max_length(), 0);
        assert_eq!(editor.active_composition_range(), None);
        assert!(!editor.is_composing());
        assert!(changes.borrow().is_empty());
        assert!(editor.undo().unwrap());
        assert_eq!(editor.document().text(), "base");
        assert!(changes.borrow().is_empty());
    }
}

#[test]
fn rich_composition_outer_model_batch_publication_preserves_preview_and_prior_undo() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::new();
    let editor = RichTextEditor::new(document.clone());
    editor.insert_text("a", EditKind::Typing).unwrap();
    let revision = document.revision();
    let batch = document.begin_edit();
    editor.set_preedit("日", None).unwrap();
    assert_eq!(document.revision(), revision);
    assert_eq!(editor.active_composition_range(), Some(1..2));
    assert!(editor.can_undo());
    batch.finish();
    assert_ne!(document.revision(), revision);
    assert_eq!(editor.active_composition_range(), Some(1..2));
    assert!(editor.can_undo());
    editor.commit_composition("日本").unwrap();
    assert_eq!(document.text(), "a日本");
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "a");
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "");
}

#[test]
fn rich_composition_external_text_or_format_changes_inside_a_batch_invalidate_the_preview() {
    let _scope = Dispatcher::unit_test_scope();
    for formatting in [false, true] {
        let document = RichTextDocument::from_text("base");
        let editor = RichTextEditor::new(document.clone());
        editor.insert_text("!", EditKind::Typing).unwrap();
        editor.set_preedit("日", None).unwrap();
        assert!(editor.can_undo());
        let revision = document.revision();
        let batch = document.begin_edit();
        if formatting {
            document
                .apply_format(
                    1..2,
                    &CharacterFormat {
                        font_weight: Some(FontWeight::Bold),
                        ..Default::default()
                    },
                )
                .unwrap();
        } else {
            document.replace_text(1..2, "外").unwrap();
        }
        assert_eq!(document.revision(), revision);
        assert_eq!(editor.active_composition_range(), None);
        assert!(!editor.can_undo());
        assert_eq!(editor.surrounding_text(), document.text());
        let external = document.snapshot();
        assert!(editor.cancel_composition().unwrap());
        assert_eq!(document.snapshot(), external);
        batch.finish();
        assert!(!editor.can_undo());
        assert!(!editor.is_composing());
    }
}

#[test]
fn rich_composition_selection_projection_rejects_mutating_preview_reentry() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = Rc::new(RichTextEditor::new(RichTextDocument::from_text("abc")));
    let (changes, _subscription) = collect_changes(&editor);
    let calls = Rc::new(Cell::new(0));
    let observed = calls.clone();
    let weak = Rc::downgrade(&editor);
    editor.set_selection_projection(Some(Rc::new(move |selection| {
        let editor = weak.upgrade().unwrap();
        assert!(matches!(
            editor.set_preedit("nested", None),
            Err(EditError::ReentrantEdit)
        ));
        assert!(matches!(
            editor.commit_composition("nested"),
            Err(EditError::ReentrantEdit)
        ));
        assert!(matches!(
            editor.cancel_composition(),
            Err(EditError::ReentrantEdit)
        ));
        observed.set(observed.get() + 1);
        selection
    })));
    editor.set_selection(EditorSelection::caret(1)).unwrap();
    assert!(calls.get() > 0);
    assert_eq!(editor.document().text(), "abc");
    assert!(!editor.is_composing());
    assert!(changes.borrow().is_empty());
}

#[test]
fn rich_composition_subscriptions_are_disposable_and_do_not_retain_the_editor() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = Rc::new(RichTextEditor::new(RichTextDocument::new()));
    let (changes, subscription) = collect_changes(&editor);
    editor.set_preedit("first", None).unwrap();
    assert_eq!(*changes.borrow(), vec![preview("first")]);
    subscription.dispose();
    editor.set_preedit("second", None).unwrap();
    editor.cancel_composition().unwrap();
    assert_eq!(*changes.borrow(), vec![preview("first")]);
    let weak = Rc::downgrade(&editor);
    drop(editor);
    assert!(weak.upgrade().is_none());
    subscription.dispose();
}
