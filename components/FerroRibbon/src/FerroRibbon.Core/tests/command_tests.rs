use ferroribbon_core::commands::*;
use ferroui_system::{IDisposable, ObservableError};
use futures_util::FutureExt;
#[path = "support/command_executor.rs"]
pub mod executor;
use executor::{Executor, canceled, channel};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Default)]
struct Trace(Rc<RefCell<Vec<String>>>);
impl Trace {
    fn add(&self, text: impl Into<String>) {
        self.0.borrow_mut().push(text.into());
    }
    fn check(&self, name: &str) {
        let reference: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/commands.json")).unwrap();
        assert_eq!(
            serde_json::json!(*self.0.borrow()),
            reference[name],
            "{name}"
        );
    }
}
fn error(message: &'static str) -> ObservableError {
    Rc::new(std::io::Error::other(message))
}
fn parameter(p: Option<&dyn Any>) -> &str {
    p.and_then(|v| v.downcast_ref::<String>())
        .map_or("<null>", String::as_str)
}

#[test]
fn relay_source_trace_with_mutation_during_notification() {
    let trace = Trace::default();
    let enabled = Rc::new(Cell::new(false));
    let t = trace.clone();
    let p = trace.clone();
    let e = enabled.clone();
    let command = RibbonRelayCommand::new_with_parameter(
        move |v| t.add(format!("execute:{}", parameter(v))),
        Some(Rc::new(move |v| {
            p.add(format!("can:{}", parameter(v)));
            e.get()
        })),
    );
    let interface: Rc<dyn ICommand> = command.clone();
    interface.execute(Some(&"a".to_owned()));
    enabled.set(true);
    interface.execute(Some(&"b".to_owned()));
    let removal = Rc::new(RefCell::new(None::<Rc<dyn IDisposable>>));
    let r = removal.clone();
    let t = trace.clone();
    command.can_execute_changed(Rc::new(move || {
        t.add("first");
        if let Some(subscription) = r.borrow().as_ref() {
            subscription.dispose();
        }
    }));
    let t = trace.clone();
    *removal.borrow_mut() = Some(command.can_execute_changed(Rc::new(move || t.add("second"))));
    command.notify_can_execute_changed();
    command.notify_can_execute_changed();
    trace.check("relay");
}

#[test]
fn typed_parameters_match_default_conversion() {
    let trace = Trace::default();
    let t = trace.clone();
    let strings = RibbonRelayCommandOf::<Option<String>>::new(
        move |v| t.add(format!("string:{}", v.as_deref().unwrap_or("<null>"))),
        None,
    );
    let interface: Rc<dyn ICommand> = strings.clone();
    interface.execute(Some(&"hello".to_owned()));
    interface.execute(Some(&12_i32));
    interface.execute(None);
    let t = trace.clone();
    let numbers = RibbonRelayCommandOf::<i32>::new(move |v| t.add(format!("int:{v}")), None);
    numbers.execute(Some(&12_i32));
    numbers.execute(Some(&"wrong".to_owned()));
    numbers.execute(None);
    trace.check("typed");
}

#[test]
fn async_success_suppresses_reentry_before_predicate() {
    let mut executor = Executor::new();
    let trace = Trace::default();
    let t = trace.clone();
    let p = trace.clone();
    let (completion, c) = channel();
    let command = RibbonAsyncCommand::new_with_parameter(
        executor.spawner(),
        move |v| {
            t.add(format!("execute:{}", parameter(v)));
            c.clone()
        },
        Some(Rc::new(move |_| {
            p.add("can");
            true
        })),
    );
    let weak = Rc::downgrade(&command);
    let t = trace.clone();
    command.can_execute_changed(Rc::new(move || {
        t.add(format!("changed:{}", weak.upgrade().unwrap().is_running()))
    }));
    let mut first = command.execute_async(Some(&"x".to_owned()));
    assert!(executor.poll(&mut first).is_none());
    trace.add(format!("running:{}", command.is_running()));
    trace.add(format!("can-running:{}", command.can_execute(None)));
    assert!(command.execute_async(None).now_or_never().unwrap().is_ok());
    trace.add("second-completed");
    completion.send(Ok(())).unwrap();
    executor.pump();
    assert!(executor.poll(&mut first).unwrap().is_ok());
    trace.add(format!("running:{}", command.is_running()));
    trace.check("async_success");
}

#[test]
fn failure_handlers_share_args_and_original_error() {
    let mut executor = Executor::new();
    for handled in [false, true] {
        let trace = Trace::default();
        let original = error("boom");
        let e = original.clone();
        let command = RibbonAsyncCommand::new(executor.spawner(), move || {
            std::future::ready(Err(e.clone()))
        });
        let weak = Rc::downgrade(&command);
        let t = trace.clone();
        command.can_execute_changed(Rc::new(move || {
            t.add(format!("changed:{}", weak.upgrade().unwrap().is_running()))
        }));
        let args_seen = Rc::new(RefCell::new(None::<Rc<RibbonCommandErrorEventArgs>>));
        let seen = args_seen.clone();
        let e = original.clone();
        let weak = Rc::downgrade(&command);
        let t = trace.clone();
        command.failed(Rc::new(move |args| {
            t.add(format!(
                "failed:{}:{}",
                Rc::ptr_eq(&args.exception(), &e),
                weak.upgrade().unwrap().is_running()
            ));
            args.set_handled(handled);
            *seen.borrow_mut() = Some(args.clone());
        }));
        let t = trace.clone();
        command.failed(Rc::new(move |args| {
            t.add(format!(
                "same-args:{}:{}",
                Rc::ptr_eq(args, args_seen.borrow().as_ref().unwrap()),
                args.handled()
            ))
        }));
        match executor.wait(command.execute_async(None)) {
            Ok(()) => trace.add("completed"),
            Err(e) => trace.add(format!("thrown:{e}:{}", Rc::ptr_eq(&e, &original))),
        }
        trace.add(format!("running:{}", command.is_running()));
        trace.check(&format!("async_error_{handled}"));
    }
}

#[test]
fn all_exception_boundaries_match_csharp_try_finally_order() {
    let mut executor = Executor::new();
    for stage in ["predicate", "start", "factory", "failed", "finish"] {
        let trace = Trace::default();
        let t = trace.clone();
        let p = trace.clone();
        let command = RibbonAsyncCommand::new_with_parameter(
            executor.spawner(),
            move |_| {
                t.add("execute");
                if stage == "factory" {
                    panic!("factory");
                }
                std::future::ready(Err(error("body")))
            },
            Some(Rc::new(move |_| {
                p.add("can");
                if stage == "predicate" {
                    panic!("predicate");
                }
                true
            })),
        );
        let weak = Rc::downgrade(&command);
        let t = trace.clone();
        command.can_execute_changed(Rc::new(move || {
            let running = weak.upgrade().unwrap().is_running();
            t.add(format!("changed:{running}"));
            if running && stage == "start" {
                panic!("start");
            }
            if !running && stage == "finish" {
                panic!("finish");
            }
        }));
        let t = trace.clone();
        command.failed(Rc::new(move |args| {
            t.add(format!("failed:{}", args.exception()));
            if stage == "failed" {
                panic!("failed");
            }
        }));
        let result = executor.wait(command.execute_async(None));
        trace.add(format!("thrown:{}", result.unwrap_err()));
        trace.add(format!("running:{}", command.is_running()));
        trace.check(&format!("throws_{stage}"));
    }
}

#[test]
fn cancellation_can_be_handled_and_start_notification_can_reenter() {
    let mut executor = Executor::new();
    let trace = Trace::default();
    let t = trace.clone();
    let command =
        RibbonAsyncCommand::new(executor.spawner(), || std::future::ready(Err(canceled())));
    command.failed(Rc::new(move |args| {
        t.add(format!(
            "canceled:{}",
            args.exception()
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::Interrupted)
        ));
        args.set_handled(true);
    }));
    assert!(executor.wait(command.execute_async(None)).is_ok());
    trace.add(format!("running:{}", command.is_running()));
    trace.check("async_cancel_handled");
    let trace = Trace::default();
    let t = trace.clone();
    let command = RibbonAsyncCommand::new(executor.spawner(), move || {
        t.add("execute");
        std::future::ready(Ok(()))
    });
    let weak = Rc::downgrade(&command);
    let t = trace.clone();
    command.can_execute_changed(Rc::new(move || {
        let command = weak.upgrade().unwrap();
        t.add(format!("changed:{}", command.is_running()));
        if command.is_running() {
            t.add(format!(
                "nested-completed:{}",
                command.execute_async(None).now_or_never().is_some()
            ));
        }
    }));
    executor.wait(command.execute_async(None)).unwrap();
    trace.check("start_reentrancy");
}

#[test]
fn disposal_releases_handlers_without_owning_command_and_drop_does_not_unsubscribe() {
    let command = RibbonRelayCommand::new(|| {});
    let sentinel = Rc::new(());
    let weak = Rc::downgrade(&sentinel);
    let subscription = command.can_execute_changed(Rc::new(move || {
        let _ = &sentinel;
    }));
    assert!(weak.upgrade().is_some());
    subscription.dispose();
    subscription.dispose();
    assert!(weak.upgrade().is_none());
    let calls = Rc::new(Cell::new(0));
    let c = calls.clone();
    drop(command.can_execute_changed(Rc::new(move || c.set(c.get() + 1))));
    command.notify_can_execute_changed();
    assert_eq!(calls.get(), 1);
    let weak = Rc::downgrade(&command);
    drop(command);
    assert!(weak.upgrade().is_none());
    subscription.dispose();
}

#[test]
fn pending_execution_retains_command_when_observation_future_is_dropped() {
    let mut executor = Executor::new();
    let (completion, c) = channel();
    let command = RibbonAsyncCommand::new(executor.spawner(), move || c.clone());
    let weak = Rc::downgrade(&command);
    let calls = Rc::new(Cell::new(0));
    let n = calls.clone();
    command.can_execute_changed(Rc::new(move || n.set(n.get() + 1)));
    drop(command.execute_async(None));
    drop(command);
    assert!(weak.upgrade().is_some());
    completion.send(Ok(())).unwrap();
    executor.pump();
    assert_eq!(calls.get(), 2);
    assert!(weak.upgrade().is_none());
}

#[test]
fn relay_reentrant_invocation_is_not_suppressed_and_parameter_identity_is_retained() {
    let nested = Rc::new(RefCell::new(None::<std::rc::Weak<RibbonRelayCommand>>));
    let n = nested.clone();
    let count = Rc::new(Cell::new(0));
    let c = count.clone();
    let value = Rc::new(42);
    let v = value.clone();
    let command = RibbonRelayCommand::new_with_parameter(
        move |parameter| {
            assert!(Rc::ptr_eq(
                parameter.unwrap().downcast_ref::<Rc<i32>>().unwrap(),
                &v
            ));
            c.set(c.get() + 1);
            if c.get() == 1 {
                n.borrow()
                    .as_ref()
                    .unwrap()
                    .upgrade()
                    .unwrap()
                    .execute(parameter);
            }
        },
        None,
    );
    *nested.borrow_mut() = Some(Rc::downgrade(&command));
    command.execute(Some(&value));
    assert_eq!(count.get(), 2);
}

#[test]
fn unhandled_cancellation_returns_the_error_for_ready_and_pending_futures() {
    let mut executor = Executor::new();
    for pending in [false, true] {
        let (source, s) = channel();
        let command =
            RibbonAsyncCommand::new(executor.spawner(), move || -> RibbonAsyncCommandFuture {
                if pending {
                    Box::pin(s.clone())
                } else {
                    Box::pin(std::future::ready(Err(canceled())))
                }
            });
        let mut result = command.execute_async(None);
        executor.pump();
        if pending {
            assert!(executor.poll(&mut result).is_none());
            source.send(Err(canceled())).unwrap();
            executor.pump();
        }
        assert_eq!(
            executor
                .wait(result)
                .unwrap_err()
                .downcast_ref::<std::io::Error>()
                .unwrap()
                .kind(),
            std::io::ErrorKind::Interrupted
        );
        assert!(!command.is_running());
    }
}

#[test]
fn native_future_factory_starts_eagerly_preserves_busy_notifications_and_resumes_without_ui() {
    let mut executor = Executor::new();
    use std::{
        future::poll_fn,
        task::{Poll, Waker},
    };
    let trace = Rc::new(RefCell::new(Vec::new()));
    let gate = Rc::new(Cell::new(false));
    let waker: Rc<RefCell<Option<Waker>>> = Rc::new(RefCell::new(None));
    let body_trace = trace.clone();
    let body_gate = gate.clone();
    let body_waker = waker.clone();
    let command = RibbonAsyncCommand::new(executor.spawner(), move || {
        let trace = body_trace.clone();
        let gate = body_gate.clone();
        let waker = body_waker.clone();
        async move {
            trace.borrow_mut().push("start");
            poll_fn(move |cx| {
                if gate.get() {
                    Poll::Ready(())
                } else {
                    waker.replace(Some(cx.waker().clone()));
                    Poll::Pending
                }
            })
            .await;
            trace.borrow_mut().push("end");
            Ok(())
        }
    });
    let observed = trace.clone();
    let weak = Rc::downgrade(&command);
    let subscription = command.can_execute_changed(Rc::new(move || {
        let current = weak.upgrade().unwrap();
        observed
            .borrow_mut()
            .push(if current.is_running() { "busy" } else { "idle" });
    }));
    let mut result = command.execute_async(None);
    assert_eq!(*trace.borrow(), ["busy", "start"]);
    assert!(command.is_running());
    assert!(!command.can_execute(None));
    assert!(executor.poll(&mut result).is_none());
    gate.set(true);
    waker.take().unwrap().wake();
    executor.pump();
    assert_eq!(*trace.borrow(), ["busy", "start", "end", "idle"]);
    assert!(!command.is_running());
    assert!(executor.wait(result).is_ok());
    subscription.dispose();
}

#[test]
fn native_future_parameter_factory_retains_actual_shared_value_and_false_gate_does_not_start() {
    let mut executor = Executor::new();
    let parameter = Rc::new(Cell::new(0));
    let expected = parameter.clone();
    let command = RibbonAsyncCommand::new_with_parameter(
        executor.spawner(),
        move |p| {
            let value = p
                .unwrap()
                .downcast_ref::<Rc<Cell<usize>>>()
                .unwrap()
                .clone();
            assert!(Rc::ptr_eq(&value, &expected));
            async move {
                value.set(value.get() + 1);
                Ok(())
            }
        },
        None,
    );
    assert!(
        executor
            .wait(command.execute_async(Some(&parameter)))
            .is_ok()
    );
    assert_eq!(parameter.get(), 1);
    let calls = Rc::new(Cell::new(0));
    let body = calls.clone();
    let rejected = RibbonAsyncCommand::new_with_can_execute(
        executor.spawner(),
        move || {
            let calls = body.clone();
            async move {
                calls.set(calls.get() + 1);
                Ok(())
            }
        },
        Some(Rc::new(|| false)),
    );
    assert!(executor.wait(rejected.execute_async(None)).is_ok());
    assert_eq!(calls.get(), 0);
    assert!(!rejected.is_running());
}

#[test]
fn executor_shutdown_reports_cancellation_and_releases_pending_command() {
    let executor = Executor::new();
    let trace = Trace::default();
    let t = trace.clone();
    let command = RibbonAsyncCommand::new(executor.spawner(), std::future::pending);
    let weak = Rc::downgrade(&command);
    let observed = weak.clone();
    command.can_execute_changed(Rc::new(move || {
        t.add(format!(
            "changed:{}",
            observed.upgrade().unwrap().is_running()
        ));
    }));
    let t = trace.clone();
    command.failed(Rc::new(move |args| {
        assert_eq!(
            args.exception()
                .downcast_ref::<std::io::Error>()
                .unwrap()
                .kind(),
            std::io::ErrorKind::Interrupted
        );
        t.add("canceled");
    }));
    let result = command.execute_async(None);
    drop(command);
    assert!(weak.upgrade().is_some());
    drop(executor);
    let outcome = result
        .now_or_never()
        .expect("shutdown completes observation");
    assert!(outcome.is_err());
    assert!(weak.upgrade().is_none());
    assert_eq!(
        *trace.0.borrow(),
        ["changed:true", "canceled", "changed:false"]
    );
}

#[test]
fn a_panic_after_await_uses_the_same_failed_finally_path() {
    let mut executor = Executor::new();
    let (sender, gate) = channel();
    let do_panic = Rc::new(Cell::new(true));
    let command = RibbonAsyncCommand::new(executor.spawner(), move || {
        let gate = gate.clone();
        let do_panic = do_panic.clone();
        async move {
            gate.await?;
            if do_panic.get() {
                panic!("after await");
            }
            Ok(())
        }
    });
    let error = Rc::new(RefCell::new(None));
    let observed = error.clone();
    command.failed(Rc::new(move |args| {
        observed.replace(Some(args.exception()));
    }));
    let mut result = command.execute_async(None);
    assert!(executor.poll(&mut result).is_none());
    sender.send(Ok(())).unwrap();
    assert_eq!(
        executor.wait(result).unwrap_err().to_string(),
        "after await"
    );
    assert_eq!(error.borrow().as_ref().unwrap().to_string(), "after await");
    assert!(!command.is_running());
}

#[test]
fn fire_and_forget_errors_are_delivered_to_the_supplied_executor() {
    let mut executor = Executor::new();
    let original = error("unhandled");
    let e = original.clone();
    let command = RibbonAsyncCommand::new(executor.spawner(), move || {
        std::future::ready(Err(e.clone()))
    });
    command.execute(None);
    executor.pump();
    let errors = executor.take_unhandled();
    assert_eq!(errors.len(), 1);
    assert!(Rc::ptr_eq(&errors[0], &original));
    assert!(!command.is_running());
}

#[test]
fn finish_notification_can_start_a_new_run_without_the_old_completion_resetting_it() {
    let mut executor = Executor::new();
    let (sender, gate) = channel();
    let count = Rc::new(Cell::new(0));
    let calls = count.clone();
    let command =
        RibbonAsyncCommand::new(executor.spawner(), move || -> RibbonAsyncCommandFuture {
            calls.set(calls.get() + 1);
            if calls.get() == 1 {
                Box::pin(std::future::ready(Ok(())))
            } else {
                Box::pin(gate.clone())
            }
        });
    let weak = Rc::downgrade(&command);
    let calls = count.clone();
    command.can_execute_changed(Rc::new(move || {
        let command = weak.upgrade().unwrap();
        if !command.is_running() && calls.get() == 1 {
            drop(command.execute_async(None));
        }
    }));
    executor.wait(command.execute_async(None)).unwrap();
    assert_eq!(count.get(), 2);
    assert!(command.is_running());
    sender.send(Ok(())).unwrap();
    executor.pump();
    assert!(!command.is_running());
    assert_eq!(count.get(), 2);
}

#[test]
fn a_ready_callback_finishes_eagerly_without_submitting_to_any_executor() {
    let submitted = Rc::new(Cell::new(0));
    let submissions = submitted.clone();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let command = RibbonAsyncCommand::new(
        Rc::new(move |_work| submissions.set(submissions.get() + 1)),
        move || {
            let count = count.clone();
            async move {
                count.set(count.get() + 1);
                Ok(())
            }
        },
    );
    let events = Rc::new(RefCell::new(Vec::new()));
    let observed = events.clone();
    let weak = Rc::downgrade(&command);
    command.can_execute_changed(Rc::new(move || {
        observed
            .borrow_mut()
            .push(weak.upgrade().unwrap().is_running())
    }));
    let result = command.execute_async(None);
    assert_eq!(calls.get(), 1);
    assert_eq!(submitted.get(), 0);
    assert_eq!(*events.borrow(), [true, false]);
    assert!(!command.is_running());
    assert!(result.now_or_never().unwrap().is_ok());
}
