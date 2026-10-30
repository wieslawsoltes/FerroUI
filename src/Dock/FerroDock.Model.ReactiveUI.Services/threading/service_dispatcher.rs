// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use ferroui_reactive::concurrency::{IScheduler, SchedulerExt};
use ferroui_reactiveui::RxSchedulers;
use ferroui_system::threading::{current_synchronization_context, SynchronizationContext};
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct ServiceDispatcher {
    context: Option<Arc<dyn SynchronizationContext>>,
    scheduler: Rc<dyn IScheduler>,
}

impl ServiceDispatcher {
    pub(crate) fn invoke_async(
        &self,
        action: impl FnOnce() + 'static,
    ) -> super::DispatcherInvocation {
        let completion = ferroui_system::threading::tasks::TaskCompletionSource::new();
        let task = completion.task();
        self.post(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action));
            match outcome {
                Ok(()) => {
                    completion.try_set_result(());
                }
                Err(payload) => {
                    completion.try_set_exception(super::task_action::panic_error(payload));
                }
            }
        });
        super::DispatcherInvocation::new(task)
    }

    pub(crate) fn new(
        context: Option<Arc<dyn SynchronizationContext>>,
        scheduler: Option<Rc<dyn IScheduler>>,
    ) -> Self {
        Self {
            context: context.or_else(current_synchronization_context),
            scheduler: scheduler.unwrap_or_else(RxSchedulers::main_thread_scheduler),
        }
    }

    pub(crate) fn check_access(&self) -> bool {
        self.context
            .as_ref()
            .is_some_and(<dyn SynchronizationContext>::is_current)
    }

    pub(crate) fn post(&self, action: impl FnOnce() + 'static) {
        if let Some(context) = &self.context {
            if self.check_access() {
                action();
            } else {
                context.post_local(Box::new(action));
            }
        } else {
            let _ = self.scheduler.schedule_action(action);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_reactive::concurrency::HistoricalScheduler;
    use ferroui_system::threading::set_synchronization_context;
    use std::cell::Cell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Default)]
    struct InlineContext {
        posts: AtomicUsize,
    }
    impl SynchronizationContext for InlineContext {
        fn post(&self, work: Box<dyn FnOnce() + Send>) {
            work();
        }
        fn send(&self, work: Box<dyn FnOnce() + Send>) {
            work();
        }
        fn post_local(&self, work: Box<dyn FnOnce()>) {
            self.posts.fetch_add(1, Ordering::Relaxed);
            work();
        }
        fn send_local(&self, work: Box<dyn FnOnce()>) {
            work();
        }
    }

    struct ContextScope(Option<Arc<dyn SynchronizationContext>>);
    impl ContextScope {
        fn new(context: Option<Arc<dyn SynchronizationContext>>) -> Self {
            let scope = Self(current_synchronization_context());
            set_synchronization_context(context);
            scope
        }
    }
    impl Drop for ContextScope {
        fn drop(&mut self) {
            set_synchronization_context(self.0.take());
        }
    }

    #[test]
    fn same_context_uses_inline_execution_and_captures_current_by_default() {
        let context = Arc::new(InlineContext::default());
        let _scope = ContextScope::new(Some(context.clone()));
        let dispatcher = ServiceDispatcher::new(None, Some(HistoricalScheduler::historical()));
        assert!(dispatcher.check_access());
        let result = Rc::new(Cell::new(false));
        let captured = result.clone();
        dispatcher.post(move || captured.set(true));
        assert!(result.get());
        assert_eq!(context.posts.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn explicit_context_is_posted_to_when_current_context_is_different() {
        let context = Arc::new(InlineContext::default());
        let _scope = ContextScope::new(Some(Arc::new(InlineContext::default())));
        let dispatcher = ServiceDispatcher::new(
            Some(context.clone()),
            Some(HistoricalScheduler::historical()),
        );
        assert!(!dispatcher.check_access());
        let result = Rc::new(Cell::new(false));
        let captured = result.clone();
        dispatcher.post(move || captured.set(true));
        assert!(result.get());
        assert_eq!(context.posts.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn no_context_uses_the_scheduler_and_ignoring_its_handle_does_not_cancel_work() {
        let _scope = ContextScope::new(None);
        let scheduler = HistoricalScheduler::historical();
        let dispatcher = ServiceDispatcher::new(None, Some(scheduler.clone()));
        assert!(!dispatcher.check_access());
        let result = Rc::new(Cell::new(0));
        for value in 1..=3 {
            let captured = result.clone();
            dispatcher.post(move || captured.set(captured.get() * 10 + value));
        }
        assert_eq!(result.get(), 0);
        scheduler.start();
        assert_eq!(result.get(), 123);
    }

    #[test]
    fn the_default_scheduler_is_captured_at_construction() {
        let _scope = ContextScope::new(None);
        let previous = RxSchedulers::main_thread_scheduler();
        let scheduler = HistoricalScheduler::historical();
        RxSchedulers::set_main_thread_scheduler(scheduler.clone());
        let dispatcher = ServiceDispatcher::new(None, None);
        RxSchedulers::set_main_thread_scheduler(previous);
        let result = Rc::new(Cell::new(false));
        let captured = result.clone();
        dispatcher.post(move || captured.set(true));
        assert!(!result.get());
        scheduler.start();
        assert!(result.get());
    }
}

#[cfg(test)]
mod invocation_tests {
    use super::*;
    use crate::test_environment::Environment;
    use ferroui_reactive::concurrency::HistoricalScheduler;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::mpsc;
    use std::task::{Context, Poll, Wake, Waker};
    use std::time::Duration;
    struct RecordWake(mpsc::Sender<std::thread::ThreadId>);
    impl Wake for RecordWake {
        fn wake(self: Arc<Self>) {
            self.0.send(std::thread::current().id()).unwrap();
        }
    }
    #[test]
    fn pending_invocation_never_wakes_inline_and_repeated_polls_keep_one_registration() {
        let _environment = Environment::new();
        let scheduler = HistoricalScheduler::historical();
        let dispatcher = ServiceDispatcher::new(None, Some(scheduler.clone()));
        let mut invocation = dispatcher.invoke_async(|| {});
        let (sender, receiver) = mpsc::channel();
        let waker = Waker::from(Arc::new(RecordWake(sender)));
        let mut cx = Context::from_waker(&waker);
        assert!(Pin::new(&mut invocation).poll(&mut cx).is_pending());
        assert!(Pin::new(&mut invocation).poll(&mut cx).is_pending());
        scheduler.start();
        let wake_thread = receiver.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_ne!(wake_thread, std::thread::current().id());
        assert!(receiver.try_recv().is_err());
        assert!(matches!(
            Pin::new(&mut invocation).poll(&mut cx),
            Poll::Ready(Ok(()))
        ));
    }
    #[test]
    fn already_completed_invocations_await_inline_and_action_panics_are_task_errors() {
        let _environment = Environment::new();
        let dispatcher = ServiceDispatcher::new(None, None);
        let mut invocation = dispatcher.invoke_async(|| {});
        let (sender, receiver) = mpsc::channel();
        let waker = Waker::from(Arc::new(RecordWake(sender)));
        let mut cx = Context::from_waker(&waker);
        assert!(matches!(
            Pin::new(&mut invocation).poll(&mut cx),
            Poll::Ready(Ok(()))
        ));
        assert!(receiver.try_recv().is_err());
        let mut invocation = dispatcher.invoke_async(|| panic!("action failed"));
        let Poll::Ready(Err(error)) = Pin::new(&mut invocation).poll(&mut cx) else {
            panic!("expected completed fault");
        };
        assert_eq!(error.to_string(), "action failed");
    }
}
