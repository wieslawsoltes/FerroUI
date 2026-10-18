//! `Publish` and `PublishLast` (the bindings of `Linq/QueryLanguage.Binding.cs`): `Multicast`
//! through a `Subject<T>`, a `BehaviorSubject<T>` or an `AsyncSubject<T>`.

use crate::subjects::{AsyncSubject, BehaviorSubject, IConnectableObservable, Subject};
use ferroui_system::IObservable;
use std::rc::Rc;

/// `Publish(IObservable<TSource>)`: `Multicast(new Subject<TSource>())`.
pub(crate) fn publish<T: Clone + 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IConnectableObservable<T>> {
    super::multicast::multicast::<T, T>(source, Subject::new())
}

/// `Publish(IObservable<TSource>, Func<IObservable<TSource>, IObservable<TResult>>)`:
/// `Multicast(() => new Subject<TSource>(), selector)`.
pub(crate) fn publish_with<T: Clone + 'static, R: 'static>(
    source: Rc<dyn IObservable<T>>,
    selector: impl Fn(Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<R>> + 'static,
) -> Rc<dyn IObservable<R>> {
    super::multicast::multicast_with_factory::<T, T, R, Subject<T>>(source, Subject::new, selector)
}

/// `Publish(IObservable<TSource>, TSource)`: `Multicast(new BehaviorSubject<TSource>(initialValue))`.
pub(crate) fn publish_with_initial<T: Clone + 'static>(source: Rc<dyn IObservable<T>>, initial_value: T) -> Rc<dyn IConnectableObservable<T>> {
    super::multicast::multicast::<T, T>(source, BehaviorSubject::new(initial_value))
}

/// `Publish(IObservable<TSource>, Func<IObservable<TSource>, IObservable<TResult>>, TSource)`:
/// `Multicast(() => new BehaviorSubject<TSource>(initialValue), selector)`.
pub(crate) fn publish_with_initial_with<T: Clone + 'static, R: 'static>(
    source: Rc<dyn IObservable<T>>,
    selector: impl Fn(Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<R>> + 'static,
    initial_value: T,
) -> Rc<dyn IObservable<R>> {
    super::multicast::multicast_with_factory::<T, T, R, BehaviorSubject<T>>(source, move || BehaviorSubject::new(initial_value.clone()), selector)
}

/// `PublishLast(IObservable<TSource>)`: `Multicast(new AsyncSubject<TSource>())`.
pub(crate) fn publish_last<T: Clone + 'static>(source: Rc<dyn IObservable<T>>) -> Rc<dyn IConnectableObservable<T>> {
    super::multicast::multicast::<T, T>(source, AsyncSubject::new())
}

/// `PublishLast(IObservable<TSource>, Func<IObservable<TSource>, IObservable<TResult>>)`:
/// `Multicast(() => new AsyncSubject<TSource>(), selector)`.
pub(crate) fn publish_last_with<T: Clone + 'static, R: 'static>(
    source: Rc<dyn IObservable<T>>,
    selector: impl Fn(Rc<dyn IObservable<T>>) -> Rc<dyn IObservable<R>> + 'static,
) -> Rc<dyn IObservable<R>> {
    super::multicast::multicast_with_factory::<T, T, R, AsyncSubject<T>>(source, AsyncSubject::new, selector)
}

/// The methods of `Publish` and `PublishLast` (provided methods of
/// [`ObservableExt`](super::ObservableExt)).
macro_rules! publish_methods {
    () => {
        /// Shares the sequence through a subject: observers see the values the sequence produces
        /// while it is connected (`Publish()`).
        fn publish(&self) -> std::rc::Rc<dyn crate::subjects::IConnectableObservable<T>>
        where
            T: Clone,
        {
            crate::linq::publish::publish(self.observable())
        }

        /// On each subscription, shares the sequence through a subject with the sequence `selector`
        /// makes of it (`Publish(Func<IObservable<TSource>, IObservable<TResult>>)`).
        fn publish_with<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::publish::publish_with(self.observable(), selector)
        }

        /// Shares the sequence through a behavior subject that starts with `initial_value`:
        /// observers see the latest value first (`Publish(TSource)`).
        fn publish_with_initial(&self, initial_value: T) -> std::rc::Rc<dyn crate::subjects::IConnectableObservable<T>>
        where
            T: Clone,
        {
            crate::linq::publish::publish_with_initial(self.observable(), initial_value)
        }

        /// On each subscription, shares the sequence through a behavior subject that starts with
        /// `initial_value` with the sequence `selector` makes of it
        /// (`Publish(Func<IObservable<TSource>, IObservable<TResult>>, TSource)`).
        fn publish_with_initial_with<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> + 'static,
            initial_value: T,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::publish::publish_with_initial_with(self.observable(), selector, initial_value)
        }

        /// Shares the sequence through an async subject: observers see only the last value and the
        /// end (`PublishLast()`).
        fn publish_last(&self) -> std::rc::Rc<dyn crate::subjects::IConnectableObservable<T>>
        where
            T: Clone,
        {
            crate::linq::publish::publish_last(self.observable())
        }

        /// On each subscription, shares the sequence through an async subject with the sequence
        /// `selector` makes of it (`PublishLast(Func<IObservable<TSource>, IObservable<TResult>>)`).
        fn publish_last_with<R: 'static>(
            &self,
            selector: impl Fn(std::rc::Rc<dyn ferroui_system::IObservable<T>>) -> std::rc::Rc<dyn ferroui_system::IObservable<R>> + 'static,
        ) -> std::rc::Rc<dyn ferroui_system::IObservable<R>>
        where
            T: Clone,
        {
            crate::linq::publish::publish_last_with(self.observable(), selector)
        }
    };
}
pub(crate) use publish_methods;
