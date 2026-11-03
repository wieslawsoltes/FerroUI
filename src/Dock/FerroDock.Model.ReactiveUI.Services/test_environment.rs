use ferroui_reactive::concurrency::{IScheduler, ImmediateScheduler};
use ferroui_reactiveui::RxSchedulers;
use ferroui_system::task::{set_current_schedule, Schedule};
use std::rc::Rc;
use std::sync::Arc;

/// The environment of a test of the services: no current schedule (the synchronization context
/// of the reference), and the immediate scheduler as the Rx main-thread scheduler; both are
/// restored when it is dropped.
pub(crate) struct Environment {
    schedule: Option<Arc<dyn Schedule>>,
    scheduler: Rc<dyn IScheduler>,
}
impl Environment {
    pub(crate) fn new() -> Self {
        let previous = Self {
            schedule: set_current_schedule(None),
            scheduler: RxSchedulers::main_thread_scheduler(),
        };
        RxSchedulers::set_main_thread_scheduler(ImmediateScheduler::instance());
        previous
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        drop(set_current_schedule(self.schedule.take()));
        RxSchedulers::set_main_thread_scheduler(self.scheduler.clone());
    }
}
