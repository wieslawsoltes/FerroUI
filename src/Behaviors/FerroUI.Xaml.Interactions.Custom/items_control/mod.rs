//! The behaviors, triggers and actions of items controls (`ItemsControl/`): the actions that add,
//! insert, move and remove the items of the items source of an items control, the triggers and the
//! behavior of the events of its containers, the nudge of the containers during a drag, and the
//! behaviors that scroll an item into view.

mod add_item_to_items_control_action;
mod clear_items_control_action;
mod insert_item_to_items_control_action;
mod item_nudge_drop_behavior;
mod items_control_container_clearing_trigger;
mod items_control_container_events_behavior;
mod items_control_container_index_changed_trigger;
mod items_control_container_prepared_trigger;
mod items_control_preparing_container_trigger;
mod move_item_in_items_control_action;
mod remove_item_at_action;
mod remove_item_in_items_control_action;
mod scroll_to_item_behavior;
mod scroll_to_item_index_behavior;

pub use add_item_to_items_control_action::AddItemToItemsControlAction;
pub use clear_items_control_action::ClearItemsControlAction;
pub use insert_item_to_items_control_action::InsertItemToItemsControlAction;
pub use item_nudge_drop_behavior::ItemNudgeDropBehavior;
pub use items_control_container_clearing_trigger::ItemsControlContainerClearingTrigger;
pub use items_control_container_events_behavior::{
    ItemsControlContainerEventsBehavior, ItemsControlContainerEventsBehaviorImpl,
    ItemsControlContainerEventsBehaviorImplExt, ItemsControlContainerEventsBehaviorVTable,
};
pub use items_control_container_index_changed_trigger::ItemsControlContainerIndexChangedTrigger;
pub use items_control_container_prepared_trigger::ItemsControlContainerPreparedTrigger;
pub use items_control_preparing_container_trigger::ItemsControlPreparingContainerTrigger;
pub use move_item_in_items_control_action::MoveItemInItemsControlAction;
pub use remove_item_at_action::RemoveItemAtAction;
pub use remove_item_in_items_control_action::RemoveItemInItemsControlAction;
pub use scroll_to_item_behavior::ScrollToItemBehavior;
pub use scroll_to_item_index_behavior::ScrollToItemIndexBehavior;

/// Not from the reference: the writable `IList` the actions of this folder and of `Collections/`
/// change (`IList.Add`, `Insert`, `RemoveAt`, `Clear` of a list that is not read only).
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference changes any writable `IList`; an items
/// source of the framework is a read-only view of a list, so the port changes the lists whose type
/// it knows: a `FerroList` or an `ObservableCollection` of untyped values, texts, numbers,
/// booleans, characters or objects. Another list is treated as read only. An item that does not
/// convert to the item type of the list is not added (the reference throws).
pub(crate) mod writable_items {
    use ferroui_base::collections::FerroList;
    use ferroui_base::metadata::from_markup_value;
    use ferroui_base::{BoxedValue, FerroObject, Ref};
    use ferroui_controls::{Control, ItemsSource};
    use ferroui_system::collections::object_model::ObservableCollection;

    /// A list whose items can be added, inserted, removed and cleared through untyped values.
    pub(crate) trait IWritableItems {
        /// The number of items.
        fn count(&self) -> usize;
        /// Adds `item` at the end; false when it is not of the item type of the list.
        fn add(&self, item: &Option<BoxedValue>) -> bool;
        /// Inserts `item` at `index`; false when it is not of the item type of the list or the
        /// index is beyond the end of the list.
        fn insert(&self, index: usize, item: &Option<BoxedValue>) -> bool;
        /// Removes the item at `index` (in range).
        fn remove_at(&self, index: usize);
        /// Removes every item.
        fn clear(&self);
    }

    struct FerroListItems<'a, T>(&'a FerroList<T>);

    impl<T: Clone + PartialEq + 'static> IWritableItems for FerroListItems<'_, T> {
        fn count(&self) -> usize {
            self.0.count()
        }

        fn add(&self, item: &Option<BoxedValue>) -> bool {
            match from_markup_value::<T>(item) {
                Some(item) => {
                    self.0.add(item);
                    true
                }
                None => false,
            }
        }

        fn insert(&self, index: usize, item: &Option<BoxedValue>) -> bool {
            if index > self.0.count() {
                return false;
            }
            match from_markup_value::<T>(item) {
                Some(item) => {
                    self.0.insert(index, item);
                    true
                }
                None => false,
            }
        }

        fn remove_at(&self, index: usize) {
            self.0.remove_at(index);
        }

        fn clear(&self) {
            self.0.clear();
        }
    }

    struct ObservableCollectionItems<'a, T>(&'a ObservableCollection<T>);

    impl<T: Clone + PartialEq + 'static> IWritableItems for ObservableCollectionItems<'_, T> {
        fn count(&self) -> usize {
            self.0.count()
        }

        fn add(&self, item: &Option<BoxedValue>) -> bool {
            match from_markup_value::<T>(item) {
                Some(item) => {
                    self.0.add(item);
                    true
                }
                None => false,
            }
        }

        fn insert(&self, index: usize, item: &Option<BoxedValue>) -> bool {
            if index > self.0.count() {
                return false;
            }
            match from_markup_value::<T>(item) {
                Some(item) => {
                    self.0.insert(index, item);
                    true
                }
                None => false,
            }
        }

        fn remove_at(&self, index: usize) {
            self.0.remove_at(index);
        }

        fn clear(&self) {
            self.0.clear();
        }
    }

    /// The writable list behind `items_source` (`ItemsSource is IList list && !list.IsReadOnly`),
    /// or `None` when it is not one of the lists the port changes.
    pub(crate) fn writable_items(items_source: &ItemsSource) -> Option<Box<dyn IWritableItems + '_>> {
        macro_rules! writable {
            ($($item:ty),* $(,)?) => {$(
                if let Some(list) = items_source.downcast_ref::<FerroList<$item>>() {
                    return Some(Box::new(FerroListItems(list)));
                }
                if let Some(list) = items_source.downcast_ref::<ObservableCollection<$item>>() {
                    return Some(Box::new(ObservableCollectionItems(list)));
                }
            )*};
        }

        writable!(
            Option<BoxedValue>,
            BoxedValue,
            String,
            i32,
            i64,
            u32,
            u64,
            f32,
            f64,
            bool,
            char,
            Ref<FerroObject>,
            Ref<Control>,
        );
        None
    }
}
