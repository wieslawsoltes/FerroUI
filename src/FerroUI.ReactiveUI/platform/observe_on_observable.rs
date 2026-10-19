//! `ObserveOnObservable<T>` (`Shared/Platform/ObserveOnObservable.cs`).

use ferroui_reactive::concurrency::{IScheduler, SchedulerExt};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::rc::Rc;

/// The notifications of the source, each delivered on a scheduler (`ObserveOnObservable<T>`):
/// `ObserveOn(scheduler)` as a wrapper of the observer, without an operator pipeline.
pub struct ObserveOnObservable<T> {
    source: Rc<dyn IObservable<T>>,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> ObserveOnObservable<T> {
    /// `source`, delivered on `scheduler` (`ObserveOnObservable(IObservable<T>, ISequencer)`).
    pub fn new(source: Rc<dyn IObservable<T>>, scheduler: Rc<dyn IScheduler>) -> Rc<Self> {
        Rc::new(Self { source, scheduler })
    }
}

impl<T: 'static> IObservable<T> for ObserveOnObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        self.source.subscribe(Rc::new(ObserveOnSink { downstream: observer, scheduler: self.scheduler.clone() }))
    }
}

/// `ObserveOnObservable.Sink`: schedules each notification.
struct ObserveOnSink<T> {
    downstream: Rc<dyn IObserver<T>>,
    scheduler: Rc<dyn IScheduler>,
}

impl<T: 'static> IObserver<T> for ObserveOnSink<T> {
    fn on_next(&self, value: T) {
        let downstream = self.downstream.clone();
        let _ = self.scheduler.schedule_action(move || downstream.on_next(value));
    }

    fn on_error(&self, error: ObservableError) {
        let downstream = self.downstream.clone();
        let _ = self.scheduler.schedule_action(move || downstream.on_error(error));
    }

    fn on_completed(&self) {
        let downstream = self.downstream.clone();
        let _ = self.scheduler.schedule_action(move || downstream.on_completed());
    }
}
