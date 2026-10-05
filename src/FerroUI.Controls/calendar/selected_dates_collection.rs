// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use super::{Calendar, CalendarSelectionMode, DateTimeHelper};
use crate::primitives::SelectingItemsControl;
use crate::SelectionChangedEventArgs;
use ferroui_base::collections::{CollectionChangedHandler, FerroList};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::DateTime;
use ferroui_base::{BoxedValue, Ref, WeakRef};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Represents a set of selected dates in a [`Calendar`].
///
/// The value is a shared handle: clones refer to the same collection, and
/// handles compare by identity. Every change goes through the collection,
/// which validates it against the selection mode of the calendar that owns
/// it and keeps the selected date of that calendar in step.
#[derive(Clone)]
pub struct SelectedDatesCollection(Rc<SelectedDatesCollectionData>);

struct SelectedDatesCollectionData {
    list: FerroList<DateTime>,
    /// Inherited code: Requires comment.
    added_items: RefCell<Vec<DateTime>>,
    /// Inherited code: Requires comment.
    is_cleared: Cell<bool>,
    /// Inherited code: Requires comment.
    is_range_added: Cell<bool>,
    /// Inherited code: Requires comment.
    owner: WeakRef<Calendar>,
}

impl PartialEq for SelectedDatesCollection {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// The dates as the items of the arguments of a selection change.
fn boxed_dates(dates: &[DateTime]) -> Vec<Option<BoxedValue>> {
    dates.iter().map(|date| Some(Rc::new(*date) as BoxedValue)).collect()
}

impl SelectedDatesCollection {
    /// Initializes a new instance of the [`SelectedDatesCollection`] class.
    ///
    /// `owner` is the [`Calendar`] associated with this object.
    pub fn new(owner: &Ref<Calendar>) -> Self {
        Self(Rc::new(SelectedDatesCollectionData {
            list: FerroList::new(),
            added_items: RefCell::new(Vec::new()),
            is_cleared: Cell::new(false),
            is_range_added: Cell::new(false),
            owner: owner.downgrade(),
        }))
    }

    fn owner(&self) -> Ref<Calendar> {
        self.0.owner.upgrade().expect("the calendar of the selected dates is alive")
    }

    fn invoke_collection_changed(&self, removed_items: &[DateTime], added_items: &[DateTime]) {
        self.owner().on_selected_dates_collection_changed(&SelectionChangedEventArgs::new(
            Some(SelectingItemsControl::selection_changed_event()),
            boxed_dates(removed_items),
            boxed_dates(added_items),
        ));
    }

    /// Adds all the dates in the specified range, which includes the first
    /// and last dates, to the collection.
    ///
    /// `start` is the first date to add to the collection and `end` the
    /// last one.
    pub fn add_range(&self, start: DateTime, end: DateTime) {
        let owner = self.owner();

        // increment parameter specifies if the Days were selected in
        // Descending order or Ascending order based on this value, we add
        // the days in the range either in Ascending order or in Descending
        // order
        let increment = if DateTime::compare(end, start) >= 0 { 1 } else { -1 };

        self.0.added_items.borrow_mut().clear();

        let mut range_start = Some(start);
        self.0.is_range_added.set(true);

        if owner.is_mouse_selection() {
            // In Mouse Selection we allow the user to be able to add
            // multiple ranges in one action in MultipleRange Mode.  In
            // SingleRange Mode, we only add the first selected range.
            while let Some(current) = range_start {
                if DateTime::compare(end, current) == -increment {
                    break;
                }

                if Calendar::is_valid_date_selection(&owner, Some(current)) {
                    self.add(current);
                } else if owner.selection_mode() == CalendarSelectionMode::SingleRange {
                    owner.set_hover_end(Some(current.add_days(-increment as f64)));
                    break;
                }

                range_start = DateTimeHelper::add_days(current, increment);
            }
        } else {
            // If CalendarSelectionMode.SingleRange and a user
            // programmatically tries to add multiple ranges, we will throw
            // away the old range and replace it with the new one.  In order
            // to provide the removed items without an additional event, we
            // are calling ClearInternal
            if owner.selection_mode() == CalendarSelectionMode::SingleRange && self.count() > 0 {
                owner.add_removed_items(&self.to_vec());
                self.clear_internal();
            }

            while let Some(current) = range_start {
                if DateTime::compare(end, current) == -increment {
                    break;
                }

                self.add(current);
                range_start = DateTimeHelper::add_days(current, increment);
            }
        }

        // The arguments get snapshots of the removed and the added dates. The
        // reference passes its live lists and clears the removed ones after
        // the event: a handler that keeps the arguments of the reference
        // sees them emptied later, here it keeps the dates.
        let added_items = self.0.added_items.borrow().clone();
        owner.on_selected_dates_collection_changed(&SelectionChangedEventArgs::new(
            Some(SelectingItemsControl::selection_changed_event()),
            boxed_dates(&owner.removed_items()),
            boxed_dates(&added_items),
        ));
        owner.clear_removed_items();
        owner.update_months();
        self.0.is_range_added.set(false);
    }

    // --- the members of the collection ---------------------------------------

    /// The number of dates in the collection.
    pub fn count(&self) -> usize {
        self.0.list.count()
    }

    /// Whether the collection has no dates.
    pub fn is_empty(&self) -> bool {
        self.0.list.is_empty()
    }

    /// The date at `index`. Panics when the index is out of range.
    pub fn get(&self, index: usize) -> DateTime {
        self.0.list.get(index)
    }

    /// The dates of the collection, as they are now.
    pub fn to_vec(&self) -> Vec<DateTime> {
        self.0.list.to_vec()
    }

    /// Whether the collection contains the date.
    pub fn contains(&self, item: DateTime) -> bool {
        self.0.list.contains(&item)
    }

    /// The index of the date in the collection.
    pub fn index_of(&self, item: DateTime) -> Option<usize> {
        self.0.list.index_of(&item)
    }

    /// Adds a date to the end of the collection.
    ///
    /// Panics when the selection mode of the calendar does not allow the
    /// change or the date cannot be selected.
    pub fn add(&self, item: DateTime) {
        self.insert_item(self.count(), item);
    }

    /// Inserts a date into the collection at the specified index.
    ///
    /// Panics when the selection mode of the calendar does not allow the
    /// change or the date cannot be selected.
    pub fn insert(&self, index: usize, item: DateTime) {
        assert!(index <= self.count(), "Index must be within the bounds of the List.");
        self.insert_item(index, item);
    }

    /// Removes a date from the collection. Returns whether it was present.
    pub fn remove(&self, item: DateTime) -> bool {
        match self.index_of(item) {
            Some(index) => {
                self.remove_item(index);
                true
            }
            None => false,
        }
    }

    /// Removes the date at the specified index of the collection.
    pub fn remove_at(&self, index: usize) {
        assert!(index < self.count(), "Index was out of range.");
        self.remove_item(index);
    }

    /// Replaces the date at the specified index.
    pub fn set(&self, index: usize, item: DateTime) {
        assert!(index < self.count(), "Index was out of range.");
        self.set_item(index, item);
    }

    /// Removes all dates from the collection.
    pub fn clear(&self) {
        self.clear_items();
    }

    /// Moves the date at `old_index` to `new_index`.
    pub fn move_item(&self, old_index: usize, new_index: usize) {
        self.0.list.move_item(old_index, new_index);
    }

    /// Subscribes to the changes of the collection; returns the token of
    /// the subscription.
    pub fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<DateTime>>) -> u64 {
        self.0.list.add_collection_changed(handler)
    }

    /// Ends a subscription to the changes of the collection.
    pub fn remove_collection_changed(&self, token: u64) -> bool {
        self.0.list.remove_collection_changed(token)
    }

    // --- the overrides of the collection --------------------------------------

    /// Removes all items from the collection.
    ///
    /// This implementation raises the collection changed event.
    fn clear_items(&self) {
        Self::ensure_valid_thread();
        let owner = self.owner();

        let added_items: Vec<DateTime> = Vec::new();
        let removed_items = self.to_vec();

        self.0.list.clear();

        // The event fires after SelectedDate changes
        if owner.selection_mode() != CalendarSelectionMode::None && owner.selected_date().is_some() {
            owner.set_selected_date(None);
        }

        if !removed_items.is_empty() {
            self.invoke_collection_changed(&removed_items, &added_items);
        }
        owner.update_months();
    }

    /// Inserts an item into the collection at the specified index.
    ///
    /// This implementation raises the collection changed event.
    fn insert_item(&self, index: usize, item: DateTime) {
        Self::ensure_valid_thread();
        let owner = self.owner();

        if !self.contains(item) {
            if self.check_selection_mode() {
                if Calendar::is_valid_date_selection(&owner, Some(item)) {
                    let mut index = index;

                    // If the Collection is cleared since it is SingleRange
                    // and it had another range set the index to 0
                    if self.0.is_cleared.get() {
                        index = 0;
                        self.0.is_cleared.set(false);
                    }

                    self.0.list.insert(index, item);

                    // The event fires after SelectedDate changes
                    if index == 0
                        && !owner.selected_date().is_some_and(|selected| DateTime::compare(selected, item) == 0)
                    {
                        owner.set_selected_date(Some(item));
                    }

                    if !self.0.is_range_added.get() {
                        // A snapshot of the removed dates; the reference passes
                        // its live list (see `add_range`).
                        self.invoke_collection_changed(&owner.removed_items(), &[item]);
                        owner.clear_removed_items();
                        let month_difference =
                            DateTimeHelper::compare_year_month(item, owner.display_date_internal());

                        if month_difference < 2 && month_difference > -2 {
                            owner.update_months();
                        }
                    } else {
                        self.0.added_items.borrow_mut().push(item);
                    }
                } else {
                    panic!("Specified argument was out of the range of valid values. (Parameter 'SelectedDate value is not valid.')");
                }
            }
        }
    }

    /// Removes the item at the specified index of the collection.
    ///
    /// This implementation raises the collection changed event.
    fn remove_item(&self, index: usize) {
        Self::ensure_valid_thread();
        let owner = self.owner();

        if index >= self.count() {
            self.0.list.remove_at(index);
        } else {
            let added_items: Vec<DateTime> = Vec::new();
            let removed = self.get(index);
            let month_difference = DateTimeHelper::compare_year_month(removed, owner.display_date_internal());

            self.0.list.remove_at(index);

            // The event fires after SelectedDate changes
            if index == 0 {
                if self.count() > 0 {
                    owner.set_selected_date(Some(self.get(0)));
                } else {
                    owner.set_selected_date(None);
                }
            }

            self.invoke_collection_changed(&[removed], &added_items);

            if month_difference < 2 && month_difference > -2 {
                owner.update_months();
            }
        }
    }

    /// Replaces the element at the specified index.
    ///
    /// This implementation raises the collection changed event.
    fn set_item(&self, index: usize, item: DateTime) {
        Self::ensure_valid_thread();
        let owner = self.owner();

        if !self.contains(item) {
            if index >= self.count() {
                self.0.list.set(index, item);
            } else {
                let current = self.get(index);
                if DateTime::compare(current, item) != 0 && Calendar::is_valid_date_selection(&owner, Some(item)) {
                    self.0.list.set(index, item);

                    // The event fires after SelectedDate changes
                    if index == 0
                        && !owner.selected_date().is_some_and(|selected| DateTime::compare(selected, item) == 0)
                    {
                        owner.set_selected_date(Some(item));
                    }

                    self.invoke_collection_changed(&[current], &[item]);

                    let month_difference = DateTimeHelper::compare_year_month(item, owner.display_date_internal());

                    if month_difference < 2 && month_difference > -2 {
                        owner.update_months();
                    }
                }
            }
        }
    }

    /// Removes all dates without the notifications of [`clear`](Self::clear)
    /// to the calendar.
    pub(crate) fn clear_internal(&self) {
        self.0.list.clear();
    }

    fn check_selection_mode(&self) -> bool {
        let owner = self.owner();

        if owner.selection_mode() == CalendarSelectionMode::None {
            panic!("The SelectedDate property cannot be set when the selection mode is None.");
        }
        if owner.selection_mode() == CalendarSelectionMode::SingleDate && self.count() > 0 {
            panic!("The SelectedDates collection can be changed only in a multiple selection mode. Use the SelectedDate in a single selection mode.");
        }

        // if user tries to add an item into the SelectedDates in
        // SingleRange mode, we throw away the old range and replace it with
        // the new one in order to provide the removed items without an
        // additional event, we are calling ClearInternal
        if owner.selection_mode() == CalendarSelectionMode::SingleRange
            && !self.0.is_range_added.get()
            && self.count() > 0
        {
            owner.add_removed_items(&self.to_vec());
            self.clear_internal();
            self.0.is_cleared.set(true);
        }
        true
    }

    fn ensure_valid_thread() {
        Dispatcher::ui_thread().verify_access();
    }
}
