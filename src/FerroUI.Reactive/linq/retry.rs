//! `Retry` (`Linq/QueryLanguage.Single.cs`: `Catch` of the source repeated).

use crate::linq::catch::{catch_sources, sources_of};
use ferroui_system::IObservable;
use std::rc::Rc;

/// Subscribes to the source again each time it fails (`RepeatInfinite(source).Catch()`).
pub(crate) fn retry<T: 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<T>> {
    catch_sources(sources_of(std::iter::repeat(source)))
}

/// Subscribes to the source up to `retry_count` times while it fails; the last error ends the
/// sequence (`Enumerable.Repeat(source, retryCount).Catch()`).
pub(crate) fn retry_count<T: 'static>(source: Rc<dyn IObservable<T>>, retry_count: usize) -> Rc<dyn IObservable<T>> {
    catch_sources(sources_of(std::iter::repeat(source).take(retry_count)))
}

/// The methods of `Retry` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! retry_methods {
    () => {
        /// Subscribes to the source again each time it fails, until it completes (`Retry()`).
        fn retry(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::retry::retry(self.observable())
        }

        /// Subscribes to the source up to `retry_count` times while it fails; the last error ends
        /// the sequence (`Retry(int)`).
        fn retry_count(&self, retry_count: usize) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            crate::linq::retry::retry_count(self.observable(), retry_count)
        }
    };
}
pub(crate) use retry_methods;
