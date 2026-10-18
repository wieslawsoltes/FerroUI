//! `Repeat` of a value (`Linq/Observable/Repeat.cs`): the factories `Repeat<TResult>(TResult, ...)`.
//! `Repeat` of a source sequence is an operator of its own family.

use super::creation::try_set_first;
use crate::concurrency::{IScheduler, SchedulerDefaults};
use crate::disposables::{Disposable, MultipleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::Cell;
use std::rc::Rc;

/// `Repeat<TResult>.ForeverRecursive`: the value, forever, each time by work of its own on the
/// scheduler.
///
/// Deviation (DEVIATIONS.md, Reactive): the schedulers of the port have no long-running service
/// (`ISchedulerLongRunning`), so the loops of `ForeverLongRunning` and `CountLongRunning` have no
/// counterpart; every scheduler takes the recursive forms.
struct RepeatForever<T> {
    value: T,
    scheduler: Rc<dyn IScheduler>,
}

/// The sink of `ForeverRecursive`.
struct ForeverSink<T> {
    sink: Sink<T>,
    value: T,
    task: MultipleAssignmentDisposableValue,
}

impl<T: Clone + 'static> ForeverSink<T> {
    fn run(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>) {
        let this = self.clone();
        let first = scheduler.schedule(Box::new(move |inner| this.loop_rec_inf(inner)));
        try_set_first(&self.task, first);
    }

    fn loop_rec_inf(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>) -> Rc<dyn IDisposable> {
        self.sink.forward_on_next(self.value.clone());
        let this = self.clone();
        let next = scheduler.schedule(Box::new(move |inner| this.loop_rec_inf(inner)));
        self.task.set_disposable(Some(next));
        Disposable::empty()
    }
}

impl<T: 'static> IDisposable for ForeverSink<T> {
    fn dispose(&self) {
        self.sink.dispose();
        self.task.dispose();
    }
}

impl<T: Clone + 'static> IObservable<T> for RepeatForever<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let value = self.value.clone();
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            move |observer| Rc::new(ForeverSink { sink: Sink::new(observer), value, task: MultipleAssignmentDisposableValue::new() }),
            move |sink: &Rc<ForeverSink<T>>| sink.run(&scheduler),
        )
    }
}

/// `Repeat<TResult>.CountRecursive`: the value, a number of times, each time by work of its own
/// on the scheduler.
struct RepeatCount<T> {
    value: T,
    repeat_count: usize,
    scheduler: Rc<dyn IScheduler>,
}

/// The sink of `CountRecursive`.
struct CountSink<T> {
    sink: Sink<T>,
    value: T,
    remaining: Cell<usize>,
    task: MultipleAssignmentDisposableValue,
}

impl<T: Clone + 'static> CountSink<T> {
    fn run(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>) {
        let this = self.clone();
        let first = scheduler.schedule(Box::new(move |inner| this.loop_rec(inner)));
        try_set_first(&self.task, first);
    }

    fn loop_rec(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>) -> Rc<dyn IDisposable> {
        let mut remaining = self.remaining.get();
        if remaining > 0 {
            self.sink.forward_on_next(self.value.clone());
            remaining -= 1;
            self.remaining.set(remaining);
        }

        if remaining == 0 {
            let observer = self.sink.observer();
            observer.on_completed();
            self.dispose();
        } else {
            let this = self.clone();
            let next = scheduler.schedule(Box::new(move |inner| this.loop_rec(inner)));
            self.task.set_disposable(Some(next));
        }
        Disposable::empty()
    }
}

impl<T: 'static> IDisposable for CountSink<T> {
    fn dispose(&self) {
        self.sink.dispose();
        self.task.dispose();
    }
}

impl<T: Clone + 'static> IObservable<T> for RepeatCount<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let value = self.value.clone();
        let repeat_count = self.repeat_count;
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new(CountSink {
                    sink: Sink::new(observer),
                    value,
                    remaining: Cell::new(repeat_count),
                    task: MultipleAssignmentDisposableValue::new(),
                })
            },
            move |sink: &Rc<CountSink<T>>| sink.run(&scheduler),
        )
    }
}

impl crate::linq::Observable {
    /// The value, forever, on the scheduler of iteration, the current-thread one
    /// (`Repeat<TResult>(TResult)`).
    pub fn repeat<T: Clone + 'static>(value: T) -> Rc<dyn IObservable<T>> {
        Self::repeat_on(value, SchedulerDefaults::iteration())
    }

    /// The value, forever, each time by work of its own on `scheduler`
    /// (`Repeat<TResult>(TResult, IScheduler)`).
    pub fn repeat_on<T: Clone + 'static>(value: T, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        Rc::new(RepeatForever { value, scheduler })
    }

    /// The value, `repeat_count` times, on the scheduler of iteration, the current-thread one
    /// (`Repeat<TResult>(TResult, int)`).
    pub fn repeat_count<T: Clone + 'static>(value: T, repeat_count: usize) -> Rc<dyn IObservable<T>> {
        Self::repeat_count_on(value, repeat_count, SchedulerDefaults::iteration())
    }

    /// The value, `repeat_count` times, each time by work of its own on `scheduler`
    /// (`Repeat<TResult>(TResult, int, IScheduler)`).
    pub fn repeat_count_on<T: Clone + 'static>(value: T, repeat_count: usize, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
        Rc::new(RepeatCount { value, repeat_count, scheduler })
    }
}
