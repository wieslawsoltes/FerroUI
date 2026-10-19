//! `TransformWithForcedTransform<TDestination, TSource, TKey>`
//! (`Cache/Internal/TransformWithForcedTransform.cs`).

use super::transform::{transform_shared, ExceptionCallback, TransformFactory};
use super::Cache;
use crate::cache::{Change, ChangeReason, ChangeSet, ICache, IQuery};
use ferroui_reactive::disposables::CompositeDisposable;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IDisposable, IObservable, IObserver};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The predicate of a forced transform: the items it accepts are transformed again
/// (`Func<TSource, TKey, bool>`).
pub(crate) type ForcePredicate<S, K> = Rc<dyn Fn(&S, &K) -> bool>;

/// [`transform`](super::transform) that also transforms again, as updates, the items that each
/// predicate of `force_transform` accepts (`TransformWithForcedTransform.Run`).
///
/// The source is shared: one subscription keeps a copy of its items, from which each predicate
/// makes a change set of refreshes, and the transform reads the source merged with those
/// refreshes, transforming on refresh. Upstream serializes both through a delivery queue; on one
/// thread they are delivered in order as they come (docs/porting/reactive.md, section 5.4).
pub(crate) fn transform_with_forced_transform<D: Clone + 'static, S: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S, K>>>,
    transform_factory: TransformFactory<S, D, K>,
    force_transform: Rc<dyn IObservable<ForcePredicate<S, K>>>,
    exception_callback: Option<ExceptionCallback<S, K>>,
) -> Rc<dyn IObservable<ChangeSet<D, K>>> {
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<D, K>>>| {
        let shared = source.publish();
        let shared_source: Rc<dyn IObservable<ChangeSet<S, K>>> = shared.clone();

        // Capture all items so that a forced transform can be applied.
        let cache = Rc::new(RefCell::new(Cache::<S, K>::new()));
        let loader = cache.clone();
        let cache_loader = shared_source.subscribe_next(move |changes: ChangeSet<S, K>| loader.borrow_mut().clone_changes(&changes));

        // The change sets of the items to which the forced transform applies.
        let captured = cache.clone();
        let refresher = force_transform
            .select(move |should_transform: ForcePredicate<S, K>| capture_changes(&captured.borrow(), &*should_transform))
            .where_(|changes: &ChangeSet<S, K>| changes.count() != 0);

        let source_and_refreshes = shared_source.merge(refresher);

        // The raw transform.
        let transformed = transform_shared(source_and_refreshes, transform_factory.clone(), exception_callback.clone(), true);
        let subscription = transformed.subscribe_safe(observer);
        let connection = shared.connect();
        let disposables: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([cache_loader, subscription, connection]);
        disposables
    })
}

/// The refreshes of the items that `should_transform` accepts (`CaptureChanges`).
fn capture_changes<S: Clone, K: Eq + Hash + Clone>(cache: &Cache<S, K>, should_transform: &dyn Fn(&S, &K) -> bool) -> ChangeSet<S, K> {
    cache
        .key_values()
        .into_iter()
        .filter(|(key, value)| should_transform(value, key))
        .map(|(key, value)| Change::new(ChangeReason::Refresh, key, value))
        .collect()
}
