//! `Synchronize` (`Linq/Observable/Synchronize.cs`, `Concurrency/Synchronization.Synchronize.cs`).
//!
//! Upstream wraps each notification to the observer in a lock, so that a source that notifies
//! from several threads at once delivers its notifications one at a time.
//!
//! Deviation (DEVIATIONS.md, Reactive): the port is single-threaded; a sequence and its observers
//! live on one thread, so its notifications cannot overlap and there is nothing to lock.
//! `synchronize()` is the source itself. `Synchronize(IObservable<TSource>, object gate)`, which
//! shares a lock object between sequences, is not ported: the gate has no meaning on one thread.

/// The methods of `Synchronize` (provided methods of [`ObservableExt`](super::ObservableExt)).
macro_rules! synchronize_methods {
    () => {
        /// The sequence whose notifications reach the observer one at a time (`Synchronize()`).
        ///
        /// Deviation (DEVIATIONS.md, Reactive): the port is single-threaded, so notifications of a
        /// sequence never overlap; this is the sequence itself.
        fn synchronize(&self) -> std::rc::Rc<dyn ferroui_system::IObservable<T>> {
            self.observable()
        }
    };
}
pub(crate) use synchronize_methods;
