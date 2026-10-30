//! Port of `ListBox/RemoveItemInListBoxAction.cs`.

use ferroui_base::collections::FerroList;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObject, FerroObjectImpl, Ref, StyledElementImpl,
};
use ferroui_controls::{Control, ItemsControl, ItemsSource, ListBox, ListBoxItem};
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Allows a user to remove the item from a [`ListBox`] item template.
///
/// The sender is a control of the item; the action removes its data context from the items source
/// of the first items control among the sender and its logical ancestors, or else from the items
/// of the list box.
#[repr(C)]
pub struct RemoveItemInListBoxAction {
    base: StyledElementAction,
}

ferro_class!(RemoveItemInListBoxAction: StyledElementAction);
ferro_class_info!(RemoveItemInListBoxAction { new: RemoveItemInListBoxAction::new });
ferroui_base::ferro_impl_classes!(RemoveItemInListBoxAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for RemoveItemInListBoxAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let Some(control) = sender.and_then(object_of).and_then(|sender| sender.cast::<Control>()) else {
            return Some(Rc::new(false));
        };

        let Some(items_control) = control.find_logical_ancestor_of_type::<ItemsControl>(true) else {
            return Some(Rc::new(false));
        };

        let items_source = items_control.items_source();
        if let Some(removed) = items_source.as_ref().and_then(|items_source| {
            remove_from_items_source(items_source, &control.data_context())
        }) {
            return Some(Rc::new(removed));
        } else if let Some(list_box) = items_control.clone().cast::<ListBox>() {
            if let Some(list_box_item) = control.find_logical_ancestor_of_type::<ListBoxItem>(true) {
                let list_items = list_box.items();
                let data = list_box_item.data_context();
                // Deviation (DEVIATIONS.md, Behaviors): the item collection of the reference
                // throws when it is the read-only view of an items source; the port does not
                // remove from it and returns false.
                if !list_items.is_read_only() && list_items.contains(&data) {
                    list_items.remove(&data);
                    return Some(Rc::new(true));
                }
            }
        }

        Some(Rc::new(false))
    }
}

impl RemoveItemInListBoxAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// Removes `data` from the items source when it is a list the port can change (`IList` that is not
/// read only in the reference). Returns `None` when the items source is not such a list, else
/// whether the item was removed.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference removes the item from any `IList`; an items
/// source of the framework is read only, so the port removes it from the lists whose type it knows:
/// a `FerroList` of untyped values, texts, numbers, booleans, characters or objects.
fn remove_from_items_source(items_source: &ItemsSource, data: &Option<BoxedValue>) -> Option<bool> {
    macro_rules! remove_from {
        ($($item:ty),* $(,)?) => {$(
            if let Some(list) = items_source.downcast_ref::<FerroList<$item>>() {
                let index = items_source.index_of(data);
                if index < 0 {
                    return Some(false);
                }
                list.remove_at(index as usize);
                return Some(true);
            }
        )*};
    }

    remove_from!(
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
