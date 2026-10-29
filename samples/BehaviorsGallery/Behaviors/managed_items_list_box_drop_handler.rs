//! Port of `Behaviors/ManagedItemsListBoxDropHandler.cs`.

use crate::view_models::items::{as_item, boxed, index_of, ItemList};
use crate::view_models::{DragAndDropSampleViewModel, DragItemViewModel};
use ferroui_base::input::{DragDropEffects, DragEventArgs};
use ferroui_base::{ferro_markup_type, BoxedValue, Ref, Visual};
use ferroui_controls::{Control, ItemsSource, ListBox, ListBoxItem};
use ferroui_xaml_interactions_draganddrop::values::object_of;
use ferroui_xaml_interactions_draganddrop::{DropHandlerBase, IDropHandler};
use std::rc::Rc;

/// The drop handler of the two lists of the managed drag and drop: a copy inserts a copy of the
/// dragged item after the item it is dropped on, a move moves it there (to the other list too), a
/// link swaps the two items.
#[derive(Default)]
pub struct ManagedItemsListBoxDropHandler;

impl PartialEq for ManagedItemsListBoxDropHandler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// Whether the list box shows `items` (`ReferenceEquals(listBox.ItemsSource, items)`).
fn shows(list_box: &ListBox, items: &ItemList) -> bool {
    list_box
        .items_source()
        .is_some_and(|source| source.ptr_eq(&ItemsSource::from(items.clone())))
}

impl ManagedItemsListBoxDropHandler {
    /// `TryGetManagedCollections(ListBox, DragAndDropSampleViewModel, out ..)`.
    fn try_get_managed_collections(
        list_box: &ListBox,
        vm: &DragAndDropSampleViewModel,
    ) -> Option<ItemList> {
        let source_items = vm.managed_source_items();
        if shows(list_box, &source_items) {
            return Some(source_items);
        }

        let target_items = vm.managed_target_items();
        if shows(list_box, &target_items) {
            return Some(target_items);
        }

        None
    }

    /// `GetTargetIndex(ListBox, DragEventArgs, ..., ListBoxItem?)`: the index of the item the drag
    /// is over, or the last index (`None` for an empty list).
    fn get_target_index(
        list_box: &Ref<ListBox>,
        e: &DragEventArgs,
        items: &ItemList,
        explicit_target: Option<&Ref<ListBoxItem>>,
    ) -> Option<usize> {
        if let Some(target_from_container) = explicit_target
            .and_then(|target| target.data_context())
            .and_then(|data_context| as_item::<DragItemViewModel>(&data_context))
        {
            if let Some(container_index) = index_of(items, &target_from_container) {
                return Some(container_index);
            }
        }

        let list_box_visual: &Visual = list_box;
        if let Some(target_item) = list_box
            .get_visual_at(e.get_position(list_box_visual))
            .and_then(|visual| visual.cast::<Control>())
            .and_then(|target_control| target_control.data_context())
            .and_then(|data_context| as_item::<DragItemViewModel>(&data_context))
        {
            if let Some(index) = index_of(items, &target_item) {
                return Some(index);
            }
        }

        items.count().checked_sub(1)
    }

    /// `Validate(ListBox, ListBoxItem?, DragEventArgs, object?, object?, bool)`.
    fn validate_list_box(
        &self,
        list_box: &Ref<ListBox>,
        target_container: Option<&Ref<ListBoxItem>>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        execute: bool,
    ) -> bool {
        let (Some(source_item), Some(vm)) = (
            source_context.and_then(as_item::<DragItemViewModel>),
            target_context.and_then(as_item::<DragAndDropSampleViewModel>),
        ) else {
            return false;
        };
        let Some(target_items) = Self::try_get_managed_collections(list_box, &vm) else {
            return false;
        };

        let target_index = Self::get_target_index(list_box, e, &target_items, target_container);
        let insert_index = match target_index {
            Some(target_index) => target_index + 1,
            None => target_items.count(),
        };

        let managed_source_items = vm.managed_source_items();
        let managed_target_items = vm.managed_target_items();
        let source_items = if index_of(&managed_source_items, &source_item).is_some() {
            managed_source_items
        } else if index_of(&managed_target_items, &source_item).is_some() {
            managed_target_items
        } else {
            return false;
        };

        let Some(source_index) = index_of(&source_items, &source_item) else {
            return false;
        };
        let same_items = Rc::ptr_eq(&source_items, &target_items);

        let effects = e.drag_effects();
        if effects == DragDropEffects::COPY {
            if execute {
                let clone = DragItemViewModel::new(&format!(
                    "{}_copy",
                    source_item.title().unwrap_or_default()
                ));
                self.insert_item(&target_items, boxed(&clone), insert_index);
            }
            true
        } else if effects == DragDropEffects::MOVE {
            if execute {
                if same_items {
                    // `targetIndex >= 0 ? targetIndex : targetItems.Count - 1`, at least 0.
                    let move_index = target_index
                        .or_else(|| target_items.count().checked_sub(1))
                        .unwrap_or(0);
                    self.move_item(&target_items, source_index, move_index);
                } else {
                    self.move_item_between(
                        &source_items,
                        &target_items,
                        source_index,
                        insert_index,
                    );
                }
            }
            true
        } else if effects == DragDropEffects::LINK {
            let Some(target_index) = target_index else {
                return false;
            };

            if execute {
                if same_items {
                    self.swap_item(&target_items, source_index, target_index);
                } else {
                    self.swap_item_between(
                        &source_items,
                        &target_items,
                        source_index,
                        target_index,
                    );
                }
            }
            true
        } else {
            false
        }
    }

    /// The list box and the container a drop handler is called for: the sender, when it is a list
    /// box, or the list box of the sender, when it is an item of a list box.
    fn validate_sender(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        execute: bool,
    ) -> bool {
        let Some(sender) = object_of(sender) else {
            return false;
        };

        if let Some(list_box) = sender.cast::<ListBox>() {
            return self.validate_list_box(
                &list_box,
                None,
                e,
                source_context,
                target_context,
                execute,
            );
        }

        if let Some(list_box_item) = sender.cast::<ListBoxItem>() {
            let item_visual: &Visual = &list_box_item;
            if let Some(owner) = item_visual.find_ancestor_of_type::<ListBox>(false) {
                return self.validate_list_box(
                    &owner,
                    Some(&list_box_item),
                    e,
                    source_context,
                    target_context,
                    execute,
                );
            }
        }

        false
    }
}

impl DropHandlerBase for ManagedItemsListBoxDropHandler {
    fn validate(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        self.validate_sender(sender, e, source_context, target_context, false)
    }

    fn execute(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        self.validate_sender(sender, e, source_context, target_context, true)
    }
}

ferro_markup_type!(class ManagedItemsListBoxDropHandler {
    this: Rc<ManagedItemsListBoxDropHandler>,
    handles: [ManagedItemsListBoxDropHandler, Rc<ManagedItemsListBoxDropHandler>, Option<Rc<ManagedItemsListBoxDropHandler>>],
    interfaces: [Rc<dyn IDropHandler>],
    constructors: [() => || Rc::new(ManagedItemsListBoxDropHandler)],
});
