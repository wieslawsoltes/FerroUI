//! `Replay` (the bindings of `Linq/QueryLanguage.Binding.cs`): `Multicast` through a
//! `ReplaySubject<T>`, with its buffer size, window and scheduler.

use crate::subjects::{IConnectableObservable, ReplaySubject};
use ferroui_system::IObservable;
use std::rc::Rc;

/// `Replay(...)`: `Multicast(new ReplaySubject<TSource>(...))`, with the subject `subject`.
pub(crate) fn replay<T: Clone + 'static>(source: Rc<dyn IObservable<T>>, subject: Rc<ReplaySubject<T>>) -> Rc<dyn IConnectableObservable<T>> {
    super::multicast::multicast::<T, T>(source, subject)
}

/// `Replay(Func<IObservable<TSource>, IObservable<TResult>>, ...)`:
/// `Multicast(() => new ReplaySubject<TSource>(...), selector)`, with the subjects of `factory`.
pub(crate) fn replay_with<T: Clone + 'static, R: 'static>(
    source: Rc<dyn IObservable<T>>,
    factory: impl Fn() -> Rc<ReplaySubject<T>> + 'static,
    selector: impl Fn(Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<R>> + 'static,
) -> Rc<dyn IObservable<R>> {
    super::multicast::multicast_with_factory::<T, T, R, ReplaySubject<T>>(source, factory, selector)
}

/// The methods of `Replay` (provided methods of [`ObservableExt`](super::ObservableExt)).
///
/// The forms without a selector return a connectable observable; the forms `replay_with*` take a
/// selector. `_buffer` bounds the number of values replayed, `_window` their age, and `_on` names
/// the scheduler of the subject.
macro_rules! replay_methods {
    () => {
        /// Shares the sequence through a replay subject: observers see every value the sequence
        /// has produced while connected, then the next ones (`Replay()`).
        fn replay(&self) -> std::rc::Rc<dyn crate::subjects::IConnectableObservable<T>>
        where
            T: Clone,
        {
            crate::linq::replay::replay(self.observable(), crate::subjects::ReplaySubject::new())
        }

        /// [`replay`](Self::replay), replaying on `scheduler` (`Replay(IScheduler)`).
        fn replay_on(&self, scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>) -> std::rc::Rc<dyn crate::subjects::IConnectableObservable<T>>
        where
            T: Clone,
        {
            crate::linq::replay::replay(self.observable(), crate::subjects::ReplaySubject::with_scheduler(scheduler))
        }

        /// On each subscription, shares the sequence through a replay subject with the sequence
        /// `selector` makes of it (`Replay(Func<IObservable<TSource>, IObservable<TResult>>)`).
        fn replay_with<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::replay::replay_with(self.observable(), crate::subjects::ReplaySubject::new, selector)
        }

        /// [`replay_with`](Self::replay_with), replaying on `scheduler`
        /// (`Replay(Func<IObservable<TSource>, IObservable<TResult>>, IScheduler)`).
        fn replay_with_on<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> + 'static,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::replay::replay_with(self.observable(), move || crate::subjects::ReplaySubject::with_scheduler(scheduler.clone()), selector)
        }

        /// [`replay`](Self::replay) of the values of the last `window` (`Replay(TimeSpan)`).
        fn replay_window(&self, window: std::time::Duration) -> std::rc::Rc<dyn crate::subjects::IConnectableObservable<T>>
        where
            T: Clone,
        {
            crate::linq::replay::replay(self.observable(), crate::subjects::ReplaySubject::with_window(window))
        }

        /// [`replay_window`](Self::replay_window), replaying on `scheduler` (`Replay(TimeSpan, IScheduler)`).
        fn replay_window_on(
            &self,
            window: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn crate::subjects::IConnectableObservable<T>>
        where
            T: Clone,
        {
            crate::linq::replay::replay(self.observable(), crate::subjects::ReplaySubject::with_window_and_scheduler(window, scheduler))
        }

        /// [`replay_with`](Self::replay_with) of the values of the last `window`
        /// (`Replay(Func<IObservable<TSource>, IObservable<TResult>>, TimeSpan)`).
        fn replay_with_window<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> + 'static,
            window: std::time::Duration,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::replay::replay_with(self.observable(), move || crate::subjects::ReplaySubject::with_window(window), selector)
        }

        /// [`replay_with_window`](Self::replay_with_window), replaying on `scheduler`
        /// (`Replay(Func<IObservable<TSource>, IObservable<TResult>>, TimeSpan, IScheduler)`).
        fn replay_with_window_on<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> + 'static,
            window: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::replay::replay_with(
                self.observable(),
                move || crate::subjects::ReplaySubject::with_window_and_scheduler(window, scheduler.clone()),
                selector,
            )
        }

        /// [`replay`](Self::replay) of the last `buffer_size` values (`Replay(int)`).
        fn replay_buffer(&self, buffer_size: usize) -> std::rc::Rc<dyn crate::subjects::IConnectableObservable<T>>
        where
            T: Clone,
        {
            crate::linq::replay::replay(self.observable(), crate::subjects::ReplaySubject::with_buffer_size(buffer_size))
        }

        /// [`replay_buffer`](Self::replay_buffer), replaying on `scheduler` (`Replay(int, IScheduler)`).
        fn replay_buffer_on(
            &self,
            buffer_size: usize,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn crate::subjects::IConnectableObservable<T>>
        where
            T: Clone,
        {
            crate::linq::replay::replay(self.observable(), crate::subjects::ReplaySubject::with_buffer_size_and_scheduler(buffer_size, scheduler))
        }

        /// [`replay_with`](Self::replay_with) of the last `buffer_size` values
        /// (`Replay(Func<IObservable<TSource>, IObservable<TResult>>, int)`).
        fn replay_with_buffer<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> + 'static,
            buffer_size: usize,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::replay::replay_with(self.observable(), move || crate::subjects::ReplaySubject::with_buffer_size(buffer_size), selector)
        }

        /// [`replay_with_buffer`](Self::replay_with_buffer), replaying on `scheduler`
        /// (`Replay(Func<IObservable<TSource>, IObservable<TResult>>, int, IScheduler)`).
        fn replay_with_buffer_on<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> + 'static,
            buffer_size: usize,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::replay::replay_with(
                self.observable(),
                move || crate::subjects::ReplaySubject::with_buffer_size_and_scheduler(buffer_size, scheduler.clone()),
                selector,
            )
        }

        /// [`replay`](Self::replay) of at most the last `buffer_size` values of the last `window`
        /// (`Replay(int, TimeSpan)`).
        fn replay_buffer_window(&self, buffer_size: usize, window: std::time::Duration) -> std::rc::Rc<dyn crate::subjects::IConnectableObservable<T>>
        where
            T: Clone,
        {
            crate::linq::replay::replay(self.observable(), crate::subjects::ReplaySubject::with_buffer_size_and_window(buffer_size, window))
        }

        /// [`replay_buffer_window`](Self::replay_buffer_window), replaying on `scheduler`
        /// (`Replay(int, TimeSpan, IScheduler)`).
        fn replay_buffer_window_on(
            &self,
            buffer_size: usize,
            window: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn crate::subjects::IConnectableObservable<T>>
        where
            T: Clone,
        {
            crate::linq::replay::replay(
                self.observable(),
                crate::subjects::ReplaySubject::with_buffer_size_window_and_scheduler(buffer_size, window, scheduler),
            )
        }

        /// [`replay_with`](Self::replay_with) of at most the last `buffer_size` values of the last
        /// `window` (`Replay(Func<IObservable<TSource>, IObservable<TResult>>, int, TimeSpan)`).
        fn replay_with_buffer_window<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> + 'static,
            buffer_size: usize,
            window: std::time::Duration,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::replay::replay_with(
                self.observable(),
                move || crate::subjects::ReplaySubject::with_buffer_size_and_window(buffer_size, window),
                selector,
            )
        }

        /// [`replay_with_buffer_window`](Self::replay_with_buffer_window), replaying on `scheduler`
        /// (`Replay(Func<IObservable<TSource>, IObservable<TResult>>, int, TimeSpan, IScheduler)`).
        fn replay_with_buffer_window_on<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> + 'static,
            buffer_size: usize,
            window: std::time::Duration,
            scheduler: std::rc::Rc<dyn crate::concurrency::IScheduler>,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::replay::replay_with(
                self.observable(),
                move || crate::subjects::ReplaySubject::with_buffer_size_window_and_scheduler(buffer_size, window, scheduler.clone()),
                selector,
            )
        }
    };
}
pub(crate) use replay_methods;
