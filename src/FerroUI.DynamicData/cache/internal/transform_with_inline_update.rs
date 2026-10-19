//! `TransformWithInlineUpdate<TDestination, TSource, TKey>`
//! (`Cache/Internal/TransformWithInlineUpdate.cs`).

use super::transform::ExceptionCallback;
use crate::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache, IQuery};
use crate::kernel::Error;
use ferroui_reactive::{Observable, ObservableExt};
use ferroui_system::{IObservable, ObservableError};
use std::cell::RefCell;
use std::fmt;
use std::hash::Hash;
use std::rc::Rc;

/// The factory of the transform (`Func<TSource, TDestination>`).
pub(crate) type InlineTransformFactory<S, D> = Rc<dyn Fn(&S) -> Result<D, ObservableError>>;

/// The update of a transformed item in place from the new source item
/// (`Action<TDestination, TSource>`).
pub(crate) type InlineUpdateAction<S, D> = Rc<dyn Fn(&D, &S) -> Result<(), ObservableError>>;

/// The error of an update whose key has no transformed item (`MissingKeyException`).
#[derive(Debug)]
struct MissingKeyError(String);

impl fmt::Display for MissingKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for MissingKeyError {}

/// Each item of the source transformed once by `transform_factory`; an update applies
/// `update_action` to the transformed item instead and emits a refresh of it
/// (`TransformWithInlineUpdate.Run`).
///
/// The transformed items are shared objects (upstream's `TDestination : class`): the update
/// changes the object that downstream holds already. With `transform_on_refresh`, a refresh is
/// an inline update too. With an `exception_callback`, the errors of the factory and of the
/// update go to it; otherwise they end the sequence.
pub(crate) fn transform_with_inline_update<D: Clone + 'static, S: Clone + 'static, K: Eq + Hash + Clone + 'static>(
    source: Rc<dyn IObservable<ChangeSet<S, K>>>,
    transform_factory: InlineTransformFactory<S, D>,
    update_action: InlineUpdateAction<S, D>,
    exception_callback: Option<ExceptionCallback<S, K>>,
    transform_on_refresh: bool,
) -> Rc<dyn IObservable<ChangeSet<D, K>>> {
    Observable::defer(move || {
        let cache: RefCell<Option<ChangeAwareCache<D, K>>> = RefCell::new(None);
        let (transform_factory, update_action, exception_callback) = (transform_factory.clone(), update_action.clone(), exception_callback.clone());
        source.try_select(move |changes: ChangeSet<S, K>| {
            let mut slot = cache.borrow_mut();
            let cache = slot.get_or_insert_with(|| ChangeAwareCache::with_capacity(changes.count()));
            for change in &changes {
                match change.reason() {
                    ChangeReason::Add => transform_item(cache, &*transform_factory, exception_callback.as_ref(), change.current(), change.key())?,
                    ChangeReason::Update => inline_update(cache, &*update_action, exception_callback.as_ref(), change.current(), change.key())?,
                    ChangeReason::Remove => cache.remove(change.key()),
                    ChangeReason::Refresh => {
                        if transform_on_refresh {
                            inline_update(cache, &*update_action, exception_callback.as_ref(), change.current(), change.key())?;
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

/// `Transform(ChangeAwareCache, in Change)`.
fn transform_item<D: Clone, S: Clone, K: Eq + Hash + Clone>(
    cache: &mut ChangeAwareCache<D, K>,
    transform_factory: &dyn Fn(&S) -> Result<D, ObservableError>,
    exception_callback: Option<&ExceptionCallback<S, K>>,
    current: &S,
    key: &K,
) -> Result<(), ObservableError> {
    match exception_callback {
        Some(exception_callback) => match transform_factory(current) {
            Ok(transformed) => cache.add_or_update(transformed, key.clone()),
            Err(error) => exception_callback(Error::new(Some(error), current.clone(), key.clone())),
        },
        None => {
            let transformed = transform_factory(current)?;
            cache.add_or_update(transformed, key.clone());
        }
    }
    Ok(())
}

/// `InlineUpdate(ChangeAwareCache, Change)`: a key without a transformed item ends the sequence
/// (`MissingKeyException`), whether or not there is a callback, as upstream.
///
/// Deviation (DEVIATIONS.md, DynamicData): the message does not name the key, which would need
/// `K: Debug`.
fn inline_update<D: Clone, S: Clone, K: Eq + Hash + Clone>(
    cache: &mut ChangeAwareCache<D, K>,
    update_action: &dyn Fn(&D, &S) -> Result<(), ObservableError>,
    exception_callback: Option<&ExceptionCallback<S, K>>,
    current: &S,
    key: &K,
) -> Result<(), ObservableError> {
    let previous = cache.lookup(key).ok_or_else(|| -> ObservableError { Rc::new(MissingKeyError("The key is not found.".to_owned())) })?;
    match exception_callback {
        Some(exception_callback) => {
            if let Err(error) = update_action(&previous, current) {
                exception_callback(Error::new(Some(error), current.clone(), key.clone()));
            }
        }
        None => update_action(&previous, current)?,
    }
    cache.refresh(key);
    Ok(())
}
