// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferrodock_model_reactiveui::services::IDockDispatcher;
use ferroui_reactive::concurrency::SchedulerExt;
use ferroui_reactiveui::RxSchedulers;
use ferroui_system::threading::tasks::{Task, TaskCompletionSource};
use std::rc::Rc;
/// Source default dispatcher: looks up RxSchedulers.MainThreadScheduler on every
/// invocation and always schedules through it, independently of synchronization context.
#[derive(Default)]
pub struct MainThreadDispatcher;
impl MainThreadDispatcher {
    /// Creates the stateless dispatcher, without capturing a scheduler or context.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    /// Schedules one invocation and captures action failure in the shared System task.
    /// Scheduling failure itself propagates synchronously, as in source.
    /// The pinned shared task lacks RunContinuationsAsynchronously; that runtime
    /// option remains a recorded parity gap, rather than a separate task engine.
    pub fn invoke_async(&self, action: Rc<dyn Fn()>) -> Task<()> {
        let completion = TaskCompletionSource::new();
        let task = completion.task();
        let _ = RxSchedulers::main_thread_scheduler().schedule_action(move || {
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| action())) {
                Ok(()) => {
                    completion.try_set_result(());
                }
                Err(payload) => {
                    completion.try_set_exception(super::task_action::panic_error(payload));
                }
            }
        });
        task
    }
}
impl IDockDispatcher for MainThreadDispatcher {
    fn invoke_async(&self, action: Rc<dyn Fn()>) -> Task<()> {
        self.invoke_async(action)
    }
}
#[cfg(test)]
#[path = "main_thread_dispatcher_tests.rs"]
mod tests;
