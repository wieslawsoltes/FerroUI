//! `RoutingStateTests` (`ReactiveUI.Tests/Routing/RoutingStateTests.cs`).

use ferroui_reactive::concurrency::{IScheduler, ScheduledAction};
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::IDisposable;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::time::{Duration, SystemTime};

/// A routable view model used as the navigation target (`TestViewModel`).
struct TestViewModel {
    reactive: ReactiveObject,
}
reactive_object!(TestViewModel);

impl IRoutableViewModel for TestViewModel {
    fn url_path_segment(&self) -> Option<String> {
        Some("test".to_owned())
    }

    fn host_screen(&self) -> Option<Rc<dyn IScreen>> {
        None
    }
}

/// A scheduler that holds work until [`run_newest_first`](Self::run_newest_first), then runs it
/// newest first (`ReorderingScheduler`): it stands in for a concurrent scheduler, which gives no
/// order between work scheduled apart.
struct ReorderingScheduler {
    this: Weak<ReorderingScheduler>,
    /// The pending work, newest last.
    pending: RefCell<Vec<ScheduledAction>>,
}

impl ReorderingScheduler {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), pending: RefCell::new(Vec::new()) })
    }

    /// Runs the pending work, newest first, until none remains.
    fn run_newest_first(&self) {
        let scheduler: Rc<dyn IScheduler> = self.this.upgrade().expect("the scheduler is alive while it runs");
        loop {
            let work = self.pending.borrow_mut().pop();
            let Some(work) = work else {
                break;
            };
            let _ = work(&scheduler);
        }
    }
}

impl IScheduler for ReorderingScheduler {
    fn now(&self) -> SystemTime {
        SystemTime::UNIX_EPOCH
    }

    fn schedule(&self, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.pending.borrow_mut().push(action);
        Disposable::empty()
    }

    fn schedule_after(&self, _due_time: Duration, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule(action)
    }

    fn schedule_at(&self, _due_time: SystemTime, action: ScheduledAction) -> Rc<dyn IDisposable> {
        self.schedule(action)
    }
}

#[test]
fn navigate_delivers_view_model_before_completion_on_reordering_scheduler() {
    let scheduler = ReorderingScheduler::new();
    let router = RoutingState::with_scheduler(Some(scheduler.clone()));
    let view_model: Rc<dyn IRoutableViewModel> = Rc::new(TestViewModel { reactive: ReactiveObject::new() });
    let notifications = Rc::new(RefCell::new(Vec::new()));
    let (on_next, on_completed) = (notifications.clone(), notifications.clone());
    let expected = view_model.clone();

    let subscription = router.navigate().execute(Some(view_model.clone())).subscribe_next_completed(
        move |vm| on_next.borrow_mut().push(if Rc::ptr_eq(&vm, &expected) { "next" } else { "next-other" }),
        move || on_completed.borrow_mut().push("completed"),
    );
    scheduler.run_newest_first();

    assert_eq!(*notifications.borrow(), ["next", "completed"]);
    subscription.dispose();
}
