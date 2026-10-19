//! `TransformAsync<TDestination, TSource, TKey>` (`Cache/Internal/TransformAsync.cs`).
//!
//! Deviation (DEVIATIONS.md, DynamicData): upstream's factory returns a `Task<TDestination>`; here
//! it returns a future of `Result<TDestination, ObservableError>`, which runs as a task of the
//! thread (`Observable::from_async_with_token`, docs/porting/reactive.md, section 5.8). The factory
//! takes its arguments by value, so that the future it returns owns them. An `Err` of the future
//! is upstream's exception.

use super::{ExceptionCallback, ForcePredicate};
use crate::cache::{Change, ChangeAwareCache, ChangeReason, ChangeSet, ICache, IQuery};
use crate::internal::{synchronize_safe_shared, unsynchronized_merge, SharedDeliveryQueue};
use crate::kernel::Error;
use ferroui_reactive::disposables::{CompositeDisposable, Disposable};
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::threading::CancellationToken;
use ferroui_system::{IDisposable, IObservable, IObserver, ObservableError};
use std::cell::RefCell;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::rc::Rc;

/// A future of the thread that ends with a value or an error (what an `async` method returns
/// upstream, `Task<T>`).
pub(crate) type TransformFuture<T> = Pin<Box<dyn Future<Output = Result<T, ObservableError>>>>;

/// The factory of an asynchronous transform: the current item, the item it replaced (for an
/// update), the key and the token that the disposal of the subscription cancels
/// (`Func<TSource, Optional<TSource>, TKey, CancellationToken, Task<TDestination>>`).
pub(crate) type AsyncTransformFactory<S, D, K> = Rc<dyn Fn(S, Option<S>, K, CancellationToken) -> TransformFuture<D>>;

/// A shared factory of a closure that returns a future.
pub(crate) fn async_transform_factory<S: 'static, D: 'static, K: 'static, Fut: Future<Output = Result<D, ObservableError>> + 'static>(
    transform_factory: impl Fn(S, Option<S>, K, CancellationToken) -> Fut + 'static,
) -> AsyncTransformFactory<S, D, K> {
    Rc::new(move |current: S, previous: Option<S>, key: K, token: CancellationToken| -> TransformFuture<D> {
        Box::pin(transform_factory(current, previous, key, token))
    })
}

/// An item of the source and its transformed value (`TransformedItemContainer`).
#[derive(Clone)]
struct TransformedItemContainer<S, D> {
    destination: D,
    source: S,
}

/// The outcome of the transform of one change (`TransformResult`): the container of an addition,
/// an update or a transforming refresh; nothing for another change; the error of the factory when
/// an exception callback takes it (`Success` is false then).
#[derive(Clone)]
struct TransformResult<S, D, K> {
    change: Change<S, K>,
    container: Option<TransformedItemContainer<S, D>>,
    error: Option<ObservableError>,
}

impl<S, D, K> TransformResult<S, D, K> {
    /// `Success`.
    fn success(&self) -> bool {
        self.error.is_none()
    }
}

/// The arguments of the operator, shared by the subscriptions.
struct Context<S, D, K> {
    transform_factory: AsyncTransformFactory<S, D, K>,
    exception_callback: Option<ExceptionCallback<S, K>>,
    maximum_concurrency: Option<usize>,
    transform_on_refresh: bool,
}

type ContainerCache<S, D, K> = Rc<RefCell<ChangeAwareCache<TransformedItemContainer<S, D>, K>>>;

/// Each item of the source in the form the future of `transform_factory` gives it, under the same
/// key (`TransformAsync.Run`).
///
/// - The transforms of one change set run at once (at most `maximum_concurrency` at a time), and
///   the change set of their results is emitted when they have all ended. The change sets are
///   handled one after another (`Concat`), so the sequence ends when the source has ended and every
///   transform in flight has.
/// - An addition or an update awaits the factory; a removal removes the item, without the factory;
///   a refresh is forwarded, or with `transform_on_refresh` transforms the item again and emits an
///   update; a move is ignored.
/// - With an `exception_callback`, an error of the factory goes to the callback and the item is
///   left out; otherwise it ends the sequence.
/// - With `force_transform`, each predicate transforms again, as updates, the items it accepts. The
///   two chains share the cache, and their updates go through one delivery queue.
pub(crate) fn transform_async<D: Clone + 'static, S: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S, K>>>,
    transform_factory: AsyncTransformFactory<S, D, K>,
    exception_callback: Option<ExceptionCallback<S, K>>,
    force_transform: Option<Rc<dyn IObservable<ForcePredicate<S, K>>>>,
    maximum_concurrency: Option<usize>,
    transform_on_refresh: bool,
) -> Rc<dyn IObservable<ChangeSet<D, K>>> {
    let context = Rc::new(Context { transform_factory, exception_callback, maximum_concurrency, transform_on_refresh });
    Observable::create(move |observer: Rc<dyn IObserver<ChangeSet<D, K>>>| {
        let cache: ContainerCache<S, D, K> = Rc::new(RefCell::new(ChangeAwareCache::new()));

        let Some(force_transform) = force_transform.clone() else {
            // A single `Concat` chain serializes itself: the next transform is not subscribed
            // until the previous one has completed, so nothing else can be touching the cache.
            let (transformer_context, transformer_cache) = (context.clone(), cache.clone());
            return source
                .select(move |changes: ChangeSet<S, K>| do_transform(&transformer_context, &transformer_cache, &changes, None))
                .concat_inner::<ChangeSet<D, K>>()
                .subscribe_safe(observer);
        };

        // Two independent `Concat` chains share the cache, and each applies its updates when its
        // transforms end. The update of the cache, and the emission that follows it, run on the
        // shared queue, which serializes the two chains; that is also what `UnsynchronizedMerge`
        // requires of its inputs.
        let queue = SharedDeliveryQueue::new();

        let (transformer_context, transformer_cache, transformer_queue) = (context.clone(), cache.clone(), queue.clone());
        let transformer = source
            .select(move |changes: ChangeSet<S, K>| do_transform(&transformer_context, &transformer_cache, &changes, Some(&transformer_queue)))
            .concat_inner::<ChangeSet<D, K>>();

        // The forced predicate reads the cache to decide what to transform again; that read runs
        // during this delivery of the queue, which keeps it apart from the updates of either chain.
        let (forced_context, forced_cache, forced_queue) = (context.clone(), cache.clone(), queue.clone());
        let forced = synchronize_safe_shared(force_transform, &queue)
            .select(move |should_transform: ForcePredicate<S, K>| do_forced_transform(&forced_context, &forced_cache, &*should_transform, &forced_queue))
            .concat_inner::<ChangeSet<D, K>>();

        let subscription = unsynchronized_merge(transformer, vec![forced]).subscribe_safe(observer);
        let queue_disposal = Disposable::create(move || queue.dispose());
        let disposables: Rc<dyn IDisposable> = CompositeDisposable::from_disposables([subscription, queue_disposal]);
        disposables
    })
}

/// The transforms of the items that `should_transform` accepts, as updates
/// (`DoTransform(cache, Func<TSource, TKey, bool>, queue)`).
fn do_forced_transform<D: Clone + 'static, S: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    context: &Rc<Context<S, D, K>>,
    cache: &ContainerCache<S, D, K>,
    should_transform: &dyn Fn(&S, &K) -> bool,
    queue: &Rc<SharedDeliveryQueue>,
) -> Rc<dyn IObservable<ChangeSet<D, K>>> {
    let to_transform: Vec<Change<S, K>> = cache
        .borrow()
        .key_values()
        .into_iter()
        .filter(|(key, container)| should_transform(&container.source, key))
        .map(|(key, container)| Change::update(key, container.source.clone(), container.source))
        .collect();
    transform_changes(context, cache, to_transform, Some(queue))
}

/// The transforms of a change set of the source (`DoTransform(cache, IChangeSet, queue?)`): gated
/// by the queue only when a forced-transform chain also writes to the cache.
fn do_transform<D: Clone + 'static, S: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    context: &Rc<Context<S, D, K>>,
    cache: &ContainerCache<S, D, K>,
    changes: &ChangeSet<S, K>,
    queue: Option<&Rc<SharedDeliveryQueue>>,
) -> Rc<dyn IObservable<ChangeSet<D, K>>> {
    transform_changes(context, cache, changes.to_vec(), queue)
}

/// The transforms of `changes`, merged (`Merge(maximumConcurrency ?? int.MaxValue)`), gathered
/// (`ToArray`) and applied to the cache.
fn transform_changes<D: Clone + 'static, S: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    context: &Rc<Context<S, D, K>>,
    cache: &ContainerCache<S, D, K>,
    changes: Vec<Change<S, K>>,
    queue: Option<&Rc<SharedDeliveryQueue>>,
) -> Rc<dyn IObservable<ChangeSet<D, K>>> {
    let transforms: Vec<Rc<dyn IObservable<TransformResult<S, D, K>>>> = changes
        .into_iter()
        .map(|change| {
            let context = context.clone();
            Observable::from_async_with_token(move |token: CancellationToken| transform(context.clone(), change.clone(), token))
        })
        .collect();
    let merged = match context.maximum_concurrency {
        Some(maximum_concurrency) => Observable::merge_all_max(transforms, maximum_concurrency),
        None => Observable::merge_all(transforms),
    };
    let results = merged.to_array();
    let results = match queue {
        Some(queue) => synchronize_safe_shared(results, queue),
        None => results,
    };
    let (context, cache) = (context.clone(), cache.clone());
    results.select(move |transformed: Vec<TransformResult<S, D, K>>| process_updates(&context, &cache, transformed))
}

/// Applies the results to the cache and gives its changes, with the transformed items
/// (`ProcessUpdates`).
fn process_updates<D: Clone + 'static, S: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    context: &Context<S, D, K>,
    cache: &ContainerCache<S, D, K>,
    transformed_items: Vec<TransformResult<S, D, K>>,
) -> ChangeSet<D, K> {
    // Check for errors and call back if a handler has been specified.
    if let Some(exception_callback) = &context.exception_callback {
        for result in transformed_items.iter().filter(|result| !result.success()) {
            exception_callback(Error::new(result.error.clone(), result.change.current().clone(), result.change.key().clone()));
        }
    }

    let changes = {
        let mut cache = cache.borrow_mut();
        for result in transformed_items.into_iter().filter(TransformResult::success) {
            let key = result.change.key().clone();
            match result.change.reason() {
                ChangeReason::Add | ChangeReason::Update => {
                    cache.add_or_update(result.container.expect("an addition or an update has its transformed item"), key);
                }
                ChangeReason::Remove => cache.remove(&key),
                ChangeReason::Refresh => {
                    if context.transform_on_refresh {
                        cache.add_or_update(result.container.expect("a refresh that transforms has its transformed item"), key);
                    } else {
                        cache.refresh(&key);
                    }
                }
                ChangeReason::Moved => {}
            }
        }
        cache.capture_changes()
    };

    changes
        .iter()
        .map(|change| {
            Change::with_previous(
                change.reason(),
                change.key().clone(),
                change.current().destination.clone(),
                change.previous().map(|previous| previous.destination.clone()),
                change.current_index(),
                change.previous_index(),
            )
        })
        .collect()
}

/// The transform of one change (`Transform(Change, CancellationToken)`): the factory runs for an
/// addition, an update, or a refresh with `transform_on_refresh`. An error of the factory is the
/// result when an exception callback takes it, and the error of the future otherwise.
async fn transform<D: Clone + 'static, S: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    context: Rc<Context<S, D, K>>,
    change: Change<S, K>,
    token: CancellationToken,
) -> Result<TransformResult<S, D, K>, ObservableError> {
    let transforms = matches!(change.reason(), ChangeReason::Add | ChangeReason::Update) || (change.reason() == ChangeReason::Refresh && context.transform_on_refresh);
    if !transforms {
        return Ok(TransformResult { change, container: None, error: None });
    }
    let future = (context.transform_factory)(change.current().clone(), change.previous().cloned(), change.key().clone(), token);
    match future.await {
        Ok(destination) => {
            let container = TransformedItemContainer { destination, source: change.current().clone() };
            Ok(TransformResult { change, container: Some(container), error: None })
        }
        // Only handle errors if a handler has been specified.
        Err(error) if context.exception_callback.is_some() => Ok(TransformResult { change, container: None, error: Some(error) }),
        Err(error) => Err(error),
    }
}
