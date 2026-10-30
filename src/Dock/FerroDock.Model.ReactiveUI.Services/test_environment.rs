use ferroui_reactive::concurrency::{IScheduler, ImmediateScheduler};
use ferroui_reactiveui::RxSchedulers;
use ferroui_system::threading::{
    current_synchronization_context, set_synchronization_context, SynchronizationContext,
};
use std::rc::Rc;
use std::sync::Arc;

pub(crate) struct Environment {
    context: Option<Arc<dyn SynchronizationContext>>,
    scheduler: Rc<dyn IScheduler>,
}
impl Environment {
    pub(crate) fn new() -> Self {
        let previous = Self {
            context: current_synchronization_context(),
            scheduler: RxSchedulers::main_thread_scheduler(),
        };
        set_synchronization_context(None);
        RxSchedulers::set_main_thread_scheduler(ImmediateScheduler::instance());
        previous
    }
}
impl Drop for Environment {
    fn drop(&mut self) {
        set_synchronization_context(self.context.take());
        RxSchedulers::set_main_thread_scheduler(self.scheduler.clone());
    }
}
