//! `For` (`Linq/Observable/For.cs`), and its overload of `QueryLanguage.Imperative.cs`.

use crate::linq::catch::{tail_recursive, SourceIterator, Sources, TailRecursiveMode};
use crate::linq::Observable;
use ferroui_system::{IObservable, ObservableError};
use std::rc::Rc;

type ResultSelector<S, R> = Rc<dyn Fn(S) -> Result<Rc<dyn IObservable<R>>, ObservableError>>;

/// `For<TSource, TResult>`: the sequences of the items of `source`, one after another
/// (`GetSources`, run by a `ConcatSink<TResult>`). The items are enumerated, and the sequence of
/// each made, only when the previous sequence has completed; an item that is an error, or a
/// selector that fails, ends the sequence with the error.
pub(crate) fn for_observable<S: 'static, R: 'static, I>(source: I, result_selector: ResultSelector<S, R>) -> Rc<dyn IObservable<R>>
where
    I: IntoIterator<Item = Result<S, ObservableError>> + Clone + 'static,
    I::IntoIter: 'static,
{
    let sources: Sources<R> = Rc::new(move || {
        let selector = result_selector.clone();
        let iterator = source.clone().into_iter().map(move |item| item.and_then(|item| selector(item)));
        Ok(Box::new(iterator) as SourceIterator<R>)
    });
    tail_recursive(sources, TailRecursiveMode::Concat)
}

impl Observable {
    /// The values of the sequences `result_selector` makes of the items of `source`, one sequence
    /// after another (`For<TSource, TResult>(IEnumerable<TSource>, Func<TSource, IObservable<TResult>>)`).
    /// Each subscription enumerates `source` anew.
    pub fn for_<S: 'static, R: 'static, I>(
        source: I,
        result_selector: impl Fn(S) -> Rc<dyn IObservable<R>> + 'static,
    ) -> Rc<dyn IObservable<R>>
    where
        I: IntoIterator<Item = S> + Clone + 'static,
        I::IntoIter: 'static,
    {
        let items = OkItems(source);
        for_observable(items, Rc::new(move |item| Ok(result_selector(item))))
    }

    /// [`for_`](Self::for_) over items whose enumeration may fail (an item that is an error is a
    /// `MoveNext` that throws) and with a selector that may fail; a failure ends the sequence with
    /// its error.
    pub fn try_for<S: 'static, R: 'static, I>(
        source: I,
        result_selector: impl Fn(S) -> Result<Rc<dyn IObservable<R>>, ObservableError> + 'static,
    ) -> Rc<dyn IObservable<R>>
    where
        I: IntoIterator<Item = Result<S, ObservableError>> + Clone + 'static,
        I::IntoIter: 'static,
    {
        for_observable(source, Rc::new(result_selector))
    }
}

/// The items of an enumerable that cannot fail, as items of one that may.
#[derive(Clone)]
struct OkItems<I>(I);

impl<I: IntoIterator> IntoIterator for OkItems<I> {
    type Item = Result<I::Item, ObservableError>;
    type IntoIter = std::iter::Map<I::IntoIter, fn(I::Item) -> Result<I::Item, ObservableError>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter().map(Ok as fn(I::Item) -> Result<I::Item, ObservableError>)
    }
}
