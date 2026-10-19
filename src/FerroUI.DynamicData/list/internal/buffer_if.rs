//! `BufferIf<T>` (`List/Internal/BufferIf.cs`): the change sets held back while paused, delivered
//! as one when the pause ends or its time-out passes.

use crate::list::ChangeSet;
use ferroui_reactive::concurrency::{IScheduler, SchedulerDefaults};
use ferroui_reactive::disposables::{Disposable, SerialDisposable};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

/// `BufferIf.Run`: the pause signals are observed on `scheduler`; `time_out` of `None` or zero
/// waits for the end of the pause.
pub(crate) fn buffer_if<T: Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    pause_if_true_selector: Rc<dyn IObservable<bool>>,
    initial_pause_state: bool,
    time_out: Option<Duration>,
    scheduler: Option<Rc<dyn IScheduler>>,
) -> Rc<dyn IObservable<ChangeSet<T>>> {
    let scheduler = scheduler.unwrap_or_else(SchedulerDefaults::time_based_operations);
    let time_out = time_out.unwrap_or(Duration::ZERO);
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<T>>>| {
        let paused = Rc::new(Cell::new(initial_pause_state));
        let buffer: Rc<RefCell<Vec<crate::list::Change<T>>>> = Rc::new(RefCell::new(Vec::new()));
        let timeout_subscriber = SerialDisposable::new();
        let timeout_subject = Subject::<bool>::new();

        let buffer_selector = Observable::return_(initial_pause_state)
            .concat(pause_if_true_selector.merge(timeout_subject.as_observable()))
            .observe_on(scheduler.clone())
            .publish();
        let selector: Rc<dyn IObservable<bool>> = buffer_selector.clone();

        let pause = {
            let (paused, timeout_subscriber, timeout_subject, scheduler) = (paused.clone(), timeout_subscriber.clone(), timeout_subject.clone(), scheduler.clone());
            selector.where_(|state: &bool| *state).subscribe_next(move |_| {
                paused.set(true);
                if time_out != Duration::ZERO {
                    let timer = Observable::timer_on(time_out, scheduler.clone()).select(|_| false).subscribe_safe(timeout_subject.as_observer());
                    timeout_subscriber.set_disposable(Some(timer));
                }
            })
        };

        let resume = {
            let (paused, buffer, observer, timeout_subscriber) = (paused.clone(), buffer.clone(), observer.clone(), timeout_subscriber.clone());
            selector.where_(|state: &bool| !*state).subscribe_next(move |_| {
                paused.set(false);
                let buffered = std::mem::take(&mut *buffer.borrow_mut());
                if buffered.is_empty() {
                    return;
                }
                observer.on_next(ChangeSet::from(buffered));
                timeout_subscriber.set_disposable(Some(Disposable::empty()));
            })
        };

        let update_subscriber = {
            let (paused, buffer) = (paused.clone(), buffer.clone());
            source.subscribe_next(move |updates: ChangeSet<T>| {
                if paused.get() {
                    buffer.borrow_mut().extend(updates.iter().cloned());
                } else {
                    observer.on_next(updates);
                }
            })
        };

        let connected = buffer_selector.connect();
        Disposable::create(move || {
            connected.dispose();
            pause.dispose();
            resume.dispose();
            update_subscriber.dispose();
            timeout_subject.on_completed();
            timeout_subscriber.dispose();
        })
    })
}
