//! `FullJoinMany<TLeft, TLeftKey, TRight, TRightKey, TDestination>`
//! (`Cache/Internal/FullJoinMany.cs`).

use super::{full_join, right_grouped_with_immutable_state, Cache};
use crate::cache::{ChangeSet, IGrouping, ImmutableGroup};
use ferroui_system::IObservable;
use std::hash::Hash;
use std::rc::Rc;

/// The left items and the groups of the right items, by the left key that `right_key_selector`
/// maps them to, joined by that key, an empty group when there is none (`FullJoinMany.Run`): a
/// result exists while the left item or the group does.
pub(crate) fn full_join_many<TL, KL, TR, KR, D>(
    left: Rc<dyn IObservable<ChangeSet<TL, KL>>>,
    right: Rc<dyn IObservable<ChangeSet<TR, KR>>>,
    right_key_selector: Rc<dyn Fn(&TR) -> KL>,
    result_selector: Rc<dyn Fn(&KL, Option<&TL>, &ImmutableGroup<TR, KR, KL>) -> D>,
) -> Rc<dyn IObservable<ChangeSet<D, KL>>>
where
    TL: Clone + 'static,
    KL: Eq + Hash + Clone + 'static,
    TR: Clone + 'static,
    KR: Eq + Hash + Clone + 'static,
    D: Clone + 'static,
{
    let right_grouped = right_grouped_with_immutable_state(right, right_key_selector);
    full_join(
        left,
        right_grouped,
        Rc::new(|grouping: &ImmutableGroup<TR, KR, KL>| grouping.key().clone()),
        Rc::new(move |left_key: &KL, left: Option<&TL>, grouping: Option<&ImmutableGroup<TR, KR, KL>>| match grouping {
            Some(grouping) => result_selector(left_key, left, grouping),
            // `Cache<TRight, TRightKey>.Empty`
            None => result_selector(left_key, left, &ImmutableGroup::<TR, KR, KL>::new(left_key.clone(), &Cache::<TR, KR>::new())),
        }),
    )
}
