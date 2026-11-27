//! The sorting of a [`TableView`](crate::TableView), a WinUI addition (WinUI's tabular
//! `SortDirection`, `TableViewSortCycle` and `TableViewSortedEventArgs`;
//! `docs/porting/winui-controls.md`, section 10.11).

use crate::TableViewColumn;
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::{ferro_markup_enum, ferro_routed_event_args, Ref};

/// The direction a column of a [`TableView`](crate::TableView) sorts the rows in (WinUI's
/// `SortDirection`; the name keeps the type apart from the other sort directions of the default
/// XML namespace).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TableViewSortDirection {
    /// Not sorted: the order of the items source (the default).
    #[default]
    None = 0,
    /// From the lowest value to the highest.
    Ascending = 1,
    /// From the highest value to the lowest.
    Descending = 2,
}

/// The directions a click on the header of a column goes through, in order (WinUI's
/// `TableViewSortCycle`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum TableViewSortCycle {
    /// Ascending, descending, then the order of the source again (the default).
    #[default]
    AscendingDescendingNone = 0,
    /// Descending, ascending, then the order of the source again.
    DescendingAscendingNone = 1,
    /// Ascending and descending, in turn.
    AscendingDescending = 2,
    /// Descending and ascending, in turn.
    DescendingAscending = 3,
}

ferro_markup_enum!(TableViewSortDirection { None, Ascending, Descending }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(
    TableViewSortCycle { AscendingDescendingNone, DescendingAscendingNone, AscendingDescending, DescendingAscending },
    { namespace: "FerroUI.Controls" }
);

impl TableViewSortCycle {
    /// The direction that follows `current` in the cycle.
    pub fn next(self, current: TableViewSortDirection) -> TableViewSortDirection {
        use TableViewSortDirection::{Ascending, Descending, None};
        match (self, current) {
            (Self::AscendingDescendingNone, None) => Ascending,
            (Self::AscendingDescendingNone, Ascending) => Descending,
            (Self::AscendingDescendingNone, Descending) => None,
            (Self::DescendingAscendingNone, None) => Descending,
            (Self::DescendingAscendingNone, Descending) => Ascending,
            (Self::DescendingAscendingNone, Ascending) => None,
            (Self::AscendingDescending, Ascending) => Descending,
            (Self::AscendingDescending, _) => Ascending,
            (Self::DescendingAscending, Descending) => Ascending,
            (Self::DescendingAscending, _) => Descending,
        }
    }
}

/// Provides data for the `Sorted` event of a [`TableView`](crate::TableView): the column the rows
/// are sorted by and the direction, [`None`](TableViewSortDirection::None) when the rows are in
/// the order of the source again.
#[derive(Clone)]
pub struct TableViewSortedEventArgs {
    base: RoutedEventArgs,
    column: Ref<TableViewColumn>,
    direction: TableViewSortDirection,
}

ferro_routed_event_args!(TableViewSortedEventArgs: RoutedEventArgs);

impl TableViewSortedEventArgs {
    /// Creates args for a routed event, the column and the direction.
    pub fn new<T: ?Sized>(
        routed_event: Option<&RoutedEvent<T>>,
        column: Ref<TableViewColumn>,
        direction: TableViewSortDirection,
    ) -> Self {
        let base = RoutedEventArgs::new();
        base.set_routed_event(routed_event);
        Self { base, column, direction }
    }

    /// The column the rows were sorted by.
    pub fn column(&self) -> &Ref<TableViewColumn> {
        &self.column
    }

    /// The direction of the sort.
    pub fn direction(&self) -> TableViewSortDirection {
        self.direction
    }
}
