//! `SchedulerExtensions` (`ReactiveUI.Testing/SchedulerExtensions.cs`).

use ferroui_reactive::concurrency::IScheduler;
use ferroui_reactive::disposables::Disposable;
use ferroui_reactive_testing::TestScheduler;
use ferroui_reactiveui::{PanicException, RxSchedulers};
use ferroui_system::threading::tasks::Task;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// Restores the schedulers of [`RxSchedulers`] when it is dropped: the `using` block of upstream's
/// `With`, which restores them also when the block throws (here, panics).
struct RestoreOnDrop(Rc<dyn IDisposable>);

impl Drop for RestoreOnDrop {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// Runs code with a scheduler as both schedulers of [`RxSchedulers`], the main thread scheduler
/// and the task pool scheduler (`SchedulerExtensions`).
///
/// It is implemented for a scheduler handle (`Rc<dyn IScheduler>`, `Rc<S>`) and for the
/// [`TestScheduler`] of `ferroui-reactive-testing`; the block receives the scheduler it was called
/// on.
///
/// Deviation (DEVIATIONS.md, ReactiveUI): the schedulers of [`RxSchedulers`] are per thread, so the
/// scheduler is the scheduler of the current thread only.
pub trait SchedulerExtensions {
    /// The scheduler as the handle that [`RxSchedulers`] holds.
    fn as_scheduler(&self) -> Rc<dyn IScheduler>;

    /// Makes the scheduler both schedulers of [`RxSchedulers`] until the returned disposable is
    /// disposed, which restores the schedulers set before (`WithScheduler`).
    fn with_scheduler(&self) -> Rc<dyn IDisposable> {
        let previous_main_thread = RxSchedulers::main_thread_scheduler();
        let previous_task_pool = RxSchedulers::task_pool_scheduler();

        let scheduler = self.as_scheduler();
        RxSchedulers::set_main_thread_scheduler(scheduler.clone());
        RxSchedulers::set_task_pool_scheduler(scheduler);

        Disposable::create(move || {
            RxSchedulers::set_main_thread_scheduler(previous_main_thread);
            RxSchedulers::set_task_pool_scheduler(previous_task_pool);
        })
    }

    /// Runs `block` with the scheduler as both schedulers of [`RxSchedulers`] and returns its
    /// result; the schedulers set before are restored afterwards, also when `block` panics
    /// (`With<TRet>(Func<T, TRet>)`, and `With(Action<T>)` with a block that returns `()`).
    fn with<R>(&self, block: impl FnOnce(&Self) -> R) -> R {
        let _restore = RestoreOnDrop(self.with_scheduler());
        block(self)
    }

    /// Runs `block` with the scheduler as both schedulers of [`RxSchedulers`] until the task it
    /// returns ends, and gives the outcome of that task (`WithAsync<TRet>(Func<T, Task<TRet>>)`,
    /// and `WithAsync(Func<T, Task>)` with a `Task<()>`).
    ///
    /// A panic of `block` ends the returned task with the error of the panic
    /// ([`PanicException`]), as an exception thrown in an `async` method faults its task; the
    /// schedulers are restored in every case.
    fn with_async<R: Clone + 'static>(&self, block: impl FnOnce(&Self) -> Task<R>) -> Task<R> {
        let restore = self.with_scheduler();
        let task = match PanicException::catch(|| block(self)) {
            Ok(task) => task,
            Err((error, _payload)) => {
                restore.dispose();
                return Task::from_exception(error);
            }
        };
        Task::spawn_local(async move {
            let outcome = task.await;
            restore.dispose();
            outcome
        })
    }
}

impl SchedulerExtensions for Rc<dyn IScheduler> {
    fn as_scheduler(&self) -> Rc<dyn IScheduler> {
        self.clone()
    }
}

impl<S: IScheduler + 'static> SchedulerExtensions for Rc<S> {
    fn as_scheduler(&self) -> Rc<dyn IScheduler> {
        self.clone()
    }
}

impl SchedulerExtensions for TestScheduler {
    fn as_scheduler(&self) -> Rc<dyn IScheduler> {
        self.scheduler()
    }
}
