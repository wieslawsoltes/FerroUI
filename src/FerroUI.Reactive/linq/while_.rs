//! `While` (`Linq/Observable/While.cs`), and its overload of `QueryLanguage.Imperative.cs`.

use crate::linq::catch::{tail_recursive, SourceIterator, Sources, TailRecursiveMode};
use crate::linq::Observable;
use ferroui_system::{IObservable, ObservableError};
use std::rc::Rc;

pub(crate) type Condition = Rc<dyn Fn() -> Result<bool, ObservableError>>;

/// The sources of `While<TSource>` (`GetSources`): `source` again for as long as `condition`
/// holds, evaluated before each repetition; a condition that fails is an item that is its error.
pub(crate) fn repetitions<T: 'static>(condition: Condition, source: Rc<dyn IObservable<T>>) -> impl Iterator<Item = Result<Rc<dyn IObservable<T>>, ObservableError>> {
    std::iter::from_fn(move || match condition() {
        Ok(true) => Some(Ok(source.clone())),
        Ok(false) => None,
        Err(error) => Some(Err(error)),
    })
}

/// `While<TSource>`: the repetitions of `source`, one after another, run by a `ConcatSink<TSource>`.
pub(crate) fn while_observable<T: 'static>(condition: Condition, source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    let sources: Sources<T> = Rc::new(move || Ok(Box::new(repetitions(condition.clone(), source.clone())) as SourceIterator<T>));
    tail_recursive(sources, TailRecursiveMode::Concat)
}

impl Observable {
    /// The values of `source`, repeated for as long as `condition` holds when the previous
    /// repetition has completed, and at the subscription (`While<TSource>(Func<bool>, IObservable<TSource>)`).
    pub fn while_<T: 'static>(condition: impl Fn() -> bool + 'static, source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
        while_observable(Rc::new(move || Ok(condition())), source)
    }

    /// [`while_`](Self::while_) with a condition that may fail; a failure ends the sequence with its
    /// error (`While` with a condition that throws).
    pub fn try_while<T: 'static>(
        condition: impl Fn() -> Result<bool, ObservableError> + 'static,
        source: Rc<dyn IObservable<T>>,
    ) -> Rc<dyn IObservable<T>> {
        while_observable(Rc::new(condition), source)
    }
}
