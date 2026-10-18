//! `Expand` (`Linq/QueryLanguageEx.cs`, the experimental `ObservableEx.Expand`): the values of a
//! sequence and, recursively, of the sequences its selector makes of each value.

use crate::concurrency::{IScheduler, SchedulerExt};
use crate::disposables::{CompositeDisposable, SerialDisposable, SingleAssignmentDisposable};
use crate::internal::subscribe_auto_detach;
use crate::linq::ObservableExt;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

type Selector<T> = Rc<dyn Fn(T) -> Result<Rc<dyn IObservable<T>>, ObservableError>>;

/// `ExpandObservable<TSource>` (an `ObservableBase<TSource>`).
struct ExpandObservable<T> {
    source: Rc<dyn IObservable<T>>,
    selector: Selector<T>,
    scheduler: Rc<dyn IScheduler>,
}

/// The state of a subscription (the locals of `ExpandObservable<TSource>.SubscribeCore`): the
/// sequences waiting to be subscribed, the subscriptions, and the count of the sequences that have
/// not completed.
struct ExpandState<T> {
    observer: Rc<dyn IObserver<T>>,
    selector: Selector<T>,
    scheduler: Rc<dyn IScheduler>,
    queue: RefCell<VecDeque<Rc<dyn IObservable<T>>>>,
    m: Rc<SerialDisposable>,
    d: Rc<CompositeDisposable>,
    active_count: Cell<usize>,
    is_acquired: Cell<bool>,
}

impl<T: Clone + 'static> ExpandState<T> {
    /// `ensureActive`: when sequences wait and no loop drains them, a loop on the scheduler
    /// subscribes to them one at a time.
    fn ensure_active(self: &Rc<Self>) {
        let is_owner = if self.queue.borrow().is_empty() {
            false
        } else {
            !self.is_acquired.replace(true)
        };
        if !is_owner {
            return;
        }
        let this = self.clone();
        let work = self.scheduler.schedule_recursive(move |recurse| {
            let next = this.queue.borrow_mut().pop_front();
            let Some(work) = next else {
                this.is_acquired.set(false);
                return;
            };
            this.subscribe_work(work);
            recurse();
        });
        self.m.set_disposable(Some(work));
    }

    /// Subscribes to one sequence: each value is notified, and the sequence the selector makes of
    /// it waits in the queue.
    fn subscribe_work(self: &Rc<Self>, work: Rc<dyn IObservable<T>>) {
        let m1 = SingleAssignmentDisposable::new();
        let m1_handle: Rc<dyn IDisposable> = m1.clone();
        self.d.add(m1_handle.clone());

        let (on_next, on_error, on_completed) = (self.clone(), self.clone(), self.clone());
        let subscription = work.subscribe_all(
            move |value: T| {
                on_next.observer.on_next(value.clone());
                let result = match (on_next.selector)(value) {
                    Ok(result) => result,
                    Err(error) => {
                        on_next.observer.on_error(error);
                        return;
                    }
                };
                on_next.queue.borrow_mut().push_back(result);
                on_next.active_count.set(on_next.active_count.get() + 1);
                on_next.ensure_active();
            },
            move |error| on_error.observer.on_error(error),
            move || {
                on_completed.d.remove(&m1_handle);
                let active_count = on_completed.active_count.get() - 1;
                on_completed.active_count.set(active_count);
                if active_count == 0 {
                    on_completed.observer.on_completed();
                }
            },
        );
        m1.set_disposable(Some(subscription));
    }
}

impl<T: Clone + 'static> IObservable<T> for ExpandObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let (source, selector, scheduler) = (self.source.clone(), self.selector.clone(), self.scheduler.clone());
        subscribe_auto_detach(observer, move |observer: Rc<dyn IObserver<T>>| -> Rc<dyn IDisposable> {
            let m = SerialDisposable::new();
            let d = CompositeDisposable::new();
            d.add(m.clone());
            let state = Rc::new(ExpandState {
                observer,
                selector,
                scheduler,
                queue: RefCell::new(VecDeque::new()),
                m,
                d: d.clone(),
                active_count: Cell::new(0),
                is_acquired: Cell::new(false),
            });
            state.queue.borrow_mut().push_back(source);
            state.active_count.set(state.active_count.get() + 1);
            state.ensure_active();
            d
        })
    }
}

pub(crate) fn expand<T: Clone + 'static>(source: Rc<dyn IObservable<T>>, selector: Selector<T>, scheduler: Rc<dyn IScheduler>) -> Rc<dyn IObservable<T>> {
    Rc::new(ExpandObservable { source, selector, scheduler })
}

/// The methods of `Expand` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! expand_methods {
    () => {
        /// The values of the sequence and, recursively, of the sequences `selector` makes of each
        /// value, subscribed on the scheduler of iteration, the current-thread one
        /// (`Expand<TSource>(IObservable<TSource>, Func<TSource, IObservable<TSource>>)`).
        fn expand(
            &self,
            selector: impl Fn(T) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            self.expand_on(selector, crate::concurrency::SchedulerDefaults::iteration())
        }

        /// [`expand`](Self::expand), which subscribes to the sequences on `scheduler`
        /// (`Expand<TSource>(IObservable<TSource>, Func<TSource, IObservable<TSource>>, IScheduler)`).
        fn expand_on(
            &self,
            selector: impl Fn(T) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> + 'static,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::expand::expand(self.observable(), std::rc::Rc::new(move |value| Ok(selector(value))), scheduler)
        }

        /// [`expand`](Self::expand) with a selector that may fail; a failure ends the sequence with
        /// its error (`Expand` with a selector that throws).
        fn try_expand(
            &self,
            selector: impl Fn(T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<T>>, ferroui_system::ObservableError> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            self.try_expand_on(selector, crate::concurrency::SchedulerDefaults::iteration())
        }

        /// [`try_expand`](Self::try_expand) on `scheduler`.
        fn try_expand_on(
            &self,
            selector: impl Fn(T) -> Result<std::rc::Rc<dyn ferroui_system::IObservable<T>>, ferroui_system::ObservableError> + 'static,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>>
        where
            T: Clone,
        {
            crate::linq::expand::expand(self.observable(), std::rc::Rc::new(selector), scheduler)
        }
    };
}
pub(crate) use expand_methods;
