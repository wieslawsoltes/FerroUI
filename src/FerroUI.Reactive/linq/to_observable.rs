//! `ToObservable` of an enumerable sequence (`Linq/Observable/ToObservable.cs`), and the
//! `Subscribe` of an enumerable sequence that it implements (`QueryLanguage.Conversions.cs`).

use crate::concurrency::{IScheduler, SchedulerDefaults};
use crate::disposables::Disposable;
use crate::internal::{subscribe_sink, Sink};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// An enumerator of the values of the sequence, each of which may be an error (`IEnumerator<T>`,
/// whose `MoveNext` may throw).
type Enumerator<T> = Box<dyn Iterator<Item = Result<T, ObservableError>>>;

/// `ToObservableRecursive<TSource>`: each value is notified by work of its own on the scheduler,
/// which schedules the next. An iterator is made for each subscription (`GetEnumerator`) and
/// dropped (`Dispose`) when it ends, fails, or after the subscription was disposed.
///
/// Deviation (DEVIATIONS.md, Reactive): the schedulers of the port have no long-running service
/// (`ISchedulerLongRunning`), so the loop of `ToObservableLongRunning` has no counterpart; every
/// scheduler takes the recursive form. `GetEnumerator` cannot fail (`IntoIterator::into_iter`).
struct ToObservableRecursive<T> {
    source: Rc<dyn Fn() -> Enumerator<T>>,
    scheduler: Rc<dyn IScheduler>,
}

/// The sink of `ToObservableRecursive`.
struct ToObservableSink<T> {
    sink: Sink<T>,
    enumerator: RefCell<Option<Enumerator<T>>>,
    disposed: Cell<bool>,
}

impl<T: 'static> ToObservableSink<T> {
    fn run(self: &Rc<Self>, source: &Rc<dyn Fn() -> Enumerator<T>>, scheduler: &Rc<dyn IScheduler>) {
        *self.enumerator.borrow_mut() = Some(source());

        // The scheduled work is never cancelled: the flag of the disposal has the loop end and
        // drop the enumerator.
        let this = self.clone();
        let _ = scheduler.schedule(Box::new(move |inner| this.loop_rec(inner)));
    }

    /// Drops the enumerator (`enumerator.Dispose()`), outside of the borrow of its slot.
    fn dispose_enumerator(&self) {
        let enumerator = self.enumerator.borrow_mut().take();
        drop(enumerator);
    }

    fn loop_rec(self: &Rc<Self>, scheduler: &Rc<dyn IScheduler>) -> Rc<dyn IDisposable> {
        if self.disposed.get() {
            self.dispose_enumerator();
            return Disposable::empty();
        }

        // The enumerator leaves its slot while it moves, so that nothing it runs borrows the slot.
        let Some(mut enumerator) = self.enumerator.borrow_mut().take() else {
            return Disposable::empty();
        };
        let next = enumerator.next();
        *self.enumerator.borrow_mut() = Some(enumerator);

        match next {
            Some(Err(error)) => {
                self.dispose_enumerator();
                self.sink.forward_on_error(error);
            }
            None => {
                self.dispose_enumerator();
                self.sink.forward_on_completed();
            }
            Some(Ok(current)) => {
                self.sink.forward_on_next(current);
                let this = self.clone();
                let _ = scheduler.schedule(Box::new(move |inner| this.loop_rec(inner)));
            }
        }
        Disposable::empty()
    }
}

impl<T: 'static> IDisposable for ToObservableSink<T> {
    fn dispose(&self) {
        self.sink.dispose();
        self.disposed.set(true);
    }
}

impl<T: 'static> IObservable<T> for ToObservableRecursive<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let source = self.source.clone();
        let scheduler = self.scheduler.clone();
        subscribe_sink(
            observer,
            |observer| Rc::new(ToObservableSink { sink: Sink::new(observer), enumerator: RefCell::new(None), disposed: Cell::new(false) }),
            move |sink: &Rc<ToObservableSink<T>>| sink.run(&source, &scheduler),
        )
    }
}

fn enumerable<I>(source: I) -> Rc<dyn Fn() -> Enumerator<I::Item>>
where
    I: IntoIterator + Clone + 'static,
    I::Item: 'static,
{
    Rc::new(move || -> Enumerator<I::Item> { Box::new(source.clone().into_iter().map(Ok)) })
}

fn try_enumerable<I, T>(source: I) -> Rc<dyn Fn() -> Enumerator<T>>
where
    I: IntoIterator<Item = Result<T, ObservableError>> + Clone + 'static,
    T: 'static,
{
    Rc::new(move || -> Enumerator<T> { Box::new(source.clone().into_iter()) })
}

impl crate::linq::Observable {
    /// The values of `source`, on the scheduler of iteration, the current-thread one
    /// (`ToObservable<TSource>(IEnumerable<TSource>)`). Each subscription iterates a clone of
    /// `source`.
    pub fn to_observable<I>(source: I) -> Rc<dyn IObservable<I::Item>>
    where
        I: IntoIterator + Clone + 'static,
        I::Item: 'static,
    {
        Self::to_observable_on(source, SchedulerDefaults::iteration())
    }

    /// The values of `source`, each notified by work of its own on `scheduler`
    /// (`ToObservable<TSource>(IEnumerable<TSource>, IScheduler)`).
    pub fn to_observable_on<I>(source: I, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<I::Item>>
    where
        I: IntoIterator + Clone + 'static,
        I::Item: 'static,
    {
        Rc::new(ToObservableRecursive { source: enumerable(source), scheduler })
    }

    /// [`to_observable`](Self::to_observable) of a sequence whose iteration may fail: an error
    /// ends the sequence with it (`ToObservable` of an enumerable whose `MoveNext` throws).
    pub fn try_to_observable<I, T>(source: I) -> Rc<dyn IObservable<T>>
    where
        I: IntoIterator<Item = Result<T, ObservableError>> + Clone + 'static,
        T: 'static,
    {
        Self::try_to_observable_on(source, SchedulerDefaults::iteration())
    }

    /// [`to_observable_on`](Self::to_observable_on) of a sequence whose iteration may fail: an
    /// error ends the sequence with it (`ToObservable` of an enumerable whose `MoveNext` throws).
    pub fn try_to_observable_on<I, T>(source: I, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>>
    where
        I: IntoIterator<Item = Result<T, ObservableError>> + Clone + 'static,
        T: 'static,
    {
        Rc::new(ToObservableRecursive { source: try_enumerable(source), scheduler })
    }

    /// Notifies the values of `source` to `observer`, on the scheduler of iteration, the
    /// current-thread one (`Subscribe<TSource>(IEnumerable<TSource>, IObserver<TSource>)`).
    pub fn subscribe_enumerable<I>(source: I, observer: Rc<dyn IObserver<I::Item>>) -> Rc<dyn IDisposable>
    where
        I: IntoIterator + Clone + 'static,
        I::Item: 'static,
    {
        Self::subscribe_enumerable_on(source, observer, SchedulerDefaults::iteration())
    }

    /// Notifies the values of `source` to `observer`, each by work of its own on `scheduler`
    /// (`Subscribe<TSource>(IEnumerable<TSource>, IObserver<TSource>, IScheduler)`).
    pub fn subscribe_enumerable_on<I>(source: I, observer: Rc<dyn IObserver<I::Item>>, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IDisposable>
    where
        I: IntoIterator + Clone + 'static,
        I::Item: 'static,
    {
        ToObservableRecursive { source: enumerable(source), scheduler }.subscribe(observer)
    }
}
