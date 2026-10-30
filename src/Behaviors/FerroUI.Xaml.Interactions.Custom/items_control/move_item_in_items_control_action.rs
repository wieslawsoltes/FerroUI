//! Port of `ItemsControl/MoveItemInItemsControlAction.cs`.

use super::writable_items::writable_items;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::{Control, ItemsControl};
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Moves an item within an [`ItemsControl`] from [`from_index`](Self::from_index) to
/// [`to_index`](Self::to_index).
///
/// The items control is the [`items_control`](Self::items_control), or else the first items
/// control among the sender (a control) and its logical ancestors. A negative target index moves
/// the item to the end. The action returns true when the item was moved; else false.
#[repr(C)]
pub struct MoveItemInItemsControlAction {
    base: StyledElementAction,
}

ferro_class!(MoveItemInItemsControlAction: StyledElementAction);
ferro_class_info!(MoveItemInItemsControlAction {
    new: MoveItemInItemsControlAction::new,
    markup: {
        property_attributes: [
            ItemsControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(MoveItemInItemsControlAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl MoveItemInItemsControlAction {
        /// Identifies the `ItemsControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the items control is held by an element reference
        /// (a weak reference), as the framework holds the elements a property names.
        pub fn items_control_property() -> StyledProperty<Option<ElementRef<ItemsControl>>> {
            ValueTypes::register_element_ref::<ItemsControl>();
            FerroProperty::register::<MoveItemInItemsControlAction, _>("ItemsControl", None)
        }

        /// Identifies the `FromIndex` property.
        pub fn from_index_property() -> StyledProperty<i32> {
            FerroProperty::register::<MoveItemInItemsControlAction, _>("FromIndex", 0)
        }

        /// Identifies the `ToIndex` property.
        pub fn to_index_property() -> StyledProperty<i32> {
            FerroProperty::register::<MoveItemInItemsControlAction, _>("ToIndex", 0)
        }
    }
}

impl StyledElementActionImpl for MoveItemInItemsControlAction {
    /// Executes the action.
    ///
    /// Returns true if the item was moved; else false.
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(this.execute_core(sender)))
    }
}

impl MoveItemInItemsControlAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets items control. Markup resolves a name given for it to the named element.
    pub fn items_control(&self) -> Option<Ref<ItemsControl>> {
        ElementRef::resolve(&self.get_value(Self::items_control_property()))
    }

    pub fn set_items_control(&self, value: Option<Ref<ItemsControl>>) {
        self.set_value(Self::items_control_property(), ElementRef::from_nullable(value))
    }

    /// Gets or sets source index.
    pub fn from_index(&self) -> i32 {
        self.get_value(Self::from_index_property())
    }

    pub fn set_from_index(&self, value: i32) {
        self.set_value(Self::from_index_property(), value)
    }

    /// Gets or sets target index.
    pub fn to_index(&self) -> i32 {
        self.get_value(Self::to_index_property())
    }

    pub fn set_to_index(&self, value: i32) {
        self.set_value(Self::to_index_property(), value)
    }

    fn execute_core(&self, sender: Option<&BoxedValue>) -> bool {
        if !self.is_enabled() {
            return false;
        }

        let items_control = self.items_control().or_else(|| {
            sender
                .and_then(object_of)
                .and_then(|sender| sender.cast::<Control>())
                .and_then(|control| control.find_logical_ancestor_of_type::<ItemsControl>(true))
        });

        let Some(items_source) = items_control.and_then(|items_control| items_control.items_source()) else {
            return false;
        };
        let Some(list) = writable_items(&items_source) else {
            return false;
        };

        let count = list.count() as i64;
        let from_index = self.from_index() as i64;
        let to_index = self.to_index() as i64;
        let target_index = if to_index < 0 { count - 1 } else { to_index };

        if from_index >= 0
            && from_index < count
            && target_index >= 0
            && target_index < count
            && from_index != target_index
        {
            let item = items_source.get_at(from_index as usize);
            list.remove_at(from_index as usize);
            return list.insert(target_index as usize, &item);
        }

        false
    }
}
