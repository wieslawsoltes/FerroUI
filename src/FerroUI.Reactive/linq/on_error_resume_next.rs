//! `OnErrorResumeNext` (`Linq/Observable/OnErrorResumeNext.cs`).

use crate::linq::catch::{sources_of, tail_recursive, try_sources_of, Sources, TailRecursiveMode};
use crate::linq::Observable;
use ferroui_system::{IObservable, ObservableError};
use std::rc::Rc;

/// Continues with the next source when one fails or completes; the end of the sources completes
/// (`OnErrorResumeNext<TSource>`, the sink a `TailRecursiveSink<TSource>`).
pub(crate) fn on_error_resume_next_sources<T: 'static>(sources: Sources<T>) -> Rc<dyn IObservable<T>> {
    tail_recursive(sources, TailRecursiveMode::OnErrorResumeNext)
}

impl Observable {
    /// Each of `sources` in turn, whether it fails or completes; the sequence completes after the
    /// last one (`OnErrorResumeNext(IEnumerable<IObservable<TSource>>)`, `OnErrorResumeNext(params
    /// IObservable<TSource>[])`). The sources are enumerated again for each subscription.
    pub fn on_error_resume_next_all<T: 'static, I>(sources: I) -> Rc<dyn IObservable<T>>
    where
        I: IntoIterator<Item = Rc<dyn IObservable<T>>> + Clone + 'static,
        I::IntoIter: 'static,
    {
        on_error_resume_next_sources(sources_of(sources))
    }

    /// [`on_error_resume_next_all`](Self::on_error_resume_next_all) over sources whose enumeration
    /// may fail: `sources` starts an enumeration (`GetEnumerator`, which fails with an `Err`), and
    /// each item is the next source or the error of the step (`MoveNext` that throws). A failure
    /// ends the sequence with its error.
    pub fn try_on_error_resume_next_all<T: 'static, I>(sources: impl Fn() -> Result<I, ObservableError> + 'static) -> Rc<dyn IObservable<T>>
    where
        I: IntoIterator<Item = Result<Rc<dyn IObservable<T>>, ObservableError>>,
        I::IntoIter: 'static,
    {
        on_error_resume_next_sources(try_sources_of(sources))
    }
}

/// The methods of `OnErrorResumeNext` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! on_error_resume_next_methods {
    () => {
        /// The source, then `second`, whether the source fails or completes
        /// (`OnErrorResumeNext(IObservable<TSource>, IObservable<TSource>)`).
        fn on_error_resume_next(
            &self,
            second: std::rc::Rc<dyn ferroui_system::IObservable<T>>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::Observable::on_error_resume_next_all(vec![self.observable(), second])
        }
    };
}
pub(crate) use on_error_resume_next_methods;
