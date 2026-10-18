//! `Case` (`Linq/Observable/Case.cs`), and its overloads of `QueryLanguage.Imperative.cs`.

use crate::concurrency::IScheduler;
use crate::internal::subscribe_sink;
use crate::linq::creation::IdentitySink;
use crate::linq::Observable;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

type Selector<K> = Rc<dyn Fn() -> Result<K, ObservableError>>;

/// `Case<TValue, TResult>`: each subscription evaluates the selector and subscribes to the sequence
/// of its key, or to the default sequence when no sequence has the key (`Eval`); a selector that
/// fails ends the sequence with its error.
struct CaseObservable<K, T> {
    selector: Selector<K>,
    sources: Rc<HashMap<K, Rc<dyn IObservable<T>>>>,
    default_source: Rc<dyn IObservable<T>>,
}

impl<K: Eq + Hash + 'static, T: 'static> IObservable<T> for CaseObservable<K, T> {
    fn subscribe(&self, observer: Rc<dyn IObserver<T>>) -> Rc<dyn IDisposable> {
        let selector = self.selector.clone();
        let (sources, default_source) = (self.sources.clone(), self.default_source.clone());
        subscribe_sink(observer, IdentitySink::new, move |sink: &Rc<IdentitySink<T>>| {
            // `Case<TValue, TResult>._.Run`.
            let result = match selector() {
                Ok(key) => sources.get(&key).cloned().unwrap_or(default_source),
                Err(error) => {
                    sink.sink.forward_on_error(error);
                    return;
                }
            };
            sink.run(result);
        })
    }
}

pub(crate) fn case_observable<K: Eq + Hash + 'static, T: 'static>(
    selector: Selector<K>,
    sources: HashMap<K, Rc<dyn IObservable<T>>>,
    default_source: Rc<dyn IObservable<T>>,
) -> Rc<dyn IObservable<T>> {
    Rc::new(CaseObservable { selector, sources: Rc::new(sources), default_source })
}

impl Observable {
    /// The values of the sequence of `sources` whose key `selector` returns at the subscription, or
    /// of `default_source` when none has the key
    /// (`Case<TValue, TResult>(Func<TValue>, IDictionary<TValue, IObservable<TResult>>, IObservable<TResult>)`).
    pub fn case_with_default<K: Eq + Hash + 'static, T: 'static>(
        selector: impl Fn() -> K + 'static,
        sources: HashMap<K, Rc<dyn IObservable<T>>>,
        default_source: Rc<dyn IObservable<T>>,
    ) -> Rc<dyn IObservable<T>> {
        case_observable(Rc::new(move || Ok(selector())), sources, default_source)
    }

    /// [`case_with_default`](Self::case_with_default) whose default is an empty sequence
    /// (`Case<TValue, TResult>(Func<TValue>, IDictionary<TValue, IObservable<TResult>>)`).
    pub fn case<K: Eq + Hash + 'static, T: 'static>(
        selector: impl Fn() -> K + 'static,
        sources: HashMap<K, Rc<dyn IObservable<T>>>,
    ) -> Rc<dyn IObservable<T>> {
        Self::case_with_default(selector, sources, Observable::empty())
    }

    /// [`case_with_default`](Self::case_with_default) whose default is an empty sequence that ends
    /// on `scheduler` (`Case<TValue, TResult>(Func<TValue>, IDictionary<TValue, IObservable<TResult>>, IScheduler)`).
    pub fn case_on<K: Eq + Hash + 'static, T: 'static>(
        selector: impl Fn() -> K + 'static,
        sources: HashMap<K, Rc<dyn IObservable<T>>>,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        Self::case_with_default(selector, sources, Observable::empty_on(scheduler))
    }

    /// [`case_with_default`](Self::case_with_default) with a selector that may fail; a failure ends
    /// the sequence with its error (`Case` with a selector that throws).
    pub fn try_case_with_default<K: Eq + Hash + 'static, T: 'static>(
        selector: impl Fn() -> Result<K, ObservableError> + 'static,
        sources: HashMap<K, Rc<dyn IObservable<T>>>,
        default_source: Rc<dyn IObservable<T>>,
    ) -> Rc<dyn IObservable<T>> {
        case_observable(Rc::new(selector), sources, default_source)
    }

    /// [`case`](Self::case) with a selector that may fail.
    pub fn try_case<K: Eq + Hash + 'static, T: 'static>(
        selector: impl Fn() -> Result<K, ObservableError> + 'static,
        sources: HashMap<K, Rc<dyn IObservable<T>>>,
    ) -> Rc<dyn IObservable<T>> {
        Self::try_case_with_default(selector, sources, Observable::empty())
    }

    /// [`case_on`](Self::case_on) with a selector that may fail.
    pub fn try_case_on<K: Eq + Hash + 'static, T: 'static>(
        selector: impl Fn() -> Result<K, ObservableError> + 'static,
        sources: HashMap<K, Rc<dyn IObservable<T>>>,
        scheduler: Rc<dyn IScheduler>,
    ) -> Rc<dyn IObservable<T>> {
        Self::try_case_with_default(selector, sources, Observable::empty_on(scheduler))
    }
}
