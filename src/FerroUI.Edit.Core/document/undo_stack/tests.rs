use super::*;
use std::cell::{Cell, RefCell};

struct ValueOperation {
    value: Rc<Cell<i32>>,
    before: i32,
    after: i32,
    events: Rc<RefCell<Vec<String>>>,
}
impl IUndoableOperation for ValueOperation {
    fn undo(&self) {
        self.value.set(self.before);
        self.events
            .borrow_mut()
            .push(format!("undo {}", self.before));
    }
    fn redo(&self) {
        self.value.set(self.after);
        self.events
            .borrow_mut()
            .push(format!("redo {}", self.after));
    }
}
fn action(
    value: &Rc<Cell<i32>>,
    before: i32,
    after: i32,
    events: &Rc<RefCell<Vec<String>>>,
) -> Rc<dyn IUndoableOperation> {
    value.set(after);
    Rc::new(ValueOperation {
        value: value.clone(),
        before,
        after,
        events: events.clone(),
    })
}

#[test]
fn custom_actions_interleave_with_document_changes_in_lifo_order() {
    let document = TextDocument::from_text("a");
    let stack = document.undo_stack();
    let value = Rc::new(Cell::new(0));
    let events = Rc::new(RefCell::new(Vec::new()));
    let log = events.clone();
    let _subscription = document.changed(move |e| {
        log.borrow_mut()
            .push(format!("text {}", e.inserted_text.text()))
    });
    let group = stack.start_undo_group().unwrap();
    stack.push(action(&value, 0, 1, &events)).unwrap();
    document.insert(1, "b").unwrap();
    stack.push(action(&value, 1, 2, &events)).unwrap();
    group.finish().unwrap();
    assert_eq!(stack.undo_count(), 1);
    events.borrow_mut().clear();
    stack.undo().unwrap();
    assert_eq!(document.text(), "a");
    assert_eq!(value.get(), 0);
    assert_eq!(&*events.borrow(), &["undo 1", "text ", "undo 0"]);
    assert!(stack.is_original_file());
    events.borrow_mut().clear();
    stack.redo().unwrap();
    assert_eq!(document.text(), "ab");
    assert_eq!(value.get(), 2);
    assert_eq!(&*events.borrow(), &["redo 1", "text b", "redo 2"]);
}

#[test]
fn optional_actions_require_a_group_and_do_not_create_standalone_history() {
    let stack = UndoStack::new();
    let value = Rc::new(Cell::new(0));
    let events = Rc::new(RefCell::new(Vec::new()));
    assert_eq!(
        stack.push_optional(action(&value, 0, 1, &events)),
        Err(DocumentError::NoUpdate)
    );
    let group = stack.start_undo_group().unwrap();
    stack.push_optional(action(&value, 1, 2, &events)).unwrap();
    group.finish().unwrap();
    assert!(!stack.can_undo());
    assert!(stack.is_original_file());
    let group = stack.start_undo_group().unwrap();
    stack.push_optional(action(&value, 2, 3, &events)).unwrap();
    stack.document().insert(0, "x").unwrap();
    group.finish().unwrap();
    assert_eq!(stack.undo_count(), 1);
    stack.undo().unwrap();
    assert_eq!(value.get(), 2);
    assert_eq!(stack.document().text(), "");
}

#[test]
fn optional_continuation_joins_previous_group_without_marking_new_text_state() {
    let stack = UndoStack::new();
    let value = Rc::new(Cell::new(0));
    let events = Rc::new(RefCell::new(Vec::new()));
    stack.document().insert(0, "x").unwrap();
    stack.mark_as_original_file();
    let group = stack.start_continued_undo_group().unwrap();
    stack.push_optional(action(&value, 0, 1, &events)).unwrap();
    group.finish().unwrap();
    assert_eq!(stack.undo_count(), 1);
    assert!(stack.is_original_file());
    stack.undo().unwrap();
    assert_eq!(value.get(), 0);
    assert!(!stack.is_original_file());
    stack.redo().unwrap();
    assert_eq!(value.get(), 1);
    assert!(stack.is_original_file());
}

#[test]
fn shared_groups_replay_both_documents_and_preserve_their_update_order() {
    struct RecordOrigin(WeakTextDocument, Rc<Cell<usize>>);
    impl IUndoableOperationWithContext for RecordOrigin {
        fn undo(&self, stack: &UndoStack) {
            assert_eq!(Some(stack.document()), self.0.upgrade());
            self.1.set(self.1.get() + 1);
        }

        fn redo(&self, stack: &UndoStack) {
            self.undo(stack);
        }
    }

    let first = TextDocument::from_text("a");
    let second = TextDocument::from_text("x");
    let stack = first.undo_stack();
    second.set_undo_stack(&stack).unwrap();
    assert_eq!(stack, second.undo_stack());
    let events = Rc::new(RefCell::new(Vec::new()));
    let subscriptions: Vec<_> = [(&first, "a"), (&second, "b")]
        .into_iter()
        .map(|(document, name)| {
            let log = events.clone();
            let started = document.update_started(move || log.borrow_mut().push((name, "start")));
            let log = events.clone();
            let changed = document.text_changed(move || log.borrow_mut().push((name, "text")));
            let log = events.clone();
            let finished = document.update_finished(move || log.borrow_mut().push((name, "finish")));
            (started, changed, finished)
        })
        .collect();

    let group = stack.start_undo_group().unwrap();
    first.insert(1, "b").unwrap();
    second.insert(1, "y").unwrap();
    let recorded = Rc::new(Cell::new(0));
    let operation = Rc::new(RecordOrigin(second.downgrade(), recorded.clone()));
    second.undo_stack().push_with_context(operation).unwrap();
    group.finish().unwrap();
    assert_eq!(stack.undo_count(), 1);
    assert!(first.is_modified() && second.is_modified());
    assert_eq!(*events.borrow(), [
        ("a", "start"), ("a", "text"), ("a", "finish"),
        ("b", "start"), ("b", "text"), ("b", "finish"),
    ]);
    events.borrow_mut().clear();

    second.set_is_read_only(true);
    assert_eq!(first.undo(), Err(DocumentError::ReadOnly));
    assert_eq!((first.text(), second.text()), ("ab".into(), "xy".into()));
    assert_eq!(stack.undo_count(), 1);
    assert!(events.borrow().is_empty());
    second.set_is_read_only(false);

    assert!(first.undo().unwrap());
    assert_eq!((first.text(), second.text()), ("a".into(), "x".into()));
    assert!(!first.is_modified() && !second.is_modified());
    assert_eq!(*events.borrow(), [
        ("b", "start"), ("a", "start"),
        ("b", "text"), ("b", "finish"), ("a", "text"), ("a", "finish"),
    ]);
    events.borrow_mut().clear();
    assert!(first.redo().unwrap());
    assert_eq!((first.text(), second.text()), ("ab".into(), "xy".into()));
    assert_eq!(*events.borrow(), [
        ("a", "start"), ("b", "start"),
        ("a", "text"), ("a", "finish"), ("b", "text"), ("b", "finish"),
    ]);
    assert!(first.is_modified() && second.is_modified());
    assert_eq!(recorded.get(), 2);
    second.mark_as_original_file();
    assert!(!first.is_modified() && !second.is_modified());
    drop(subscriptions);
}

#[test]
fn rebinding_clears_old_history_and_shared_history_releases_closed_documents() {
    let first = TextDocument::from_text("a");
    let second = TextDocument::from_text("x");
    let stack = first.undo_stack();
    let previous = second.undo_stack();
    second.insert(1, "y").unwrap();
    second.set_undo_stack(&stack).unwrap();
    assert!(!previous.can_undo());
    assert!(!previous.undo().unwrap());

    let group = stack.start_undo_group().unwrap();
    assert_eq!(second.set_undo_stack(&previous), Err(DocumentError::UpdateInProgress));
    first.insert(1, "b").unwrap();
    let temporary = TextDocument::from_text("z");
    temporary.set_undo_stack(&stack).unwrap();
    temporary.insert(1, "1").unwrap();
    let weak = temporary.downgrade();
    drop(temporary);
    assert!(weak.upgrade().is_none());
    group.finish().unwrap();
    assert!(second.undo().unwrap());
    assert_eq!(first.text(), "a");
    assert_eq!(second.text(), "xy");
    assert!(first.can_redo());
    second.set_undo_stack(&previous).unwrap();
    assert!(!stack.can_redo());
    assert!(!stack.redo().unwrap());
    second.insert(2, "z").unwrap();
    assert!(previous.undo().unwrap());
    assert_eq!(second.text(), "xy");
}

#[test]
fn undo_groups_and_custom_operations_do_not_open_document_updates() {
    let document = TextDocument::from_text("a");
    let stack = document.undo_stack();
    let updates = Rc::new(RefCell::new(Vec::new()));
    let log = updates.clone();
    let _started = document.update_started(move || log.borrow_mut().push("start"));
    let log = updates.clone();
    let _changed = document.text_changed(move || log.borrow_mut().push("text"));
    let log = updates.clone();
    let _finished = document.update_finished(move || log.borrow_mut().push("finish"));
    let properties = Rc::new(RefCell::new(Vec::new()));
    let log = properties.clone();
    let _properties = document.property_changed(move |name| {
        if matches!(name, "CanUndo" | "IsModified") {
            log.borrow_mut().push(name.to_owned());
        }
    });
    let value = Rc::new(Cell::new(0));
    let action_log = Rc::new(RefCell::new(Vec::new()));

    let group = stack.start_undo_group().unwrap();
    assert!(!document.is_in_update());
    assert!(updates.borrow().is_empty());
    document.insert(1, "b").unwrap();
    assert!(!document.is_in_update());
    document.insert(2, "c").unwrap();
    assert_eq!(stack.undo(), Err(DocumentError::UpdateInProgress));
    assert_eq!(stack.clear_all(), Err(DocumentError::UpdateInProgress));
    assert_eq!(*updates.borrow(), ["start", "text", "finish", "start", "text", "finish"]);
    assert!(properties.borrow().is_empty());
    group.finish().unwrap();
    assert_eq!(stack.undo_count(), 1);
    assert_eq!(updates.borrow().len(), 6);
    assert_eq!(*properties.borrow(), ["CanUndo", "IsModified"]);

    updates.borrow_mut().clear();
    assert!(stack.undo().unwrap());
    assert_eq!(document.text(), "a");
    assert_eq!(*updates.borrow(), ["start", "text", "finish"]);
    updates.borrow_mut().clear();

    stack.push(action(&value, 0, 1, &action_log)).unwrap();
    assert!(stack.undo().unwrap());
    assert!(stack.redo().unwrap());
    assert_eq!(*action_log.borrow(), ["undo 0", "redo 1"]);
    assert!(updates.borrow().is_empty());
}

#[test]
fn group_descriptors_follow_top_level_groups_and_playback() {
    let document = TextDocument::from_text("a");
    let stack = document.undo_stack();
    let descriptor = || stack.last_group_descriptor::<&'static str>().as_deref().copied();
    let label = Rc::new("initial");
    let group = stack.start_undo_group_with_descriptor(label.clone()).unwrap();
    assert!(Rc::ptr_eq(&stack.last_group_descriptor().unwrap(), &label));
    assert!(stack.last_group_descriptor::<usize>().is_none());
    let nested = stack.start_continued_undo_group_with_descriptor(Rc::new("ignored")).unwrap();
    document.insert(1, "b").unwrap();
    nested.finish().unwrap();
    assert_eq!(descriptor(), Some("initial"));
    group.finish().unwrap();
    assert_eq!(descriptor(), Some("initial"));
    assert_eq!(stack.undo_count(), 1);

    let group = stack.start_continued_undo_group_with_descriptor(Rc::new("continued")).unwrap();
    document.insert(2, "c").unwrap();
    group.finish().unwrap();
    assert_eq!(descriptor(), Some("continued"));
    assert_eq!(stack.undo_count(), 1);
    document.set_is_read_only(true);
    assert_eq!(stack.undo(), Err(DocumentError::ReadOnly));
    assert_eq!(descriptor(), Some("continued"));
    document.set_is_read_only(false);
    assert!(stack.undo().unwrap());
    assert_eq!(document.text(), "a");
    assert_eq!(descriptor(), None);
    assert!(stack.redo().unwrap());
    assert_eq!(document.text(), "abc");
    assert_eq!(descriptor(), None);

    let empty = stack.start_undo_group_with_descriptor(Rc::new("empty with previous")).unwrap();
    empty.finish().unwrap();
    assert_eq!(descriptor(), Some("empty with previous"));
    stack.clear_all().unwrap();
    assert_eq!(descriptor(), None);
    let empty = stack.start_undo_group_with_descriptor(Rc::new("empty without previous")).unwrap();
    empty.finish().unwrap();
    stack.clear_all().unwrap();
    assert!(!stack.undo().unwrap());
    assert_eq!(descriptor(), Some("empty without previous"));

    let group = stack.start_undo_group_with_descriptor(Rc::new("limited")).unwrap();
    document.insert(3, "d").unwrap();
    group.finish().unwrap();
    stack.set_size_limit(0);
    assert!(!stack.can_undo());
    assert_eq!(descriptor(), Some("limited"));
    document.insert(4, "e").unwrap();
    assert_eq!(descriptor(), None);
}

#[test]
fn descriptor_replacement_and_playback_drop_payloads_outside_document_borrows() {
    struct ReentrantDrop(TextDocument, Rc<Cell<usize>>);
    impl Drop for ReentrantDrop {
        fn drop(&mut self) {
            let _ = self.0.text();
            self.1.set(self.1.get() + 1);
        }
    }
    let stack = UndoStack::new();
    let document = stack.document();
    let dropped = Rc::new(Cell::new(0));
    let start = || {
        let descriptor = Rc::new(ReentrantDrop(document.clone(), dropped.clone()));
        stack.start_undo_group_with_descriptor(descriptor).unwrap()
    };
    let outer = start();
    start().finish().unwrap();
    assert_eq!(dropped.get(), 1);
    document.insert(0, "a").unwrap();
    outer.finish().unwrap();
    start().finish().unwrap();
    assert_eq!(dropped.get(), 2);
    assert!(stack.undo().unwrap());
    assert_eq!(dropped.get(), 3);
    let group = start();
    document.insert(0, "b").unwrap();
    group.finish().unwrap();
    stack.clear_all().unwrap();
    assert_eq!(dropped.get(), 4);
}

#[test]
fn context_callbacks_can_read_history_and_playback_rejects_new_document_writes() {
    struct ContextAction {
        calls: Rc<Cell<usize>>,
    }
    impl IUndoableOperationWithContext for ContextAction {
        fn undo(&self, stack: &UndoStack) {
            assert!(!stack.accept_changes());
            assert!(stack.can_redo());
            assert_eq!(
                stack.document().insert(0, "forbidden"),
                Err(DocumentError::ReentrantChange)
            );
            assert!(!stack
                .push_with_context(Rc::new(Self {
                    calls: self.calls.clone()
                }))
                .unwrap());
            self.calls.set(self.calls.get() + 1);
        }
        fn redo(&self, stack: &UndoStack) {
            assert!(!stack.accept_changes());
            assert!(stack.can_undo());
            self.calls.set(self.calls.get() + 1);
        }
    }
    let stack = UndoStack::new();
    let calls = Rc::new(Cell::new(0));
    stack
        .push_with_context(Rc::new(ContextAction {
            calls: calls.clone(),
        }))
        .unwrap();
    stack.undo().unwrap();
    stack.redo().unwrap();
    assert_eq!(calls.get(), 2);
    assert!(stack.accept_changes());
    assert_eq!(stack.undo_count(), 1);
}

#[test]
fn group_limits_and_clear_release_payloads_outside_document_borrows() {
    struct ReentrantDrop {
        document: TextDocument,
        dropped: Rc<Cell<usize>>,
    }
    impl IUndoableOperation for ReentrantDrop {
        fn undo(&self) {}
        fn redo(&self) {}
    }
    impl Drop for ReentrantDrop {
        fn drop(&mut self) {
            let _ = self.document.text();
            self.document.set_is_read_only(false);
            self.dropped.set(self.dropped.get() + 1);
        }
    }
    let stack = UndoStack::new();
    let dropped = Rc::new(Cell::new(0));
    let push = || {
        stack
            .push(Rc::new(ReentrantDrop {
                document: stack.document(),
                dropped: dropped.clone(),
            }))
            .unwrap();
    };
    let group = stack.start_undo_group().unwrap();
    push();
    push();
    group.finish().unwrap();
    push();
    assert_eq!(stack.undo_count(), 2);
    stack.set_size_limit(1);
    assert_eq!(dropped.get(), 2);
    stack.undo().unwrap();
    assert_eq!(stack.redo_count(), 1);
    stack.clear_redo_stack().unwrap();
    assert_eq!(dropped.get(), 3);
    push();
    stack.clear_all().unwrap();
    assert_eq!(dropped.get(), 4);
}

#[test]
fn read_only_replay_rejection_and_zero_limit_preserve_history_contracts() {
    let stack = UndoStack::new();
    let value = Rc::new(Cell::new(0));
    let events = Rc::new(RefCell::new(Vec::new()));
    stack.push(action(&value, 0, 1, &events)).unwrap();
    stack.document().set_is_read_only(true);
    assert_eq!(stack.undo(), Err(DocumentError::ReadOnly));
    assert_eq!(stack.undo_count(), 1);
    assert_eq!(value.get(), 1);
    stack.document().set_is_read_only(false);
    stack.set_size_limit(0);
    assert!(!stack.push(action(&value, 1, 2, &events)).unwrap());
    assert!(!stack.can_undo());
    stack.set_size_limit(1);
    let group = stack.start_continued_undo_group().unwrap();
    stack.push_optional(action(&value, 2, 3, &events)).unwrap();
    group.finish().unwrap();
    assert!(!stack.can_undo());
}

#[test]
fn panicking_custom_callback_releases_playback_and_update_guards() {
    struct PanicAction;
    impl IUndoableOperation for PanicAction {
        fn undo(&self) {
            panic!("client callback")
        }
        fn redo(&self) {}
    }
    let stack = UndoStack::new();
    stack.push(Rc::new(PanicAction)).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| stack.undo()));
    assert!(result.is_err());
    assert!(stack.accept_changes());
    assert!(!stack.document().is_in_update());
    assert_eq!(stack.redo_count(), 1);
    stack.clear_all().unwrap();
    stack.document().insert(0, "healthy").unwrap();
    assert_eq!(stack.document().text(), "healthy");
}
