//! `Cast<TSource, TKey, TDestination>` (`Cache/Internal/Cast.cs`).

use crate::cache::cache_change_set_ex::transform_change_set;
use crate::cache::ChangeSet;
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// `Cast.Run`: each item, and each previous item, converted by `converter`.
pub(crate) fn cast<T: 'static, U: 'static, K: Clone + 'static>(source: Rc<dyn IObservable<ChangeSet<T, K>>>, converter: Rc<dyn Fn(&T) -> U>) -> Rc<dyn IObservable<ChangeSet<U, K>>> {
    source.select(move |changes: ChangeSet<T, K>| transform_change_set(&changes, &*converter))
}
