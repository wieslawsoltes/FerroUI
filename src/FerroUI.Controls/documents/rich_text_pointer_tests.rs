// Independent regressions for live pointer coordinates during model transactions.
use super::*;
use crate::documents::CharacterFormat;
use crate::rich_text::{EditError, RichEditTextDocument};
use ferroui_base::media::FontWeight;
use ferroui_base::threading::Dispatcher;

#[test]
fn rich_pointer_created_after_an_edit_does_not_replay_that_edit_at_batch_end() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abcd");
    let before_end = document.create_pointer(4, TextGravity::Forward).unwrap();
    let events = Rc::new(Cell::new(0));
    let received = events.clone();
    let _subscription = document.changed(move |_| received.set(received.get() + 1));
    let batch = document.begin_edit();
    document.replace_text(1..3, "😀Q").unwrap();
    assert_eq!(document.text(), "a😀Qd");
    assert_eq!(before_end.offset(), 5);
    let after_edit = document.create_pointer(4, TextGravity::Forward).unwrap();
    assert_eq!(after_edit.offset(), 4);
    assert_eq!(document.revision(), 0);
    assert_eq!(events.get(), 0);
    drop(batch);
    assert_eq!(document.revision(), 1);
    assert_eq!(events.get(), 1);
    assert_eq!(before_end.offset(), 5);
    assert_eq!(after_edit.offset(), 4);
}

#[test]
fn rich_pointer_created_between_nested_replacements_tracks_only_later_changes() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abcdef");
    let original = document.create_pointer(2, TextGravity::Forward).unwrap();
    let outer = document.begin_edit();
    document.replace_text(0..0, "X").unwrap();
    assert_eq!(original.offset(), 3);
    let between = document.create_pointer(3, TextGravity::Forward).unwrap();
    let inner = document.begin_edit();
    document.replace_text(4..6, "😀").unwrap();
    document.replace_text(0..0, "Y").unwrap();
    assert_eq!(document.text(), "YXabc😀f");
    assert_eq!(original.offset(), 4);
    assert_eq!(between.offset(), 4);
    let last = document.create_pointer(5, TextGravity::Backward).unwrap();
    drop(inner);
    assert_eq!(document.revision(), 0);
    drop(outer);
    assert_eq!(document.revision(), 1);
    assert_eq!(original.offset(), 4);
    assert_eq!(between.offset(), 4);
    assert_eq!(last.offset(), 5);
}

#[test]
fn rich_pointer_gravity_remains_scalar_safe_before_public_batch_notifications() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("a😀b");
    let backward = document.create_pointer(1, TextGravity::Backward).unwrap();
    let forward = document.create_pointer(1, TextGravity::Forward).unwrap();
    let end = document.create_pointer(3, TextGravity::Forward).unwrap();
    let batch = document.begin_edit();
    document.replace_text(1..1, "X").unwrap();
    assert_eq!(backward.offset(), 1);
    assert_eq!(forward.offset(), 2);
    assert_eq!(end.offset(), 4);
    let created = end.get_position_at_offset(0, TextGravity::Forward).unwrap();
    document.replace_text(2..4, "").unwrap();
    for pointer in [&forward, &end, &created] {
        assert_eq!(pointer.offset(), 2);
        document
            .validate_range(pointer.offset()..pointer.offset())
            .unwrap();
    }
    drop(batch);
    assert_eq!(document.text(), "aXb");
    assert_eq!(backward.offset(), 1);
    assert_eq!(created.offset(), 2);
}

#[test]
fn rich_pointer_new_and_existing_positions_follow_failed_group_rollback_once() {
    let _scope = Dispatcher::unit_test_scope();
    let services = RichEditTextDocument::new(RichTextDocument::from_text("abcd"));
    let document = services.document();
    let end = document.create_pointer(4, TextGravity::Forward).unwrap();
    let range = services.get_range(1, 3).unwrap();
    let created = Rc::new(RefCell::new(None));
    let result: Result<(), EditError> = services.edit_group(|_| {
        range.set_text("😀Q")?;
        assert_eq!(range.range(), 1..4);
        assert_eq!(end.offset(), 5);
        *created.borrow_mut() = Some(document.create_pointer(4, TextGravity::Forward)?);
        Err(EditError::ReadOnly)
    });
    assert!(result.is_err());
    assert_eq!(document.text(), "abcd");
    assert_eq!(end.offset(), 4);
    assert_eq!(range.range(), 1..3);
    assert_eq!(created.borrow().as_ref().unwrap().offset(), 3);
    assert!(!services.can_undo());
}

#[test]
fn rich_pointer_range_moved_inside_nested_editor_groups_uses_current_coordinates() {
    let _scope = Dispatcher::unit_test_scope();
    let services = RichEditTextDocument::new(RichTextDocument::from_text("abcd"));
    let range = services.get_range(1, 3).unwrap();
    services
        .edit_group(|services| {
            range.set_text("😀Q")?;
            assert_eq!(range.range(), 1..4);
            range.set_range(4, 4)?;
            services.edit_group(|services| services.get_range(0, 0)?.set_text("Z"))?;
            assert_eq!(range.range(), 5..5);
            Ok(())
        })
        .unwrap();
    assert_eq!(services.text(), "Za😀Qd");
    assert_eq!(range.range(), 5..5);
    assert!(services.undo().unwrap());
    assert_eq!(services.text(), "abcd");
    assert_eq!(range.range(), 3..3);
    assert!(services.redo().unwrap());
    assert_eq!(services.text(), "Za😀Qd");
    assert_eq!(range.range(), 5..5);
}

#[test]
fn rich_pointer_reentrant_notifications_see_current_positions_in_registration_order() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("ab");
    let old: Rc<RefCell<Option<TextPointer>>> = Rc::new(RefCell::new(None));
    let created: Rc<RefCell<Option<TextPointer>>> = Rc::new(RefCell::new(None));
    let revisions = Rc::new(RefCell::new(Vec::new()));
    let (tracked, later, received) = (old.clone(), created.clone(), revisions.clone());
    let weak = document.downgrade();
    // Register the application observer before the original pointer. Pointer
    // correctness must not depend on its place in the public observer list.
    let _subscription = document.changed(move |change| {
        let document = weak.upgrade().unwrap();
        received.borrow_mut().push(change.revision);
        if change.revision == 1 {
            assert_eq!(tracked.borrow().as_ref().unwrap().offset(), 3);
            document.replace_text(0..0, "Y").unwrap();
            assert_eq!(tracked.borrow().as_ref().unwrap().offset(), 4);
            *later.borrow_mut() = Some(document.create_pointer(4, TextGravity::Forward).unwrap());
        } else {
            assert_eq!(document.text(), "YXab");
            assert_eq!(later.borrow().as_ref().unwrap().offset(), 4);
        }
    });
    *old.borrow_mut() = Some(document.create_pointer(2, TextGravity::Forward).unwrap());
    document.replace_text(0..0, "X").unwrap();
    assert_eq!(*revisions.borrow(), [1, 2]);
    assert_eq!(old.borrow().as_ref().unwrap().offset(), 4);
    assert_eq!(created.borrow().as_ref().unwrap().offset(), 4);
}

#[test]
fn rich_pointer_tracking_keeps_weak_document_and_subscription_lifetimes() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("ab");
    let pointer = document.create_pointer(1, TextGravity::Forward).unwrap();
    let pointer_weak = Rc::downgrade(&pointer.0);
    let retained = pointer.clone();
    drop(pointer);
    assert!(pointer_weak.upgrade().is_some());
    drop(retained);
    assert!(pointer_weak.upgrade().is_none());
    document.replace_text(0..0, "X").unwrap();
    let remaining = document.create_pointer(1, TextGravity::Forward).unwrap();
    let weak = document.downgrade();
    drop(document);
    assert!(weak.upgrade().is_none());
    assert!(!remaining.is_valid());
    assert_eq!(remaining.offset(), 1);
}

#[test]
fn rich_pointer_formatting_and_rejected_utf16_edits_do_not_rebase_positions() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("a😀b");
    let pointer = document.create_pointer(3, TextGravity::Forward).unwrap();
    let batch = document.begin_edit();
    document
        .apply_format(
            0..4,
            &CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..Default::default()
            },
        )
        .unwrap();
    let formatted = document.create_pointer(3, TextGravity::Backward).unwrap();
    assert!(document.replace_text(1..2, "bad").is_err());
    assert_eq!(pointer.offset(), 3);
    assert_eq!(formatted.offset(), 3);
    drop(batch);
    assert_eq!(pointer.offset(), 3);
    assert_eq!(formatted.offset(), 3);
    assert_eq!(document.text(), "a😀b");
}
