//! `Transform<TDestination, TSource, TKey>` (`Cache/Internal/Transform.cs`).

use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache};
use crate::kernel::Error;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, ObservableError};
use std::cell::RefCell;
use std::hash::Hash;
use std::rc::Rc;

/// The factory of a transform: the current item, the item it replaced (for an update) and the key
/// (`Func<TSource, Optional<TSource>, TKey, TDestination>`). An `Err` is upstream's exception.
pub(crate) type TransformFactory<S, D, K> = Rc<dyn Fn(&S, Option<&S>, &K) -> Result<D, ObservableError>>;

/// The handler of the errors of the factory (`Action<Error<TSource, TKey>>`).
pub(crate) type ExceptionCallback<S, K> = Rc<dyn Fn(Error<S, K>)>;

/// A shared factory of a closure (whose signature the bound gives it).
pub(crate) fn transform_factory<S: 'static, D: 'static, K: 'static>(
    transform_factory: impl Fn(&S, Option<&S>, &K) -> Result<D, ObservableError> + 'static,
) -> TransformFactory<S, D, K> {
    Rc::new(transform_factory)
}

/// Each item of the source in the form `transform_factory` gives it, under the same key
/// (`Transform.Run`).
///
/// - An addition or an update calls the factory and adds or updates the result; with an
///   `exception_callback`, an error of the factory goes to the callback and the item is left out,
///   otherwise it ends the sequence with the error.
/// - A removal removes the result, without the factory.
/// - A refresh is forwarded, or with `transform_on_refresh` transforms the item again and emits
///   the result as an update. The factory of a refresh is not guarded by the callback, as upstream.
/// - A move is ignored.
///
/// Every change set of the source makes one change set, empty or not.
pub(crate) fn transform<D: Clone + 'static, S: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S, K>>>,
    transform_factory: impl Fn(&S, Option<&S>, &K) -> Result<D, ObservableError> + 'static,
    exception_callback: Option<ExceptionCallback<S, K>>,
    transform_on_refresh: bool,
) -> Rc<dyn IObservable<ChangeSet<D, K>>> {
    let transform_factory: TransformFactory<S, D, K> = Rc::new(transform_factory);
    transform_shared(source, transform_factory, exception_callback, transform_on_refresh)
}

/// [`transform`] with a factory that is shared already.
pub(crate) fn transform_shared<D: Clone + 'static, S: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S, K>>>,
    transform_factory: TransformFactory<S, D, K>,
    exception_callback: Option<ExceptionCallback<S, K>>,
    transform_on_refresh: bool,
) -> Rc<dyn IObservable<ChangeSet<D, K>>> {
    // `Observable.Defer(RunImpl)`: each subscription has a cache of its own.
    Observable::defer(move || {
        let cache: Rc<RefCell<Option<ChangeAwareCache<D, K>>>> = Rc::new(RefCell::new(None));
        let transform_factory = transform_factory.clone();
        let exception_callback = exception_callback.clone();
        source.try_select(move |changes: ChangeSet<S, K>| {
            let mut slot = cache.borrow_mut();
            let cache = slot.get_or_insert_with(|| ChangeAwareCache::with_capacity(changes.count()));
            for change in &changes {
                match change.reason() {
                    ChangeReason::Add | ChangeReason::Update => match &exception_callback {
                        Some(exception_callback) => match transform_factory(change.current(), change.previous(), change.key()) {
                            Ok(transformed) => cache.add_or_update(transformed, change.key().clone()),
                            Err(error) => exception_callback(Error::new(Some(error), change.current().clone(), change.key().clone())),
                        },
                        None => {
                            let transformed = transform_factory(change.current(), change.previous(), change.key())?;
                            cache.add_or_update(transformed, change.key().clone());
                        }
                    },
                    ChangeReason::Remove => cache.remove(change.key()),
                    ChangeReason::Refresh => {
                        if transform_on_refresh {
                            let transformed = transform_factory(change.current(), change.previous(), change.key())?;
                            cache.add_or_update(transformed, change.key().clone());
                        } else {
                            cache.refresh(change.key());
                        }
                    }
                    // Do nothing!
                    ChangeReason::Moved => {}
                }
            }
            Ok(cache.capture_changes())
        })
    })
}
