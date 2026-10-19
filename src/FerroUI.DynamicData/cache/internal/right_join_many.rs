//! `RightJoinMany<TLeft, TLeftKey, TRight, TRightKey, TDestination>`
//! (`Cache/Internal/RightJoinMany.cs`).

use super::{right_grouped_with_immutable_state, right_join};
use crate::cache::{ChangeSet, IGrouping, ImmutableGroup};
use ferroui_system::IObservable;
use std::hash::Hash;
use std::rc::Rc;

/// Each group of the right items, by the left key that `right_key_selector` maps them to, with
/// the left item of that key, if any, keyed by the left key (`RightJoinMany.Run`): a result exists
/// while the group has items.
pub(crate) fn right_join_many<TL, KL, TR, KR, D>(
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
    right_join(
        left,
        right_grouped,
        Rc::new(|grouping: &ImmutableGroup<TR, KR, KL>| grouping.key().clone()),
        Rc::new(move |key: &KL, left: Option<&TL>, grouping: &ImmutableGroup<TR, KR, KL>| result_selector(key, left, grouping)),
    )
}
