//! `ForForced` (`Cache/ObservableCacheEx.ForForced.cs`): the forms of the forced transforms of
//! `Transform` and `TransformSafe` as predicates of an item and its key.

use super::transform_with_forced_transform::ForcePredicate;
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::rc::Rc;

/// A predicate that accepts every item for each value of `source`
/// (`ForForced(IObservable<Unit>)`).
pub(crate) fn for_forced_all<S: 'static, K: 'static>(source: Rc<dyn IObservable<()>>) -> Rc<dyn IObservable<ForcePredicate<S, K>>> {
    source.select(|_: ()| {
        let transformer: ForcePredicate<S, K> = Rc::new(transform_all::<S, K>);
        transformer
    })
}

/// Each predicate of an item of `source` as a predicate of an item and its key
/// (`ForForced(IObservable<Func<TSource, bool>>)`).
pub(crate) fn for_forced<S: 'static, K: 'static>(source: Rc<dyn IObservable<Rc<dyn Fn(&S) -> bool>>>) -> Rc<dyn IObservable<ForcePredicate<S, K>>> {
    source.select(|condition: Rc<dyn Fn(&S) -> bool>| {
        let transformer: ForcePredicate<S, K> = Rc::new(move |item: &S, _key: &K| condition(item));
        transformer
    })
}

fn transform_all<S, K>(_item: &S, _key: &K) -> bool {
    true
}
