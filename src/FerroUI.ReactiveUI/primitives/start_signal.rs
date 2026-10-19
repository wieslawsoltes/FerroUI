//! `StartSignal<T>` (ReactiveUI.Primitives, `Advanced/StartSignal{T}.cs`, with
//! `SubscriptionScheduling.RunOn`): a cold sequence that runs a function on a scheduler for each
//! subscription and notifies its result.
//!
//! It is not Rx's `Observable.Start`, which runs the function once, at once, and shares its result
//! (an `AsyncSubject`): the commands of ReactiveUI run their function for each execution, when the
//! execution is subscribed.

use crate::internal::scheduler_extensions::is_immediate;
use crate::PanicException;
use ferroui_reactive::concurrency::{IScheduler, SchedulerExt};
use ferroui_reactive::disposables::Disposable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// Runs `function` on `scheduler` for each subscription, then notifies its result and the end, or
/// its error (`StartSignal<T>`).
///
/// Upstream catches the exception of the function and notifies it; here the function returns a
/// `Result`, whose `Err` is notified, and a panic of the function is caught and notified as a
/// [`PanicException`].
pub(crate) struct StartSignal<T> {
    function: Rc<dyn Fn() -> Result<T, ObservableError>>,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> StartSignal<T> {
    /// A sequence that runs `function` on `scheduler` (`new StartSignal<T>(Func<T>, ISequencer)`).
    pub(crate) fn new(function: impl Fn() -> Result<T, ObservableError> + 'static, scheduler: Rc<dyn IScheduler>) -> Rc<Self> {
        Rc::new(Self { function: Rc::new(function), scheduler })
    }

    fn run(function: &Rc<dyn Fn() -> Result<T, ObservableError>>, observer: &Rc<dyn IObserver<T>>) {
        match PanicException::catch(|| function()) {
            Ok(Ok(value)) => {
                observer.on_next(value);
                observer.on_completed();
            }
            Ok(Err(error)) | Err((error, _)) => observer.on_error(error),
        }
    }
}

impl<T: 'static> IObservable<T> for StartSignal<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        // `SubscriptionScheduling.RunOn`: inline on the immediate scheduler, scheduled on any
        // other (the current-thread scheduler queues the work when its trampoline is running).
        if is_immediate(&self.scheduler) {
            Self::run(&self.function, &observer);
            return Disposable::empty();
        }
        let function = self.function.clone();
        self.scheduler.schedule_action(move || Self::run(&function, &observer))
    }
}
