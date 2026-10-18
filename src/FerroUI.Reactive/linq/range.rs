//! `Range` (`Linq/Observable/Range.cs`).

use super::creation::try_set_first;
use crate::concurrency::{IScheduler, SchedulerDefaults};
use crate::disposables::{Disposable, MultipleAssignmentDisposableValue};
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::Cell;
use std::rc::Rc;

/// `RangeRecursive`: each value is notified by work of its own on the scheduler, which schedules
/// the next.
///
/// Deviation (DEVIATIONS.md, Reactive): the schedulers of the port have no long-running service
/// (`ISchedulerLongRunning`), so the loop of `RangeLongRunning` has no counterpart; every scheduler
/// takes the recursive form.
struct RangeObservable {
    start: i32,
    count: usize,
    scheduler: Rc<dyn IScheduler>,
}

/// The sink of `RangeRecursive` (`RangeSink`). The index and the end are counted in `i64`, where
/// upstream lets `start + count` overflow `int` and compares the wrapped values.
struct RangeSink {
    sink: Sink<i32>,
    index: Cell<i64>,
    end: i64,
    task: MultipleAssignmentDisposableValue,
}

impl RangeSink {
    fn run(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>) {
        let this = self.clone();
        let first = scheduler.schedule(Box::new(move |inner| this.loop_rec(inner)));
        try_set_first(&self.task, first);
    }

    fn loop_rec(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>) -> Rc<dyn IDisposable> {
        let index = self.index.get();
        if index != self.end {
            self.index.set(index + 1);
            self.sink.forward_on_next(index as i32);
            let this = self.clone();
            let next = scheduler.schedule(Box::new(move |inner| this.loop_rec(inner)));
            self.task.set_disposable(Some(next));
        } else {
            let observer = self.sink.observer();
            observer.on_completed();
            self.dispose();
        }
        Disposable::empty()
    }
}

impl IDisposable for RangeSink {
    fn dispose(&self) {
        self.sink.dispose();
        self.task.dispose();
    }
}

impl IObservable<i32> for RangeObservable {
    fn subscribe(&self, observer: Rc<dyn IObserver<i32>>) -> Rc<dyn IDisposable> {
        let (start, count, scheduler) = (self.start, self.count, self.scheduler.clone());
        subscribe_sink(
            observer,
            move |observer| {
                Rc::new(RangeSink {
                    sink: Sink::new(observer),
                    index: Cell::new(start as i64),
                    end: start as i64 + count as i64,
                    task: MultipleAssignmentDisposableValue::new(),
                })
            },
            move |sink: &Rc<RangeSink>| sink.run(&scheduler),
        )
    }
}

impl crate::linq::Observable {
    /// The `count` integers from `start`, on the scheduler of iteration, the current-thread one
    /// (`Range(int, int)`). Panics when the last one is beyond `i32::MAX`, where upstream throws
    /// `ArgumentOutOfRangeException`.
    pub fn range(start: i32, count: usize) -> Rc<dyn IObservable<i32>> {
        Self::range_on(start, count, SchedulerDefaults::iteration())
    }

    /// The `count` integers from `start`, each notified by work of its own on `scheduler`
    /// (`Range(int, int, IScheduler)`). Panics when the last one is beyond `i32::MAX`, where
    /// upstream throws `ArgumentOutOfRangeException`.
    pub fn range_on(start: i32, count: usize, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<i32>> {
        let max = start as i128 + count as i128 - 1;
        if max > i32::MAX as i128 {
            panic!("Specified argument was out of the range of valid values. (Parameter 'count')");
        }
        Rc::new(RangeObservable { start, count, scheduler })
    }
}
