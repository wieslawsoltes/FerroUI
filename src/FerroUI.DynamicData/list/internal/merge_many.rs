//! `MergeMany<T, TDestination>` (`List/Internal/MergeMany.cs`): the values of a sequence chosen for
//! each item, merged, for as long as the item is in the list.

use super::subscribe_many;
use crate::list::ChangeSet;
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::subjects::Subject;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::Cell;
use std::rc::Rc;

/// The count of the source and the sequences not ended (`SubscriptionCounter`): the sequence that
/// follows the source ends when it reaches zero.
struct SubscriptionCounter<T: 'static> {
    subject: Rc<Subject<ChangeSet<T>>>,
    subscription_count: Cell<isize>,
}

impl<T: 'static> SubscriptionCounter<T> {
    fn new() -> Rc<Self> {
        Rc::new(Self { subject: Subject::new(), subscription_count: Cell::new(1) })
    }

    /// The sequence that follows the source: the end of the source is counted when it is
    /// subscribed (`DeferCleanup`).
    fn defer_cleanup(self: &Rc<Self>) -> Rc<dyn IObservable<ChangeSet<T>>> {
        let this = self.clone();
        Observable::defer(move || {
            this.check_completed();
            this.subject.as_observable()
        })
    }

    /// `Added`.
    fn added(&self) {
        self.subscription_count.set(self.subscription_count.get() + 1);
    }

    /// `Finally`.
    fn finally(&self) {
        self.check_completed();
    }

    /// `CheckCompleted`.
    fn check_completed(&self) {
        self.subscription_count.set(self.subscription_count.get() - 1);
        if self.subscription_count.get() == 0 {
            self.subject.on_completed();
        }
    }
}

/// `MergeMany.Run`: the sequence of each item is subscribed while the item is in the list; the
/// merge completes when the source and every sequence have completed, an error of the source ends
/// it, and those of the sequences are ignored.
pub(crate) fn merge_many<T: Clone + PartialEq + 'static, D: 'static>(
    source: Rc<dyn IObservable<ChangeSet<T>>>,
    observable_selector: Rc<dyn Fn(&T) -> Rc<dyn IObservable<D>>>,
) -> Rc<dyn IObservable<D>> {
    Observable::create(move |observer: Rc<dyn IObserver<D>>| {
        let counter = SubscriptionCounter::<T>::new();
        let (selector_counter, observable_selector) = (counter.clone(), observable_selector.clone());
        let (next_observer, error_observer, completed_observer) = (observer.clone(), observer.clone(), observer);
        let disposable = subscribe_many(
            source.concat(counter.defer_cleanup()),
            Rc::new(move |item: &T| {
                selector_counter.added();
                let finally_counter = selector_counter.clone();
                let next_observer = next_observer.clone();
                observable_selector(item)
                    .finally(move || finally_counter.finally())
                    .subscribe_all(move |value| next_observer.on_next(value), |_| {}, || {})
            }),
        )
        .subscribe_all(|_| {}, move |error| error_observer.on_error(error), move || completed_observer.on_completed());
        let subject = counter.subject.clone();
        let counter_disposal = Disposable::create(move || subject.dispose());
        let composite: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([disposable, counter_disposal]);
        composite
    })
}
