use super::delegate_command::DelegateCommand;
use super::*;
use crate::BoxedValue;
use ferroui_reactiveui::ICommand;
use ferroui_system::IDisposable;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct Scope(Cell<usize>);
impl IDisposable for Scope {
    fn dispose(&self) {
        self.0.set(self.0.get() + 1);
    }
}

#[test]
fn confirmation_metadata_and_commands_dispatch_exact_request_without_implicit_completion() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let seen = calls.clone();
    let request = ConfirmationRequest::new(
        "title".into(),
        "message".into(),
        "yes".into(),
        "no".into(),
        Rc::new(move |r, result| seen.borrow_mut().push((Rc::downgrade(&r), result))),
    );
    assert_eq!(
        (
            request.title(),
            request.message(),
            request.confirm_text(),
            request.cancel_text()
        ),
        ("title", "message", "yes", "no")
    );
    assert!(Rc::ptr_eq(&request.confirm(), &request.confirm()));
    assert!(Rc::ptr_eq(&request.cancel(), &request.cancel()));
    assert!(request.confirm().can_execute(None));
    assert!(request.cancel().can_execute(Some(&42)));
    request.confirm().execute(Some(&"ignored"));
    request.cancel().execute(None);
    assert_eq!(calls.borrow().len(), 2);
    assert!(Rc::ptr_eq(
        &calls.borrow()[0].0.upgrade().unwrap(),
        &request
    ));
    assert!(calls.borrow()[0].1);
    assert!(!calls.borrow()[1].1);
    assert!(!request.task().is_completed());
}

#[test]
fn confirmation_task_is_shared_first_completion_wins_and_completion_can_reenter() {
    let request = ConfirmationRequest::new(
        "".into(),
        "".into(),
        "".into(),
        "".into(),
        Rc::new(|request, result| request.complete(result)),
    );
    let task = request.task();
    assert_eq!(task.id(), request.task().id());
    let seen = Rc::new(Cell::new(0));
    let count = seen.clone();
    let weak = Rc::downgrade(&request);
    task.continue_with(move |task| {
        assert!(task.result().unwrap());
        count.set(count.get() + 1);
        weak.upgrade().unwrap().complete(false);
    });
    request.confirm().execute(None);
    request.cancel().execute(None);
    request.complete(false);
    assert!(task.result().unwrap());
    assert_eq!(seen.get(), 1);
}

#[test]
fn global_handles_are_shared_replaced_and_released_without_implicit_disposal() {
    let request = ConfirmationRequest::new(
        "t".into(),
        "m".into(),
        "c".into(),
        "x".into(),
        Rc::new(|_, _| {}),
    );
    assert!(request.global_handle().is_none());
    let first = Rc::new(Scope::default());
    let second = Rc::new(Scope::default());
    request.set_global_handle(Some(first.clone()));
    assert!(Rc::ptr_eq(
        &request.global_handle().unwrap(),
        &(first.clone() as Rc<dyn IDisposable>)
    ));
    request.set_global_handle(Some(second.clone()));
    request.complete(true);
    request.set_global_handle(None);
    assert_eq!(first.0.get(), 0);
    assert_eq!(second.0.get(), 0);
    let dialog = DialogRequest::new(Rc::new("content"), None, Rc::new(|_, _| {}));
    dialog.set_global_handle(Some(first.clone()));
    assert!(dialog.global_handle().is_some());
    dialog.set_global_handle(Some(second.clone()));
    dialog.complete(None);
    drop(dialog);
    assert_eq!(first.0.get(), 0);
    assert_eq!(second.0.get(), 0);
}

#[test]
fn dialog_close_keeps_nullable_result_content_and_callback_identities() {
    let content: BoxedValue = Rc::new(vec![1, 2, 3]);
    let results = Rc::new(RefCell::new(Vec::new()));
    let seen = results.clone();
    let request = DialogRequest::new(
        content.clone(),
        Some("".into()),
        Rc::new(move |r, result| seen.borrow_mut().push((Rc::downgrade(&r), result))),
    );
    assert!(Rc::ptr_eq(&request.content(), &content));
    assert_eq!(request.title().as_deref(), Some(""));
    assert!(Rc::ptr_eq(&request.close(), &request.close()));
    assert!(request.close().can_execute(None));
    let result: BoxedValue = Rc::new("result".to_owned());
    request.close().execute(Some(&result));
    request.close().execute(None);
    assert!(Rc::ptr_eq(
        &results.borrow()[0].0.upgrade().unwrap(),
        &request
    ));
    assert!(Rc::ptr_eq(results.borrow()[0].1.as_ref().unwrap(), &result));
    assert!(results.borrow()[1].1.is_none());
    assert!(!request.task().is_completed());
    let task = request.task();
    request.complete(Some(result.clone()));
    request.complete(None);
    assert_eq!(task.id(), request.task().id());
    assert!(Rc::ptr_eq(&task.result().unwrap().unwrap(), &result));
    let other = DialogRequest::new(content, None, Rc::new(|_, _| {}));
    assert!(other.title().is_none());
    other.complete(None);
    other.complete(Some(result));
    assert!(other.task().is_completed_successfully());
    assert!(other.task().result().unwrap().is_none());
}

#[test]
fn request_owned_commands_use_native_weak_callbacks_without_internal_cycles() {
    let request = ConfirmationRequest::new(
        "".into(),
        "".into(),
        "".into(),
        "".into(),
        Rc::new(|_, _| {}),
    );
    let weak = Rc::downgrade(&request);
    drop(request);
    assert!(weak.upgrade().is_none());
    let content: BoxedValue = Rc::new(());
    let content_weak = Rc::downgrade(&content);
    let request = DialogRequest::new(content, None, Rc::new(|_, _| {}));
    let weak = Rc::downgrade(&request);
    drop(request);
    assert!(weak.upgrade().is_none());
    assert!(content_weak.upgrade().is_none());
}

#[test]
fn delegate_command_preserves_predicate_execution_and_explicit_event_subscription_semantics() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    let command = DelegateCommand::new(
        Rc::new(move |p| {
            log.borrow_mut()
                .push(p.and_then(|p| p.downcast_ref::<i32>()).copied())
        }),
        Some(Rc::new(|p| {
            p.and_then(|p| p.downcast_ref::<i32>())
                .is_some_and(|x| *x > 0)
        })),
    );
    assert!(!command.can_execute(None));
    assert!(command.can_execute(Some(&1)));
    assert!(!command.can_execute(Some(&-1)));
    command.execute(Some(&-1));
    command.execute(None);
    assert_eq!(&*calls.borrow(), &[Some(-1), None]);
    let changes = Rc::new(Cell::new(0));
    let seen = changes.clone();
    let subscription = command.can_execute_changed(Rc::new(move || seen.set(seen.get() + 1)));
    command.raise_can_execute_changed();
    assert_eq!(changes.get(), 1);
    subscription.dispose();
    subscription.dispose();
    command.raise_can_execute_changed();
    assert_eq!(changes.get(), 1);
    assert!(DelegateCommand::new(Rc::new(|_| {}), None).can_execute(Some(&"anything")));
}

#[test]
fn delegate_change_handlers_can_unsubscribe_during_notification() {
    let command = DelegateCommand::new(Rc::new(|_| {}), None);
    let subscription = Rc::new(RefCell::new(None::<Rc<dyn IDisposable>>));
    let handle = subscription.clone();
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    subscription.replace(Some(command.can_execute_changed(Rc::new(move || {
        seen.set(seen.get() + 1);
        handle.borrow().as_ref().unwrap().dispose();
    }))));
    command.raise_can_execute_changed();
    command.raise_can_execute_changed();
    assert_eq!(calls.get(), 1);
}

#[test]
fn dialog_content_and_reload_contracts_use_actual_shared_tasks_and_nullable_results() {
    struct Content(RefCell<Option<DialogContentCloseAction>>);
    impl IDockDialogContent for Content {
        fn set_close_action(&self, action: DialogContentCloseAction) {
            self.0.replace(Some(action));
        }
    }
    impl IReloadable for Content {
        fn reload_async(&self) -> ferroui_system::threading::tasks::Task<()> {
            ferroui_system::threading::tasks::Task::from_result(())
        }
    }
    let content = Content(RefCell::new(None));
    let value: BoxedValue = Rc::new("result");
    let seen = Rc::new(RefCell::new(None));
    let captured = seen.clone();
    content.set_close_action(Rc::new(move |result| {
        captured.replace(result);
    }));
    content.0.borrow().as_ref().unwrap()(Some(value.clone()));
    assert!(Rc::ptr_eq(seen.borrow().as_ref().unwrap(), &value));
    content.0.borrow().as_ref().unwrap()(None);
    assert!(seen.borrow().is_none());
    assert!(content.reload_async().is_completed_successfully());
}

#[test]
fn typed_dialog_results_preserve_value_defaults_nullable_values_and_reference_identity() {
    assert_eq!(i32::from_dialog_result(None), 0);
    assert_eq!(i32::from_dialog_result(Some(Rc::new(7_i64))), 0);
    assert_eq!(i32::from_dialog_result(Some(Rc::new(7_i32))), 7);
    assert!(!bool::from_dialog_result(None));
    assert!(Option::<i32>::from_dialog_result(None).is_none());
    assert_eq!(
        Option::<i32>::from_dialog_result(Some(Rc::new(7_i32))),
        Some(7)
    );
    let text = Rc::new("same reference".to_owned());
    let boxed: BoxedValue = text.clone();
    assert!(Rc::ptr_eq(
        &Option::<Rc<String>>::from_dialog_result(Some(boxed.clone())).unwrap(),
        &text
    ));
    assert!(Rc::ptr_eq(
        &Option::<BoxedValue>::from_dialog_result(Some(boxed)).unwrap(),
        &(text.clone() as BoxedValue)
    ));
    let wrapped: BoxedValue = Rc::new(text.clone());
    assert!(Rc::ptr_eq(
        &Option::<Rc<String>>::from_dialog_result(Some(wrapped)).unwrap(),
        &text
    ));
    assert!(Option::<Rc<Vec<i32>>>::from_dialog_result(Some(text)).is_none());
}
