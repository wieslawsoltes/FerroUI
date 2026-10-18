//! `Collect` (`Linq/Observable/Collect.cs`) and `Chunkify`, which upstream writes with `Collect`
//! (`Linq/QueryLanguage.Blocking.cs`).

use super::push_to_pull_adapter::{PullEnumerable, PullSink};
use crate::disposables::SingleAssignmentDisposableValue;
use ferroui_system::{IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::rc::Rc;

type Merge<T, R> = Rc<dyn Fn(R, T) -> Result<R, ObservableError>>;
type NewCollector<R> = Rc<dyn Fn(&R) -> Result<R, ObservableError>>;

struct State<R> {
    collector: Option<R>,
    error: Option<ObservableError>,
    has_completed: bool,
    done: bool,
}

/// The sink of `Collect` (`Collect<TSource, TResult>._`).
struct CollectSink<T, R> {
    merge: Merge<T, R>,
    get_new_collector: NewCollector<R>,
    state: RefCell<State<R>>,
    upstream: SingleAssignmentDisposableValue,
}

impl<T, R> IObserver<T> for CollectSink<T, R> {
    fn on_next(&self, value: T) {
        let collector = self.state.borrow_mut().collector.take();
        let Some(collector) = collector else { return };
        match (self.merge)(collector, value) {
            Ok(collector) => self.state.borrow_mut().collector = Some(collector),
            Err(error) => {
                self.state.borrow_mut().error = Some(error);
                self.upstream.dispose();
            }
        }
    }

    fn on_error(&self, error: ObservableError) {
        self.upstream.dispose();
        self.state.borrow_mut().error = Some(error);
    }

    fn on_completed(&self) {
        self.upstream.dispose();
        self.state.borrow_mut().has_completed = true;
    }
}

impl<T, R> PullSink<R> for CollectSink<T, R> {
    fn try_move_next(&self) -> Result<Option<R>, ObservableError> {
        let mut state = self.state.borrow_mut();
        if let Some(error) = state.error.clone() {
            state.collector = None;
            return Err(error);
        }
        if state.has_completed {
            if state.done {
                state.collector = None;
                return Ok(None);
            }
            state.done = true;
            return Ok(state.collector.take());
        }
        let current = state.collector.take();
        drop(state);
        let Some(current) = current else { return Ok(None) };
        match (self.get_new_collector)(&current) {
            Ok(collector) => {
                self.state.borrow_mut().collector = Some(collector);
                Ok(Some(current))
            }
            Err(error) => {
                self.upstream.dispose();
                Err(error)
            }
        }
    }

    fn upstream(&self) -> &SingleAssignmentDisposableValue {
        &self.upstream
    }
}

pub(crate) fn try_collect<T: 'static, R: 'static>(
    source: Rc<dyn IObservable<T>>,
    get_initial_collector: Rc<dyn Fn() -> Result<R, ObservableError>>,
    merge: Merge<T, R>,
    get_new_collector: NewCollector<R>,
) -> PullEnumerable<R> {
    PullEnumerable::new(source, move || {
        let collector = get_initial_collector()?;
        Ok(Rc::new(CollectSink {
            merge: merge.clone(),
            get_new_collector: get_new_collector.clone(),
            state: RefCell::new(State { collector: Some(collector), error: None, has_completed: false, done: false }),
            upstream: SingleAssignmentDisposableValue::new(),
        }))
    })
}

macro_rules! collect_methods {
    () => {
        /// A sequence whose each step gives the values collected since the previous step, merged
        /// into a collector that `new_collector` starts again for each step
        /// (`Collect(Func<TResult>, Func<TResult, TSource, TResult>)`).
        fn collect<R: 'static>(
            &self,
            new_collector: impl Fn() -> R + 'static,
            merge: impl Fn(R, T) -> R + 'static,
        ) -> crate::linq::PullEnumerable<R> {
            let new_collector = std::rc::Rc::new(new_collector);
            let first = new_collector.clone();
            crate::linq::collect::try_collect(
                self.observable(),
                std::rc::Rc::new(move || Ok(first())),
                std::rc::Rc::new(move |collector, value| Ok(merge(collector, value))),
                std::rc::Rc::new(move |_: &R| Ok(new_collector())),
            )
        }

        /// [`collect`](Self::collect) with collectors that may fail: an error ends the
        /// enumeration with that error.
        fn try_collect<R: 'static>(
            &self,
            new_collector: impl Fn() -> Result<R, ferroui_system::ObservableError> + 'static,
            merge: impl Fn(R, T) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> crate::linq::PullEnumerable<R> {
            let new_collector = std::rc::Rc::new(new_collector);
            let first = new_collector.clone();
            crate::linq::collect::try_collect(
                self.observable(),
                std::rc::Rc::new(move || first()),
                std::rc::Rc::new(merge),
                std::rc::Rc::new(move |_: &R| new_collector()),
            )
        }

        /// A sequence whose each step gives the values collected since the previous step: the
        /// first collector is `get_initial_collector`, and each step starts the next one with
        /// `get_new_collector` of the collector it gives
        /// (`Collect(Func<TResult>, Func<TResult, TSource, TResult>, Func<TResult, TResult>)`).
        fn collect_with<R: 'static>(
            &self,
            get_initial_collector: impl Fn() -> R + 'static,
            merge: impl Fn(R, T) -> R + 'static,
            get_new_collector: impl Fn(&R) -> R + 'static,
        ) -> crate::linq::PullEnumerable<R> {
            crate::linq::collect::try_collect(
                self.observable(),
                std::rc::Rc::new(move || Ok(get_initial_collector())),
                std::rc::Rc::new(move |collector, value| Ok(merge(collector, value))),
                std::rc::Rc::new(move |collector: &R| Ok(get_new_collector(collector))),
            )
        }

        /// [`collect_with`](Self::collect_with) with collectors that may fail: an error of the
        /// first collector is the error of `get_enumerator`, before the subscription; another
        /// error ends the enumeration with that error.
        fn try_collect_with<R: 'static>(
            &self,
            get_initial_collector: impl Fn() -> Result<R, ferroui_system::ObservableError> + 'static,
            merge: impl Fn(R, T) -> Result<R, ferroui_system::ObservableError> + 'static,
            get_new_collector: impl Fn(&R) -> Result<R, ferroui_system::ObservableError> + 'static,
        ) -> crate::linq::PullEnumerable<R> {
            crate::linq::collect::try_collect(
                self.observable(),
                std::rc::Rc::new(get_initial_collector),
                std::rc::Rc::new(merge),
                std::rc::Rc::new(get_new_collector),
            )
        }

        /// A sequence whose each step gives the values that arrived since the previous step
        /// (`Chunkify`).
        fn chunkify(&self) -> crate::linq::PullEnumerable<Vec<T>> {
            crate::linq::collect::try_collect(
                self.observable(),
                std::rc::Rc::new(|| Ok(Vec::new())),
                std::rc::Rc::new(|mut list: Vec<T>, value| {
                    list.push(value);
                    Ok(list)
                }),
                std::rc::Rc::new(|_: &Vec<T>| Ok(Vec::new())),
            )
        }
    };
}
pub(crate) use collect_methods;
