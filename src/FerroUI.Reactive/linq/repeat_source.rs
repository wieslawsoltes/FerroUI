//! `Repeat` of a source (`Linq/QueryLanguage.Single.cs`: `Concat` of the source repeated). The
//! forms of a value, `Observable.Repeat(value)`, are in `Linq/Observable/Repeat.cs`.

use crate::linq::catch::{sources_of, tail_recursive, TailRecursiveMode};
use ferroui_system::IObservable;
use std::rc::Rc;

/// Subscribes to the source again each time it completes (`RepeatInfinite(source).Concat()`): the
/// loop of `Concat` over the sources, with its sink (`ConcatSink<TSource>`).
pub(crate) fn repeat<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    tail_recursive(sources_of(std::iter::repeat(source)), TailRecursiveMode::Concat)
}

/// Subscribes to the source `repeat_count` times, each time after it completes
/// (`Enumerable.Repeat(source, repeatCount).Concat()`).
pub(crate) fn repeat_count<T: 'static>(source: Rc<dyn IObservable<T>>, repeat_count: usize) -> Rc<dyn IObservable<T>> {
    tail_recursive(sources_of(std::iter::repeat(source).take(repeat_count)), TailRecursiveMode::Concat)
}

/// The methods of `Repeat` of a source (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! repeat_source_methods {
    () => {
        /// Subscribes to the source again each time it completes; an error ends the sequence
        /// (`Repeat<TSource>(IObservable<TSource>)`).
        fn repeat(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::repeat_source::repeat(self.observable())
        }

        /// Subscribes to the source `repeat_count` times, each time after it completes; an error
        /// ends the sequence (`Repeat<TSource>(IObservable<TSource>, int)`).
        fn repeat_count(&self, repeat_count: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::repeat_source::repeat_count(self.observable(), repeat_count)
        }
    };
}
pub(crate) use repeat_source_methods;
