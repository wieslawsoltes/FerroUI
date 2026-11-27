//! Runs Core command futures on the selected native dispatcher, not a port runtime.
use ferroribbon_core::commands::{RibbonAsyncCommandFuture, RibbonCommandExecutor};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use std::{rc::Rc, sync::Arc};

/// Capture the native dispatcher weakly at the composition root. Each submission
/// uses the current native priority, after the command's synchronous factory.
/// First polling is eager; subsequent wakes use that dispatcher's local executor.
/// Unhandled command errors are posted through its normal exception routing.
pub fn dispatcher_command_executor() -> Rc<RibbonCommandExecutor> {
    let dispatcher = Dispatcher::ui_thread();
    dispatcher.verify_access();
    let dispatcher = Arc::downgrade(&dispatcher);
    Rc::new(move |future: RibbonAsyncCommandFuture| {
        let dispatcher = dispatcher
            .upgrade()
            .expect("command dispatcher has expired");
        dispatcher.verify_access();
        let priority = Dispatcher::current_priority().unwrap_or(DispatcherPriority::DEFAULT);
        let weak_dispatcher = Arc::downgrade(&dispatcher);
        drop(dispatcher.start_local_with_priority(
            async move {
                if let Err(error) = future.await {
                    let dispatcher = weak_dispatcher
                        .upgrade()
                        .expect("command dispatcher has expired");
                    dispatcher.post_local(
                        move || panic!("Unhandled RibbonAsyncCommand error: {error}"),
                        priority,
                    );
                }
            },
            priority,
        ));
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        controls::RibbonButton,
        runtime_sample::{self as sample, Mounted},
    };
    use ferroribbon_core::commands::{RibbonAsyncCommand, RibbonAsyncCommandFuture};
    use futures_channel::oneshot;
    use std::{
        cell::{Cell, RefCell},
        task::{Context, Poll, Waker},
    };

    #[test]
    fn each_invocation_uses_its_native_priority_for_initial_and_resumed_polls() {
        crate::group_tests::run(|| {
            let (sender, receiver) = oneshot::channel::<()>();
            let receiver = Rc::new(RefCell::new(Some(receiver)));
            let seen = Rc::new(RefCell::new(Vec::new()));
            let output = seen.clone();
            let command = RibbonAsyncCommand::new(dispatcher_command_executor(), move || {
                let receiver = receiver.borrow_mut().take().unwrap();
                let output = output.clone();
                async move {
                    output
                        .borrow_mut()
                        .push(Dispatcher::current_priority().unwrap());
                    receiver.await.unwrap();
                    output
                        .borrow_mut()
                        .push(Dispatcher::current_priority().unwrap());
                    Ok(())
                }
            });
            let dispatcher = Dispatcher::ui_thread();
            dispatcher.post_local(
                move || drop(command.execute_async(None)),
                DispatcherPriority::INPUT,
            );
            dispatcher.run_jobs(None);
            sender.send(()).unwrap();
            dispatcher.run_jobs(None);
            assert_eq!(
                *seen.borrow(),
                [DispatcherPriority::INPUT, DispatcherPriority::INPUT]
            );
        });
    }

    #[test]
    fn dispatcher_starts_eagerly_and_updates_a_real_command_button_after_worker_completion() {
        crate::group_tests::run(|| {
            let m = Mounted::new(1400.);
            let button = sample::item::<RibbonButton>(&m.ribbon, "copy");
            let (sender, receiver) = oneshot::channel::<()>();
            let receiver = Rc::new(RefCell::new(Some(receiver)));
            let count = Rc::new(Cell::new(0));
            let calls = count.clone();
            let ui_thread = std::thread::current().id();
            let finished = Rc::new(Cell::new(false));
            let observed = finished.clone();
            let command = RibbonAsyncCommand::new(dispatcher_command_executor(), move || {
                let receiver = receiver.borrow_mut().take().unwrap();
                let observed = observed.clone();
                calls.set(calls.get() + 1);
                async move {
                    assert_eq!(std::thread::current().id(), ui_thread);
                    receiver.await.unwrap();
                    assert_eq!(std::thread::current().id(), ui_thread);
                    observed.set(true);
                    Ok(())
                }
            });
            button.set_command(Some(crate::commands::adapt_core_command(command.clone())));
            assert!(button.is_enabled());
            button.perform_click();
            assert_eq!(count.get(), 1);
            assert!(command.is_running());
            assert!(!button.is_enabled());
            button.perform_click();
            assert_eq!(count.get(), 1);
            std::thread::spawn(move || sender.send(()).unwrap())
                .join()
                .unwrap();
            assert!(!finished.get());
            m.settle();
            assert!(finished.get());
            assert!(!command.is_running());
            assert!(button.is_enabled());
        });
    }

    #[test]
    fn dropped_observation_and_command_handles_keep_only_the_pending_invocation_alive() {
        crate::group_tests::run(|| {
            let (sender, receiver) = oneshot::channel::<()>();
            let receiver = Rc::new(RefCell::new(Some(receiver)));
            let command = RibbonAsyncCommand::new(dispatcher_command_executor(), move || {
                let receiver = receiver.borrow_mut().take().unwrap();
                async move {
                    receiver.await.unwrap();
                    Ok(())
                }
            });
            let weak = Rc::downgrade(&command);
            let changes = Rc::new(Cell::new(0));
            let observed = changes.clone();
            command.can_execute_changed(Rc::new(move || observed.set(observed.get() + 1)));
            let mut result: RibbonAsyncCommandFuture = command.execute_async(None);
            assert!(matches!(
                result
                    .as_mut()
                    .poll(&mut Context::from_waker(Waker::noop())),
                Poll::Pending
            ));
            drop(result);
            drop(command);
            assert!(weak.upgrade().is_some());
            std::thread::spawn(move || sender.send(()).unwrap())
                .join()
                .unwrap();
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(changes.get(), 2);
            assert!(weak.upgrade().is_none());
        });
    }

    #[test]
    fn unhandled_failures_use_native_dispatcher_error_routing_after_execute_returns() {
        crate::group_tests::run(|| {
            for handled in [false, true] {
                let dispatcher = Dispatcher::ui_thread();
                let errors = Rc::new(RefCell::new(Vec::new()));
                let log = errors.clone();
                let subscription = dispatcher.unhandled_exception(move |args| {
                    log.borrow_mut()
                        .push(args.exception_message().unwrap().to_owned());
                    args.set_handled(true);
                });
                let command = RibbonAsyncCommand::new(dispatcher_command_executor(), || async {
                    Err(Rc::new(std::io::Error::other("body")) as ferroui_system::ObservableError)
                });
                command.failed(Rc::new(move |args| args.set_handled(handled)));
                crate::commands::adapt_core_command(command.clone()).execute(None);
                assert!(!command.is_running());
                assert!(errors.borrow().is_empty());
                dispatcher.run_jobs(None);
                assert_eq!(
                    *errors.borrow(),
                    if handled {
                        vec![]
                    } else {
                        vec!["Unhandled RibbonAsyncCommand error: body".to_owned()]
                    }
                );
                subscription.dispose();
            }
        });
    }
}
