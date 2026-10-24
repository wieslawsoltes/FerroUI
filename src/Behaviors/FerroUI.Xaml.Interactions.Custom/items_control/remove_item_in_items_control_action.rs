//! Port of `ItemsControl/RemoveItemInItemsControlAction.cs`.

use super::writable_items::writable_items;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
};
use ferroui_controls::{Control, ItemsControl};
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Allows a user to remove the item from a [`ItemsControl`]: the data context of the sender (a
/// control) is removed from the items source of the first items control among the sender and its
/// logical ancestors.
///
/// The action returns true when the item was removed; else false.
#[repr(C)]
pub struct RemoveItemInItemsControlAction {
    base: StyledElementAction,
}

ferro_class!(RemoveItemInItemsControlAction: StyledElementAction);
ferro_class_info!(RemoveItemInItemsControlAction { new: RemoveItemInItemsControlAction::new });
ferroui_base::ferro_impl_classes!(RemoveItemInItemsControlAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for RemoveItemInItemsControlAction {
    /// Executes the action.
    ///
    /// Returns true if the item was removed; else false.
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(this.execute_core(sender)))
    }
}

impl RemoveItemInItemsControlAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn execute_core(&self, sender: Option<&BoxedValue>) -> bool {
        if !self.is_enabled() {
            return false;
        }

        let Some(control) = sender.and_then(object_of).and_then(|sender| sender.cast::<Control>()) else {
            return false;
        };

        let Some(items_control) = control.find_logical_ancestor_of_type::<ItemsControl>(true) else {
            return false;
        };

        if let Some(items_source) = items_control.items_source() {
            if let Some(list) = writable_items(&items_source) {
                let data = control.data_context();
                let index = items_source.index_of(&data);
                if index >= 0 {
                    list.remove_at(index as usize);
                    return true;
                }
            }
        }

        false
    }
}
