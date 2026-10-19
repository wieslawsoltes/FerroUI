//! `FilterFixture` (`DynamicData.Tests/Cache/FilterFixture.cs`): the types and helpers shared by
//! the tests of the filter operators of caches (the parts of the partial class in the other
//! `filter_fixture_*` modules).

use crate::domain::Randomizer;
use ferroui_dynamic_data::cache::{ChangeAwareCache, ChangeReason, ChangeSet, ICache, IQuery};
use std::cell::Cell;
use std::rc::Rc;

/// Whether a unit under test suppresses its empty change sets (`EmptyChangesetPolicy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EmptyChangesetPolicy {
    /// `SuppressEmptyChangesets`.
    SuppressEmptyChangesets,
    /// `IncludeEmptyChangesets`.
    IncludeEmptyChangesets,
}

/// An input stream of a dynamic filter (`DynamicParameter`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DynamicParameter {
    /// `Source`.
    Source,
    /// `PredicateChanged`.
    PredicateChanged,
    /// `ReapplyFilter`.
    ReapplyFilter,
}

/// A predicate of items, the type of the values of `predicateChanged` (`Func<Item, bool>`).
pub(crate) type ItemPredicate = Rc<dyn Fn(&Rc<Item>) -> bool>;

/// The item of the filter tests (`Item`), a record whose `IsIncluded` can change in place: it is
/// held in an `Rc`, and compares by value as a record does.
#[derive(Debug, PartialEq)]
pub(crate) struct Item {
    id: i32,
    is_included: Cell<bool>,
}

impl Item {
    /// `new Item() { Id = id, IsIncluded = is_included }`.
    pub(crate) fn new(id: i32, is_included: bool) -> Rc<Self> {
        Rc::new(Self { id, is_included: Cell::new(is_included) })
    }

    /// `Id`.
    pub(crate) fn id(&self) -> i32 {
        self.id
    }

    /// `IsIncluded`.
    pub(crate) fn is_included(&self) -> bool {
        self.is_included.get()
    }

    /// `IsIncluded = value`.
    pub(crate) fn set_is_included(&self, value: bool) {
        self.is_included.set(value);
    }

    /// `FilterByEvenId`.
    pub(crate) fn filter_by_even_id(item: &Rc<Item>) -> bool {
        (item.id % 2) == 0
    }

    /// `FilterByIsIncluded`.
    pub(crate) fn filter_by_is_included(item: &Rc<Item>) -> bool {
        item.is_included()
    }

    /// `FilterByIdInclusionMask`.
    pub(crate) fn filter_by_id_inclusion_mask(id_inclusion_mask: &i32, item: &Rc<Item>) -> bool {
        ((item.id & *id_inclusion_mask) == 0) && item.is_included()
    }

    /// `SelectId`.
    pub(crate) fn select_id(item: &Rc<Item>) -> i32 {
        item.id
    }
}

/// `GenerateStressItemsAndChangeSets`: `edit_count` change sets of 1 to `max_change_count` random
/// changes each, and the cache of the items they make.
pub(crate) fn generate_stress_items_and_change_sets(
    edit_count: usize,
    max_change_count: i32,
    randomizer: &Randomizer,
) -> (ChangeAwareCache<Rc<Item>, i32>, Vec<ChangeSet<Rc<Item>, i32>>) {
    // Not exercising Moved, since ChangeAwareCache<> doesn't support it, and I'm too lazy to implement it by hand.
    let change_reasons = [ChangeReason::Add, ChangeReason::Refresh, ChangeReason::Remove, ChangeReason::Update];

    // Weights are chosen to make the cache size likely to grow over time,
    // exerting more pressure on the system the longer the benchmark runs.
    // Also, to prevent bogus operations (E.G. you can't remove an item from an empty cache).
    let change_reason_weights_when_count_is_0 = [
        1.0f32, // Add
        0.0,    // Refresh
        0.0,    // Remove
        0.0,    // Update
    ];

    let change_reason_weights_otherwise = [
        0.30f32, // Add
        0.25,    // Refresh
        0.20,    // Remove
        0.25,    // Update
    ];

    let mut next_item_id = 1;

    let mut change_sets = Vec::with_capacity(edit_count);

    let mut items = ChangeAwareCache::<Rc<Item>, i32>::new();

    while change_sets.len() < edit_count {
        let change_count = randomizer.int_range(1, max_change_count);
        for _ in 0..change_count {
            let weights = match items.count() {
                0 => &change_reason_weights_when_count_is_0,
                _ => &change_reason_weights_otherwise,
            };
            let change_reason = randomizer.weighted_random(&change_reasons, weights);

            match change_reason {
                ChangeReason::Add => {
                    items.add_or_update(Item::new(next_item_id, randomizer.bool()), next_item_id);
                    next_item_id += 1;
                }
                ChangeReason::Refresh => {
                    let key = items.keys()[randomizer.int_range(0, items.count() as i32 - 1) as usize];
                    items.refresh(&key);
                }
                ChangeReason::Remove => {
                    let key = items.keys()[randomizer.int_range(0, items.count() as i32 - 1) as usize];
                    items.remove(&key);
                }
                ChangeReason::Update => {
                    let id = items.keys()[randomizer.int_range(0, items.count() as i32 - 1) as usize];
                    items.add_or_update(Item::new(id, randomizer.bool()), id);
                }
                ChangeReason::Moved => {}
            }
        }

        change_sets.push(items.capture_changes());
    }

    (items, change_sets)
}

/// `GenerateRandomIdInclusionMasks`: `value_count` random masks of ids.
pub(crate) fn generate_random_id_inclusion_masks(value_count: usize, randomizer: &Randomizer) -> Vec<i32> {
    let mut values = Vec::with_capacity(value_count);

    while values.len() < value_count {
        values.push(randomizer.int());
    }

    values
}
