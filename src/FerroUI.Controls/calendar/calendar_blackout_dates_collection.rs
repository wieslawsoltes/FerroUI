// (c) Copyright Microsoft Corporation.
// This source is subject to the Microsoft Public License (Ms-PL).
// Please see https://go.microsoft.com/fwlink/?LinkID=131993 for details.
// All other rights reserved.
//
// Ported from the Silverlight Toolkit sources as adapted by the upstream
// project; the license text is in the `NOTICE.md` of this crate.

use super::{Calendar, CalendarDateRange, DateTimeHelper};
use ferroui_base::collections::{CollectionChangedHandler, FerroList};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::DateTime;
use ferroui_base::{Ref, WeakRef};
use std::rc::Rc;

/// Represents a collection of non-selectable dates in a [`Calendar`].
///
/// The value is a shared handle: clones refer to the same collection, and
/// handles compare by identity. Every change goes through the collection,
/// which validates it and refreshes the calendar that owns it.
#[derive(Clone)]
pub struct CalendarBlackoutDatesCollection {
    list: FerroList<Rc<CalendarDateRange>>,
    /// The Calendar whose dates this object represents.
    owner: WeakRef<Calendar>,
}

impl PartialEq for CalendarBlackoutDatesCollection {
    fn eq(&self, other: &Self) -> bool {
        self.list.ptr_eq(&other.list)
    }
}

impl CalendarBlackoutDatesCollection {
    /// Initializes a new instance of the
    /// [`CalendarBlackoutDatesCollection`] class.
    ///
    /// `owner` is the [`Calendar`] whose dates this object represents.
    pub fn new(owner: &Ref<Calendar>) -> Self {
        Self { list: FerroList::new(), owner: owner.downgrade() }
    }

    fn owner(&self) -> Ref<Calendar> {
        self.owner.upgrade().expect("the calendar of the blackout dates is alive")
    }

    /// Adds all dates before today to the collection.
    pub fn add_dates_in_past(&self) {
        self.add(CalendarDateRange::new_range(DateTime::MIN_VALUE, DateTime::today().add_days(-1.0)));
    }

    /// Returns a value that represents whether this collection contains the
    /// specified date.
    pub fn contains_date(&self, date: DateTime) -> bool {
        let count = self.count();
        for i in 0..count {
            if DateTimeHelper::in_range(date, &self.get(i)) {
                return true;
            }
        }
        false
    }

    /// Returns a value that represents whether this collection contains the
    /// specified range of dates: `start` is the start of the date range and
    /// `end` the end of it.
    pub fn contains_dates(&self, start: DateTime, end: DateTime) -> bool {
        let range_start;
        let range_end;

        if DateTime::compare(end, start) > -1 {
            range_start = DateTimeHelper::discard_time(start);
            range_end = DateTimeHelper::discard_time(end);
        } else {
            range_start = DateTimeHelper::discard_time(end);
            range_end = DateTimeHelper::discard_time(start);
        }

        let count = self.count();
        for i in 0..count {
            let range = self.get(i);
            if DateTime::compare(range.start(), range_start) == 0 && DateTime::compare(range.end(), range_end) == 0 {
                return true;
            }
        }
        false
    }

    /// Returns a value that represents whether this collection contains any
    /// date in the specified range.
    pub fn contains_any(&self, range: &CalendarDateRange) -> bool {
        self.list.snapshot().iter().any(|r| r.contains_any(range))
    }

    // --- the members of the collection ---------------------------------------

    /// The number of ranges in the collection.
    pub fn count(&self) -> usize {
        self.list.count()
    }

    /// Whether the collection has no ranges.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// The range at `index`. Panics when the index is out of range.
    pub fn get(&self, index: usize) -> Rc<CalendarDateRange> {
        self.list.get(index)
    }

    /// The ranges of the collection, as they are now.
    pub fn to_vec(&self) -> Vec<Rc<CalendarDateRange>> {
        self.list.to_vec()
    }

    /// Whether the collection contains the range (the same range object).
    pub fn contains(&self, item: &Rc<CalendarDateRange>) -> bool {
        self.index_of(item).is_some()
    }

    /// The index of the range (the same range object) in the collection.
    pub fn index_of(&self, item: &Rc<CalendarDateRange>) -> Option<usize> {
        self.list.snapshot().iter().position(|range| Rc::ptr_eq(range, item))
    }

    /// Adds a range to the end of the collection.
    ///
    /// Panics when the range contains a selected date of the calendar.
    pub fn add(&self, item: Rc<CalendarDateRange>) {
        self.insert_item(self.count(), item);
    }

    /// Inserts a range into the collection at the specified index.
    ///
    /// Panics when the range contains a selected date of the calendar.
    pub fn insert(&self, index: usize, item: Rc<CalendarDateRange>) {
        assert!(index <= self.count(), "Index must be within the bounds of the List.");
        self.insert_item(index, item);
    }

    /// Removes the range (the same range object) from the collection.
    /// Returns whether it was present.
    pub fn remove(&self, item: &Rc<CalendarDateRange>) -> bool {
        match self.index_of(item) {
            Some(index) => {
                self.remove_item(index);
                true
            }
            None => false,
        }
    }

    /// Removes the range at the specified index of the collection.
    pub fn remove_at(&self, index: usize) {
        assert!(index < self.count(), "Index was out of range.");
        self.remove_item(index);
    }

    /// Replaces the range at the specified index.
    ///
    /// Panics when the range contains a selected date of the calendar.
    pub fn set(&self, index: usize, item: Rc<CalendarDateRange>) {
        assert!(index < self.count(), "Index was out of range.");
        self.set_item(index, item);
    }

    /// Removes all ranges from the collection.
    pub fn clear(&self) {
        self.clear_items();
    }

    /// Moves the range at `old_index` to `new_index`.
    pub fn move_item(&self, old_index: usize, new_index: usize) {
        self.list.move_item(old_index, new_index);
    }

    /// Subscribes to the changes of the collection; returns the token of
    /// the subscription.
    pub fn add_collection_changed(&self, handler: Rc<CollectionChangedHandler<Rc<CalendarDateRange>>>) -> u64 {
        self.list.add_collection_changed(handler)
    }

    /// Ends a subscription to the changes of the collection.
    pub fn remove_collection_changed(&self, token: u64) -> bool {
        self.list.remove_collection_changed(token)
    }

    // --- the overrides of the collection --------------------------------------

    /// Removes all items from the collection.
    ///
    /// This implementation raises the collection changed event.
    fn clear_items(&self) {
        Self::ensure_valid_thread();

        self.list.clear();
        self.owner().update_months();
    }

    /// Inserts an item into the collection at the specified index.
    ///
    /// This implementation raises the collection changed event.
    fn insert_item(&self, index: usize, item: Rc<CalendarDateRange>) {
        Self::ensure_valid_thread();

        if !self.is_valid(&item) {
            panic!("Value is not valid. (Parameter 'item')");
        }

        self.list.insert(index, item);
        self.owner().update_months();
    }

    /// Removes the item at the specified index of the collection.
    ///
    /// This implementation raises the collection changed event.
    fn remove_item(&self, index: usize) {
        Self::ensure_valid_thread();

        self.list.remove_at(index);
        self.owner().update_months();
    }

    /// Replaces the element at the specified index.
    ///
    /// This implementation raises the collection changed event.
    fn set_item(&self, index: usize, item: Rc<CalendarDateRange>) {
        Self::ensure_valid_thread();

        if !self.is_valid(&item) {
            panic!("Value is not valid. (Parameter 'item')");
        }

        self.list.set(index, item);
        self.owner().update_months();
    }

    fn is_valid(&self, item: &CalendarDateRange) -> bool {
        self.owner().selected_dates().to_vec().into_iter().all(|day| !DateTimeHelper::in_range(day, item))
    }

    fn ensure_valid_thread() {
        Dispatcher::ui_thread().verify_access();
    }
}
