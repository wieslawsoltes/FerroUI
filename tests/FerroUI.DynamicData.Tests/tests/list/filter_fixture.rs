//! `FilterFixture` (`DynamicData.Tests/List/FilterFixture.cs`): the types that the parts of the
//! fixture share (`filter_fixture_base`, `filter_fixture_static`,
//! `filter_fixture_with_predicate_state`).

use ferroui_system::ObservableError;
use std::cell::Cell;
use std::rc::Rc;

/// Whether `actual` is the error `expected` itself (`results.Error.Should().Be(error)`, which
/// compares exceptions by reference).
pub(crate) fn is_same_error(actual: Option<ObservableError>, expected: &ObservableError) -> bool {
    actual.is_some_and(|actual| std::ptr::addr_eq(Rc::as_ptr(&actual), Rc::as_ptr(expected)))
}

/// When the source fails or completes: before the subscription or after it (`SourceType`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceType {
    /// Before the subscription (`Immediate`).
    Immediate,
    /// After the subscription (`Asynchronous`).
    Asynchronous,
}

/// Whether the unit under test drops the change sets without changes (`EmptyChangesetPolicy`).
///
/// Only the tests of `Base` use it, and upstream has no class derived from `Base`
/// (`filter_fixture_base`), so nothing runs them.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EmptyChangesetPolicy {
    /// `SuppressEmptyChangesets`.
    SuppressEmptyChangesets,
    /// `IncludeEmptyChangesets`.
    IncludeEmptyChangesets,
}

/// An item to filter (`Item`, a record): it is shared as an `Rc<Item>`, its inclusion can change
/// after it is added, and it compares by value, as a record does.
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
    #[allow(dead_code)]
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

    /// `FilterByEvenId` (no ported test uses it, as upstream).
    #[allow(dead_code)]
    pub(crate) fn filter_by_even_id(item: &Rc<Item>) -> bool {
        item.id % 2 == 0
    }

    /// `FilterByIsIncluded`.
    pub(crate) fn filter_by_is_included(item: &Rc<Item>) -> bool {
        item.is_included()
    }

    /// `FilterByIdInclusionMask` (no ported test uses it: upstream's only user is a test of
    /// `WithPredicateState`, with an `Item` of its own).
    #[allow(dead_code)]
    pub(crate) fn filter_by_id_inclusion_mask(id_inclusion_mask: i32, item: &Rc<Item>) -> bool {
        (item.id & id_inclusion_mask) == 0 && item.is_included()
    }
}
