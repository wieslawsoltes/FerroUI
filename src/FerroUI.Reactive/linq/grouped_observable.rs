//! `IGroupedObservable<TKey, TElement>` (`Linq/IGroupedObservable.cs`) and `GroupedObservable`
//! (`Linq/GroupedObservable.cs`): the groups of `GroupBy` and `GroupByUntil`.

use crate::disposables::{RefCountDisposable, StableCompositeDisposable};
use crate::internal::subscribe_auto_detach;
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::rc::Rc;

/// A sequence of the values that share a key (`IGroupedObservable<TKey, TElement>`): a group of
/// `GroupBy` or `GroupByUntil`.
pub trait IGroupedObservable<K, V>: IObservable<V> {
    /// The key the values of the group share (`Key`).
    fn key(&self) -> &K;
}

/// A group as the handle of a sequence: the values of `GroupBy` and `GroupByUntil`.
pub(crate) type Group<K, V> = Rc<dyn IGroupedObservable<K, V>>;

/// A group (`GroupedObservable<TKey, TElement>`): the subject the operator writes the values of
/// the key to, and, for the groups the operator hands downstream, the reference count of the
/// operator's subscription to its source: a subscription to the group keeps the source subscribed
/// while it lasts.
pub(crate) struct GroupedObservable<K, V> {
    key: K,
    subject: Rc<dyn IObservable<V>>,
    ref_count: Option<Rc<RefCountDisposable>>,
}

impl<K: 'static, V: 'static> GroupedObservable<K, V> {
    /// A group of `key` whose values `subject` gives, which keeps a reference of `ref_count` for
    /// each subscription (`GroupedObservable(TKey, ISubject<TElement>, RefCountDisposable)`), or
    /// none (`GroupedObservable(TKey, ISubject<TElement>)`).
    pub(crate) fn new(key: K, subject: Rc<dyn IObservable<V>>, ref_count: Option<Rc<RefCountDisposable>>) -> Group<K, V> {
        Rc::new(Self { key, subject, ref_count })
    }
}

impl<K: 'static, V: 'static> IObservable<V> for GroupedObservable<K, V> {
    /// Subscribes to the subject of the group (`SubscribeCore`, an `ObservableBase<TElement>`):
    /// with a reference count, the subscription holds a dependent disposable of it.
    fn subscribe(&self, observer: Rc<dyn IObserver<V>>) -> Rc<dyn IDisposable> {
        let subject = self.subject.clone();
        let ref_count = self.ref_count.clone();
        subscribe_auto_detach(observer, move |observer| -> Rc<dyn IDisposable> {
            match ref_count {
                Some(ref_count) => {
                    let release = ref_count.get_disposable();
                    let subscription = subject.subscribe(observer);
                    StableCompositeDisposable::create(release, subscription)
                }
                None => subject.subscribe(observer),
            }
        })
    }
}

impl<K: 'static, V: 'static> IGroupedObservable<K, V> for GroupedObservable<K, V> {
    fn key(&self) -> &K {
        &self.key
    }
}

/// The operators on a group, as on any sequence.
impl<K: 'static, V: 'static> crate::linq::ObservableExt<V> for Rc<dyn IGroupedObservable<K, V>> {
    fn observable(&self) -> Rc<dyn IObservable<V>> {
        self.clone()
    }
}
