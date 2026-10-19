//! `InnerJoinMany<TLeft, TLeftKey, TRight, TRightKey, TDestination>`
//! (`Cache/Internal/InnerJoinMany.cs`).

use super::{group_on_immutable, inner_join};
use crate::cache::{ChangeSet, IGrouping, ImmutableGroup, ImmutableGroupChangeSet, ObservableCacheEx};
use ferroui_reactive::ObservableExt;
use ferroui_system::IObservable;
use std::hash::Hash;
use std::rc::Rc;

/// The right items grouped by the left key that `right_key_selector` maps them to
/// (`_right.GroupWithImmutableState(_rightKeySelector)`), as plain change sets of the groups, the
/// right side of the joins of groups.
pub(crate) fn right_grouped_with_immutable_state<TR, KR, KL>(
    right: Rc<dyn IObservable<ChangeSet<TR, KR>>>,
    right_key_selector: Rc<dyn Fn(&TR) -> KL>,
) -> Rc<dyn IObservable<ChangeSet<ImmutableGroup<TR, KR, KL>, KL>>>
where
    TR: Clone + 'static,
    KR: Eq + Hash + Clone + 'static,
    KL: Eq + Hash + Clone + 'static,
{
    group_on_immutable(right, right_key_selector, None).select(|changes: ImmutableGroupChangeSet<TR, KR, KL>| changes.change_set().clone())
}

/// The left items with the group of the right items whose key `right_key_selector` maps to their
/// key, keyed by the left key (`InnerJoinMany.Run`): a result exists while the left item and at
/// least one right item do.
pub(crate) fn inner_join_many<TL, KL, TR, KR, D>(
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
    inner_join(
        left,
        right_grouped,
        Rc::new(|grouping: &ImmutableGroup<TR, KR, KL>| grouping.key().clone()),
        Rc::new(move |key: &(KL, KL), left: &TL, right: &ImmutableGroup<TR, KR, KL>| result_selector(&key.0, left, right)),
    )
    .change_key_with_key(|key_tuple: &(KL, KL), _: &D| key_tuple.0.clone())
}
