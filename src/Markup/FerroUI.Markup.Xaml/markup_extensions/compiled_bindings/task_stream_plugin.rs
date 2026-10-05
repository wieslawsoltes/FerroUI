//! Port of `MarkupExtensions/CompiledBindings/TaskStreamPlugin.cs`.
//!
//! A task is a [`TaskValue`](ferroui_base::data::core::plugins::TaskValue):
//! its result is untyped, so the type argument of the managed plugin only
//! names the result type the compiled path expects. The handling of
//! completed, failed and running tasks is the one of the task stream plugin
//! of the base library, which this plugin starts (the continuation of a
//! running task is internal to the task type).

use ferroui_base::data::core::plugins::{IStreamPlugin, TaskStreamPlugin as BaseTaskStreamPlugin};
use ferroui_base::data::core::WeakValue;
use ferroui_base::reactive::IObservable;
use ferroui_base::BoxedValue;
use std::marker::PhantomData;
use std::rc::Rc;

/// Handles binding to tasks with a result of type `T` for the `^` stream
/// binding operator of compiled paths. (Internal in the managed original,
/// where compiled markup reaches it through friend access; public here for
/// the code the XAML compiler generates.)
pub struct TaskStreamPlugin<T> {
    result: PhantomData<fn() -> T>,
}

impl<T: 'static> TaskStreamPlugin<T> {
    pub fn new() -> Self {
        Self { result: PhantomData }
    }
}

impl<T: 'static> IStreamPlugin for TaskStreamPlugin<T> {
    fn match_(&self, reference: &WeakValue) -> bool {
        BaseTaskStreamPlugin.match_(reference)
    }

    fn start(&self, reference: &WeakValue) -> Option<Rc<dyn IObservable<Option<BoxedValue>>>> {
        BaseTaskStreamPlugin.start(reference)
    }
}
