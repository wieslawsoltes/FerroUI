//! `TransformManyAsync<TSource, TKey, TDestination, TDestinationKey>`
//! (`Cache/Internal/TransformManyAsync.cs`), with the transformers of
//! `ObservableCacheEx.CreateChangeSetTransformer` (`Cache/ObservableCacheEx.CreateChangeSetTransformer.cs`).

use super::{merge_many_cache_change_sets, ChangeSetSelector, Comparer, EqualityComparer, ExceptionCallback, TransformFuture};
use crate::binding::ObservableCollectionEx;
use crate::cache::{ChangeSet, IConnectableCache, IObservableCache};
use crate::kernel::Error;
use crate::EnumerableEx;
use ferroui_reactive::Observable;
use ferroui_system::{IObservable, ObservableError};
use std::future::Future;
use std::hash::Hash;
use std::rc::Rc;

/// The change sets of the children of an item of the source, given by a future
/// (`Func<TSource, TKey, Task<IObservable<IChangeSet<TDestination, TDestinationKey>>>>`).
pub(crate) type AsyncChangeSetTransformer<S, SK, D, DK> = Rc<dyn Fn(S, SK) -> TransformFuture<Rc<dyn IObservable<ChangeSet<D, DK>>>>>;

/// The children of each item of the source, given by the future of `transformer`, merged into one
/// cache (`TransformManyAsync.Run`).
///
/// Each item of the source has a child sequence that runs the future when it is subscribed
/// (`Observable.Defer(() => transform(obj, key))`) and follows the sequence the future gives.
/// Upstream's `Subscription` is the one of `MergeManyCacheChangeSets` without
/// `IgnoreSameReferenceUpdate` (which the port of `MergeManyCacheChangeSets` leaves out as well),
/// so the operator is that one over the deferred children. Without an `error_handler`, an error of
/// a future ends the sequence; with one, the error goes to the handler with the item and its key,
/// and the item has no children.
pub(crate) fn transform_many_async<S: Clone + 'static, SK: Eq + Hash + Clone + 'static, D: Clone + 'static, DK: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S, SK>>>,
    transformer: AsyncChangeSetTransformer<S, SK, D, DK>,
    equality_comparer: EqualityComparer<D>,
    comparer: Option<Comparer<D>>,
    error_handler: Option<ExceptionCallback<S, SK>>,
) -> Rc<dyn IObservable<ChangeSet<D, DK>>> {
    let selector: ChangeSetSelector<S, SK, D, DK> = match error_handler {
        // `Transformer`.
        None => Rc::new(move |item: &S, key: &SK| {
            let (transformer, item, key) = (transformer.clone(), item.clone(), key.clone());
            Observable::defer_async(move || transformer(item.clone(), key.clone()))
        }),
        // `SafeTransformer`.
        Some(error_handler) => Rc::new(move |item: &S, key: &SK| {
            let (transformer, error_handler, item, key) = (transformer.clone(), error_handler.clone(), item.clone(), key.clone());
            Observable::defer_async(move || error_handling_transform(transformer.clone(), error_handler.clone(), item.clone(), key.clone()))
        }),
    };
    merge_many_cache_change_sets(source, selector, equality_comparer, comparer)
}

/// The future of `transformer`, whose error goes to `error_handler` and gives no children
/// (`ErrorHandlingTransform`).
async fn error_handling_transform<S: 'static, SK: 'static, D: 'static, DK: 'static>(
    transformer: AsyncChangeSetTransformer<S, SK, D, DK>,
    error_handler: ExceptionCallback<S, SK>,
    item: S,
    key: SK,
) -> Result<Rc<dyn IObservable<ChangeSet<D, DK>>>, ObservableError>
where
    S: Clone,
    SK: Clone,
{
    match transformer(item.clone(), key.clone()).await {
        Ok(children) => Ok(children),
        Err(error) => {
            error_handler(Error::new(Some(error), item, key));
            Ok(Observable::empty())
        }
    }
}

/// The transformer of a selector of items, keyed by `key_selector`
/// (`CreateChangeSetTransformer(Func<TSource, TSourceKey, Task<IEnumerable<TDestination>>>, Func<TDestination, TDestinationKey>)`):
/// the items as one change set of additions (`AsObservableChangeSet(keySelector)`).
pub(crate) fn enumerable_change_set_transformer<S: 'static, SK: 'static, D: Clone + 'static, DK: Eq + Hash + Clone + 'static, Fut: Future<Output = Result<Vec<D>, ObservableError>> + 'static>(
    many_selector: impl Fn(S, SK) -> Fut + 'static,
    key_selector: impl Fn(&D) -> DK + 'static,
) -> AsyncChangeSetTransformer<S, SK, D, DK> {
    let key_selector: Rc<dyn Fn(&D) -> DK> = Rc::new(key_selector);
    Rc::new(move |item: S, key: SK| -> TransformFuture<Rc<dyn IObservable<ChangeSet<D, DK>>>> {
        let items = many_selector(item, key);
        let key_selector = key_selector.clone();
        Box::pin(async move {
            let items = items.await?;
            Ok::<_, ObservableError>(items.as_observable_change_set_cache(move |item: &D| key_selector(item)))
        })
    })
}

/// The transformer of a selector of observable collections, keyed by `key_selector`
/// (`CreateChangeSetTransformer<.., TCollection>(Func<TSource, TSourceKey, Task<TCollection>>, Func<TDestination, TDestinationKey>)`):
/// the items of the collection and its changes (`ToObservableChangeSet().AddKey(keySelector)`).
pub(crate) fn observable_collection_change_set_transformer<
    S: 'static,
    SK: 'static,
    D: Clone + PartialEq + 'static,
    DK: Eq + Hash + Clone + 'static,
    C: ObservableCollectionEx<D> + 'static,
    Fut: Future<Output = Result<C, ObservableError>> + 'static,
>(
    many_selector: impl Fn(S, SK) -> Fut + 'static,
    key_selector: impl Fn(&D) -> DK + 'static,
) -> AsyncChangeSetTransformer<S, SK, D, DK> {
    let key_selector: Rc<dyn Fn(&D) -> DK> = Rc::new(key_selector);
    Rc::new(move |item: S, key: SK| -> TransformFuture<Rc<dyn IObservable<ChangeSet<D, DK>>>> {
        let collection = many_selector(item, key);
        let key_selector = key_selector.clone();
        Box::pin(async move {
            let collection = collection.await?;
            Ok::<_, ObservableError>(collection.to_observable_change_set_with_key(move |item: &D| key_selector(item)))
        })
    })
}

/// The transformer of a selector of observable caches
/// (`CreateChangeSetTransformer(Func<TSource, TSourceKey, Task<IObservableCache<TDestination, TDestinationKey>>>)`):
/// the items of the cache and its changes (`Connect()`).
pub(crate) fn observable_cache_change_set_transformer<
    S: 'static,
    SK: 'static,
    D: Clone + 'static,
    DK: Eq + Hash + Clone + 'static,
    C: IObservableCache<D, DK> + 'static,
    Fut: Future<Output = Result<C, ObservableError>> + 'static,
>(
    many_selector: impl Fn(S, SK) -> Fut + 'static,
) -> AsyncChangeSetTransformer<S, SK, D, DK> {
    Rc::new(move |item: S, key: SK| -> TransformFuture<Rc<dyn IObservable<ChangeSet<D, DK>>>> {
        let cache = many_selector(item, key);
        Box::pin(async move {
            let cache = cache.await?;
            Ok::<_, ObservableError>(IConnectableCache::connect(&cache))
        })
    })
}
