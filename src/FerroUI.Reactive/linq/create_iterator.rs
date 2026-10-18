//! `Create` of an iterator (`Linq/QueryLanguageEx.cs`, the experimental `ObservableEx.Create`):
//! a sequence driven by an iterator method, which notifies the observer itself and yields the
//! sequences to wait for, one after another (`CreateWithEnumerableObservable<TResult>`,
//! `CreateWithOnlyEnumerableObservable<TResult>` and `TerminalOnlyObserver<TResult>`).

use crate::disposables::Disposable;
use crate::internal::subscribe_auto_detach;
use crate::linq::catch::{tail_recursive, SourceIterator, Sources, TailRecursiveMode};
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;

/// The iterator method of a subscription: given the observer, the enumerable of the sequences to
/// wait for (`Func<IObserver<TResult>, IEnumerable<IObservable<object>>>`); an `Err` is the method
/// itself throwing.
type IteratorMethod<R, X> = Rc<dyn Fn(Rc<dyn IObserver<R>>) -> Result<Sources<X>, ObservableError>>;

/// The sources of an enumerable that is enumerated once: `Concat` of the enumerable that the
/// iterator method returned asks for its enumerator once, when it is subscribed to (`Run`). The
/// enumerable is turned into its enumerator only then, so the iteration stays lazy: each step runs
/// when the previous sequence completes.
fn once_sources<X: 'static, I>(sources: I) -> Sources<X>
where
    I: IntoIterator<Item = Result<Rc<dyn IObservable<X>>, ObservableError>> + 'static,
    I::IntoIter: 'static,
{
    let sources = RefCell::new(Some(sources));
    Rc::new(move || {
        let sources = sources.borrow_mut().take();
        Ok(match sources {
            Some(sources) => Box::new(sources.into_iter()) as SourceIterator<X>,
            None => Box::new(std::iter::empty()) as SourceIterator<X>,
        })
    })
}

/// The observer of the concatenation of the yielded sequences (`TerminalOnlyObserver<TResult>`):
/// their values are ignored, their end is the end of the sequence.
struct TerminalOnlyObserver<R, X> {
    observer: Rc<dyn IObserver<R>>,
    _values: PhantomData<fn(X)>,
}

impl<R, X> IObserver<X> for TerminalOnlyObserver<R, X> {
    fn on_next(&self, _value: X) {
        // deliberately ignored
    }

    fn on_error(&self, error: ObservableError) {
        self.observer.on_error(error);
    }

    fn on_completed(&self) {
        self.observer.on_completed();
    }
}

/// The observable of `Create` of an iterator (`CreateWithEnumerableObservable<TResult>`, an
/// `ObservableBase<TResult>`; `CreateWithOnlyEnumerableObservable<TResult>` is the same with an
/// iterator method that ignores the observer).
struct CreateWithEnumerableObservable<R, X> {
    iterator_method: IteratorMethod<R, X>,
}

impl<R: 'static, X: 'static> IObservable<R> for CreateWithEnumerableObservable<R, X> {
    fn subscribe(&self, observer: Rc<dyn IObserver<R>>) -> Rc<dyn IDisposable> {
        let iterator_method = self.iterator_method.clone();
        subscribe_auto_detach(observer, move |observer: Rc<dyn IObserver<R>>| -> Rc<dyn IDisposable> {
            match iterator_method(observer.clone()) {
                // `_iteratorMethod(observer).Concat().Subscribe(new TerminalOnlyObserver<TResult>(observer))`.
                Ok(sources) => {
                    let terminal: Rc<dyn IObserver<X>> = Rc::new(TerminalOnlyObserver { observer, _values: PhantomData });
                    tail_recursive(sources, TailRecursiveMode::Concat).subscribe(terminal)
                }
                // The exception of `SubscribeCore`, given to the observer (`AutoDetachObserver.Fail`).
                Err(error) => {
                    observer.on_error(error);
                    Disposable::empty()
                }
            }
        })
    }
}

fn create_with_enumerable<R: 'static, X: 'static>(iterator_method: IteratorMethod<R, X>) -> Rc<dyn IObservable<R>> {
    Rc::new(CreateWithEnumerableObservable { iterator_method })
}

impl crate::linq::Observable {
    /// The sequence of the values that `iterator_method` sends to the observer it is given; the
    /// sequences it yields are subscribed to one after another, and only their end (an error, or
    /// the completion of the last one) is notified
    /// (`ObservableEx.Create<TResult>(Func<IObserver<TResult>, IEnumerable<IObservable<object>>>)`).
    /// The next sequence is asked of the iterator only when the previous one completes.
    ///
    /// Deviation (DEVIATIONS.md, Reactive): upstream's yielded sequences are `IObservable<object>`;
    /// the port has no common object type, so they are sequences of one type `X`, a parameter of
    /// the method (`()` when the values do not matter).
    pub fn create_iterator<R: 'static, X: 'static, I>(iterator_method: impl Fn(Rc<dyn IObserver<R>>) -> I + 'static) -> Rc<dyn IObservable<R>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<X>>> + 'static,
        I::IntoIter: 'static,
    {
        create_with_enumerable(Rc::new(move |observer: Rc<dyn IObserver<R>>| {
            Ok::<_, ObservableError>(once_sources::<X, _>(iterator_method(observer).into_iter().map(Ok::<_, ObservableError>)))
        }))
    }

    /// [`create_iterator`](Self::create_iterator) with an iterator method that may fail: an `Err`
    /// returned by `iterator_method` is the method throwing, an `Err` item is the iterator
    /// throwing; either ends the sequence with that error
    /// (`ObservableEx.Create<TResult>(Func<IObserver<TResult>, IEnumerable<IObservable<object>>>)`).
    ///
    /// Deviation (DEVIATIONS.md, Reactive): the yielded sequences are of one type `X`, where
    /// upstream's are `IObservable<object>`.
    pub fn try_create_iterator<R: 'static, X: 'static, I>(
        iterator_method: impl Fn(Rc<dyn IObserver<R>>) -> Result<I, ObservableError> + 'static,
    ) -> Rc<dyn IObservable<R>>
    where
        I: IntoIterator<Item = Result<Rc<dyn IObservable<X>>, ObservableError>> + 'static,
        I::IntoIter: 'static,
    {
        create_with_enumerable(Rc::new(move |observer: Rc<dyn IObserver<R>>| iterator_method(observer).map(once_sources::<X, I>)))
    }

    /// The end of the sequences that `iterator_method` yields, subscribed to one after another
    /// (`ObservableEx.Create(Func<IEnumerable<IObservable<object>>>)`). As upstream's, the
    /// sequence notifies no value (its observer is a `TerminalOnlyObserver<Unit>`), only the error
    /// of a sequence or the completion of the last one. The next sequence is asked of the
    /// iterator only when the previous one completes.
    ///
    /// Deviation (DEVIATIONS.md, Reactive): the yielded sequences are of one type `X`, where
    /// upstream's are `IObservable<object>`.
    pub fn create_iterator_void<X: 'static, I>(iterator_method: impl Fn() -> I + 'static) -> Rc<dyn IObservable<()>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<X>>> + 'static,
        I::IntoIter: 'static,
    {
        create_with_enumerable(Rc::new(move |_observer: Rc<dyn IObserver<()>>| {
            Ok::<_, ObservableError>(once_sources::<X, _>(iterator_method().into_iter().map(Ok::<_, ObservableError>)))
        }))
    }

    /// [`create_iterator_void`](Self::create_iterator_void) with an iterator method that may
    /// fail: an `Err` returned by `iterator_method` is the method throwing, an `Err` item is the
    /// iterator throwing; either ends the sequence with that error
    /// (`ObservableEx.Create(Func<IEnumerable<IObservable<object>>>)`).
    ///
    /// Deviation (DEVIATIONS.md, Reactive): the yielded sequences are of one type `X`, where
    /// upstream's are `IObservable<object>`.
    pub fn try_create_iterator_void<X: 'static, I>(iterator_method: impl Fn() -> Result<I, ObservableError> + 'static) -> Rc<dyn IObservable<()>>
    where
        I: IntoIterator<Item = Result<Rc<dyn IObservable<X>>, ObservableError>> + 'static,
        I::IntoIter: 'static,
    {
        create_with_enumerable(Rc::new(move |_observer: Rc<dyn IObserver<()>>| iterator_method().map(once_sources::<X, I>)))
    }
}
