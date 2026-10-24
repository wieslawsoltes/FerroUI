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
    document.changed(move |e| {
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
