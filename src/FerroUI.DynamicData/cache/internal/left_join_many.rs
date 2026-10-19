//! `LeftJoinMany<TLeft, TLeftKey, TRight, TRightKey, TDestination>`
//! (`Cache/Internal/LeftJoinMany.cs`).

use super::{left_join, right_grouped_with_immutable_state, Cache};
use crate::cache::{ChangeSet, IGrouping, ImmutableGroup};
use ferroui_system::IObservable;
use std::hash::Hash;
use std::rc::Rc;

/// Each left item with the group of the right items whose key `right_key_selector` maps to its
/// key, an empty group when there is none, keyed by the left key (`LeftJoinMany.Run`).
pub(crate) fn left_join_many<TL, KL, TR, KR, D>(
    left: Rc<dyn IObservable<ChangeSet<TL, KL>>>,
    right: Rc<dyn IObservable<ChangeSet<TR, KR>>>,
    right_key_selector: Rc<dyn Fn(&TR) -> KL>,
    result_selector: Rc<dyn Fn(&KL, &TL, &ImmutableGroup<TR, KR, KL>) -> D>,
) -> Rc<dyn IObservable<ChangeSet<D, KL>>>
where
    TL: Clone + 'static,
    KL: Eq + Hash + Clone + 'static,
    TR: Clone + 'static,
    KR: Eq + Hash + Clone + 'static,
    D: Clone + 'static,
{
    let right_grouped = right_grouped_with_immutable_state(right, right_key_selector);
    left_join(
        left,
        right_grouped,
        Rc::new(|grouping: &ImmutableGroup<TR, KR, KL>| grouping.key().clone()),
        Rc::new(move |left_key: &KL, left: &TL, grouping: Option<&ImmutableGroup<TR, KR, KL>>| match grouping {
            Some(grouping) => result_selector(left_key, left, grouping),
            // `Cache<TRight, TRightKey>.Empty`
            None => result_selector(left_key, left, &ImmutableGroup::<TR, KR, KL>::new(left_key.clone(), &Cache::<TR, KR>::new())),
        }),
    )
}
