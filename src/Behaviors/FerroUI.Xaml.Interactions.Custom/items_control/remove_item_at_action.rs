//! Port of `ItemsControl/RemoveItemAtAction.cs`.

use super::writable_items::writable_items;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use ferroui_controls::ItemsControl;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::IAction;
use std::rc::Rc;

/// Removes an item at a specified index from an [`ItemsControl`]: the
/// [`items_control`](Self::items_control), or else the sender.
///
/// The action returns true when the item was removed; else false.
#[repr(C)]
pub struct RemoveItemAtAction {
    base: FerroObject,
}

ferro_class!(RemoveItemAtAction: FerroObject);
ferro_class_info!(RemoveItemAtAction {
    new: RemoveItemAtAction::new,
    markup: {
        property_attributes: [
            ItemsControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(RemoveItemAtAction: FerroObjectImpl);

ferro_properties! {
    impl RemoveItemAtAction {
        /// Identifies the `ItemsControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the items control is held by an element reference
        /// (a weak reference), as the framework holds the elements a property names.
        pub fn items_control_property() -> StyledProperty<Option<ElementRef<ItemsControl>>> {
            ValueTypes::register_element_ref::<ItemsControl>();
            FerroProperty::register::<RemoveItemAtAction, _>("ItemsControl", None)
        }

        /// Identifies the `Index` property.
        pub fn index_property() -> StyledProperty<i32> {
            FerroProperty::register::<RemoveItemAtAction, _>("Index", 0)
        }
    }
}

impl RemoveItemAtAction {
    /// The static constructor: the class implements the action contract.
    fn static_constructor() {
        <dyn IAction>::register::<RemoveItemAtAction>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
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

    /// Gets or sets index to remove.
    pub fn index(&self) -> i32 {
        self.get_value(Self::index_property())
    }

    pub fn set_index(&self, value: i32) {
        self.set_value(Self::index_property(), value)
    }

    fn execute_core(&self, sender: Option<&BoxedValue>) -> bool {
        let items_control = self
            .items_control()
            .or_else(|| sender.and_then(object_of).and_then(|sender| sender.cast::<ItemsControl>()));

        if let Some(items_source) = items_control.and_then(|items_control| items_control.items_source()) {
            if let Some(list) = writable_items(&items_source) {
                let index = self.index();
                if index >= 0 && (index as usize) < list.count() {
                    list.remove_at(index as usize);
                    return true;
                }
            }
        }

        false
    }
}

impl IAction for RemoveItemAtAction {
    /// Executes the action.
    ///
    /// Returns true if the item was removed; else false.
    fn execute(&self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(self.execute_core(sender)))
    }
}
