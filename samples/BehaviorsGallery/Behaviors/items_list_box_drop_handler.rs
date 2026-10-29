//! Port of `Behaviors/ItemsListBoxDropHandler.cs`.

use crate::view_models::items::{as_item, boxed, index_of};
use crate::view_models::{DragAndDropSampleViewModel, DragItemViewModel};
use ferroui_base::input::{DragDropEffects, DragEventArgs};
use ferroui_base::{ferro_markup_type, BoxedValue, Ref, Visual};
use ferroui_controls::{Control, ListBox};
use ferroui_xaml_interactions_draganddrop::values::object_of;
use ferroui_xaml_interactions_draganddrop::{DropHandlerBase, IDropHandler};
use std::rc::Rc;

/// The drop handler of the list of items of the drag and drop pages: a copy inserts a copy of the
/// dragged item after the item it is dropped on, a move moves it there, a link swaps the two.
#[derive(Default)]
pub struct ItemsListBoxDropHandler;

impl PartialEq for ItemsListBoxDropHandler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ItemsListBoxDropHandler {
    /// `Validate<T>(ListBox, DragEventArgs, object?, object?, bool)`.
    fn validate_list_box(
        &self,
        list_box: &Ref<ListBox>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        execute: bool,
    ) -> bool {
        let list_box_visual: &Visual = list_box;
        let (Some(source_item), Some(vm), Some(target_item)) = (
            source_context.and_then(as_item::<DragItemViewModel>),
            target_context.and_then(as_item::<DragAndDropSampleViewModel>),
            list_box
                .get_visual_at(e.get_position(list_box_visual))
                .and_then(|visual| visual.cast::<Control>())
                .and_then(|target_control| target_control.data_context())
                .and_then(|data_context| as_item::<DragItemViewModel>(&data_context)),
        ) else {
            return false;
        };

        let items = vm.items();
        let (Some(source_index), Some(target_index)) = (
            index_of(&items, &source_item),
            index_of(&items, &target_item),
        ) else {
            return false;
        };

        let effects = e.drag_effects();
        if effects == DragDropEffects::COPY {
            if execute {
                let clone = DragItemViewModel::new(&format!(
                    "{}_copy",
                    source_item.title().unwrap_or_default()
                ));
                self.insert_item(&items, boxed(&clone), target_index + 1);
            }
            true
        } else if effects == DragDropEffects::MOVE {
            if execute {
                self.move_item(&items, source_index, target_index);
            }
            true
        } else if effects == DragDropEffects::LINK {
            if execute {
                self.swap_item(&items, source_index, target_index);
            }
            true
        } else {
            false
        }
    }

    /// The list box a drop handler is called for: the sender, when the source of the event is a
    /// control.
    fn list_box_of(sender: Option<&BoxedValue>, e: &DragEventArgs) -> Option<Ref<ListBox>> {
        let source_is_control = e.source().is_some_and(|source| source.is::<Control>());
        object_of(sender)
            .and_then(|sender| sender.cast::<ListBox>())
            .filter(|_| source_is_control)
    }
}

impl DropHandlerBase for ItemsListBoxDropHandler {
    fn validate(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        match Self::list_box_of(sender, e) {
            Some(list_box) => {
                self.validate_list_box(&list_box, e, source_context, target_context, false)
            }
            None => false,
        }
    }

    fn execute(
        &self,
        sender: Option<&BoxedValue>,
        e: &DragEventArgs,
        source_context: Option<&BoxedValue>,
        target_context: Option<&BoxedValue>,
        _state: Option<&BoxedValue>,
    ) -> bool {
        match Self::list_box_of(sender, e) {
            Some(list_box) => {
                self.validate_list_box(&list_box, e, source_context, target_context, true)
            }
            None => false,
        }
    }
}

ferro_markup_type!(class ItemsListBoxDropHandler {
    this: Rc<ItemsListBoxDropHandler>,
    handles: [ItemsListBoxDropHandler, Rc<ItemsListBoxDropHandler>, Option<Rc<ItemsListBoxDropHandler>>],
    interfaces: [Rc<dyn IDropHandler>],
    constructors: [() => || Rc::new(ItemsListBoxDropHandler)],
});
