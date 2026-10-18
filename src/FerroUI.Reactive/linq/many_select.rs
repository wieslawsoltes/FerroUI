//! `ManySelect` (`Linq/QueryLanguageEx.cs`, the experimental `ObservableEx.ManySelect`): the
//! projection of each value with the sequence of the values from it on (a comonadic bind).

use crate::concurrency::{CurrentThreadScheduler, IScheduler, SchedulerExt};
use crate::disposables::CompositeDisposable;
use crate::linq::{Observable, ObservableExt};
use crate::subjects::AsyncSubject;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

/// `ChainObservable<T>` (an `ISubject<IObservable<T>, T>`): the sequence of a value and the values
/// after it. Its observers receive the value, then the values of the sequence the chain is given
/// next (the chain of the next value, or the end of the source).
struct ChainObservable<T> {
    head: T,
    tail: Rc<AsyncSubject<Rc<dyn IObservable<T>>>>,
}

impl<T: Clone + 'static> ChainObservable<T> {
    fn new(head: T) -> Rc<Self> {
        Rc::new(Self { head, tail: AsyncSubject::new() })
    }
}

impl<T: Clone + 'static> IObservable<T> for ChainObservable<T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let g = CompositeDisposable::new();
        let (group, head, tail) = (g.clone(), self.head.clone(), self.tail.clone());
        g.add(CurrentThreadScheduler::instance().schedule_action(move || {
            observer.on_next(head);
            group.add(tail.as_observable().merge_inner::<T>().subscribe(observer));
        }));
        g
    }
}

impl<T: Clone + 'static> IObserver<Rc<dyn IObservable<T>>> for ChainObservable<T> {
    fn on_next(&self, value: Rc<dyn IObservable<T>>) {
        self.tail.on_next(value);
        self.tail.on_completed();
    }

    fn on_error(&self, error: ObservableError) {
        self.on_next(Observable::throw(error));
    }

    fn on_completed(&self) {
        self.on_next(Observable::empty());
    }
}

pub(crate) fn many_select<T: Clone + 'static, R: 'static>(
    source: Rc<dyn IObservable<T>>,
    selector: Rc<dyn Fn(Rc<dyn IObservable<T>>) -> R>,
    scheduler: Rc<dyn IScheduler>,
) -> Rc<dyn IObservable<R>> {
    Observable::defer(move || {
        let chain: Rc<RefCell<Option<Rc<ChainObservable<T>>>>> = Rc::new(RefCell::new(None));
        let (on_value, on_error, on_completed) = (chain.clone(), chain.clone(), chain);
        let selector = selector.clone();
        source
            .select(move |value: T| {
                let current = ChainObservable::new(value);
                let previous = on_value.borrow().clone();
                if let Some(previous) = previous {
                    previous.on_next(current.clone() as Rc<dyn IObservable<T>>);
                }
                *on_value.borrow_mut() = Some(current.clone());
                current as Rc<dyn IObservable<T>>
            })
            .do_all(
                |_| {},
                move |error| {
                    let chain = on_error.borrow().clone();
                    if let Some(chain) = chain {
                        chain.on_error(error);
                    }
                },
                move || {
                    let chain = on_completed.borrow().clone();
                    if let Some(chain) = chain {
                        chain.on_completed();
                    }
                },
            )
            .observe_on(scheduler.clone())
            .select(move |chain| selector(chain))
    })
}

/// The methods of `ManySelect` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! many_select_methods {
    () => {
        /// The projections of the sequences of each value and the values after it, delivered on the
        /// default scheduler of the thread (`ManySelect<TSource, TResult>(IObservable<TSource>,
        /// Func<IObservable<TSource>, TResult>)`, which upstream delivers on `DefaultScheduler.Instance`).
        fn many_select<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> R + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            self.many_select_on(selector, crate::concurrency::SchedulerDefaults::default_scheduler())
        }

        /// [`many_select`](Self::many_select), delivered on `scheduler`
        /// (`ManySelect<TSource, TResult>(IObservable<TSource>, Func<IObservable<TSource>, TResult>, IScheduler)`).
        fn many_select_on<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> R + 'static,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::many_select::many_select(self.observable(), std::rc::Rc::new(selector), scheduler)
        }
    };
}
pub(crate) use many_select_methods;
