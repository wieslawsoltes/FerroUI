//! Port of `ItemsControl/ClearItemsControlAction.cs`.

use super::writable_items::writable_items;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::ItemsControl;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Allows a user to clear the items from a [`ItemsControl`].
///
/// The action returns true when the items source of the items control was cleared; else false.
#[repr(C)]
pub struct ClearItemsControlAction {
    base: StyledElementAction,
}

ferro_class!(ClearItemsControlAction: StyledElementAction);
ferro_class_info!(ClearItemsControlAction {
    new: ClearItemsControlAction::new,
    markup: {
        property_attributes: [
            ItemsControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ClearItemsControlAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ClearItemsControlAction {
        /// Identifies the `ItemsControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the reference registers this property with
        /// `InsertItemToItemsControlAction` as its owner; here its owner is this class (defect
        /// corrected). The items control is held by an element reference (a weak reference), as
        /// the framework holds the elements a property names.
        pub fn items_control_property() -> StyledProperty<Option<ElementRef<ItemsControl>>> {
            ValueTypes::register_element_ref::<ItemsControl>();
            FerroProperty::register::<ClearItemsControlAction, _>("ItemsControl", None)
        }
    }
}

impl StyledElementActionImpl for ClearItemsControlAction {
    /// Executes the action.
    ///
    /// Returns true if the items were cleared; else false.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(this.execute_core()))
    }
}

impl ClearItemsControlAction {
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

    fn execute_core(&self) -> bool {
        if !self.is_enabled() {
            return false;
        }

        let Some(items_control) = self.items_control() else {
            return false;
        };

        if let Some(items_source) = items_control.items_source() {
            if let Some(list) = writable_items(&items_source) {
                list.clear();
                return true;
            }
        }

        false
    }
}
