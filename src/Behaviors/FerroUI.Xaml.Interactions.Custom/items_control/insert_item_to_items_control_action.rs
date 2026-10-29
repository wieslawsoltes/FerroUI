//! Port of `ItemsControl/InsertItemToItemsControlAction.cs`.

use super::writable_items::writable_items;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::ITemplate;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::ItemsControl;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Allows a user to insert the item to a [`ItemsControl`].
///
/// The action returns true when the item was inserted in the items source of the items control;
/// else false.
#[repr(C)]
pub struct InsertItemToItemsControlAction {
    base: StyledElementAction,
}

ferro_class!(InsertItemToItemsControlAction: StyledElementAction);
ferro_class_info!(InsertItemToItemsControlAction {
    new: InsertItemToItemsControlAction::new,
    markup: {
        content: Item,
        property_attributes: [
            ItemsControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(InsertItemToItemsControlAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl InsertItemToItemsControlAction {
        /// Identifies the `ItemsControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the items control is held by an element reference
        /// (a weak reference), as the framework holds the elements a property names.
        pub fn items_control_property() -> StyledProperty<Option<ElementRef<ItemsControl>>> {
            ValueTypes::register_element_ref::<ItemsControl>();
            FerroProperty::register::<InsertItemToItemsControlAction, _>("ItemsControl", None)
        }

        /// Identifies the `Item` property.
        pub fn item_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<InsertItemToItemsControlAction, _>("Item", None)
        }

        /// Identifies the `Index` property.
        pub fn index_property() -> StyledProperty<i32> {
            FerroProperty::register::<InsertItemToItemsControlAction, _>("Index", 0)
        }
    }
}

impl StyledElementActionImpl for InsertItemToItemsControlAction {
    /// Executes the action.
    ///
    /// Returns true if the item was inserted; else false.
    fn execute(this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(this.execute_core()))
    }
}

impl InsertItemToItemsControlAction {
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

    /// Gets or sets item to insert. A template ([`ITemplate`]) inserts the object it builds.
    pub fn item(&self) -> Option<BoxedValue> {
        self.get_value(Self::item_property())
    }

    pub fn set_item(&self, value: Option<BoxedValue>) {
        self.set_value(Self::item_property(), value)
    }

    /// Gets or sets item index to insert.
    pub fn index(&self) -> i32 {
        self.get_value(Self::index_property())
    }

    pub fn set_index(&self, value: i32) {
        self.set_value(Self::index_property(), value)
    }

    fn execute_core(&self) -> bool {
        if !self.is_enabled() {
            return false;
        }

        let Some(mut item) = self.item() else {
            return false;
        };

        if let Some(template) = from_markup_value::<Rc<dyn ITemplate>>(&Some(item.clone())) {
            item = template.build();
        }

        let Some(items_control) = self.items_control() else {
            return false;
        };

        if let Some(items_source) = items_control.items_source() {
            if let Some(list) = writable_items(&items_source) {
                // Deviation (DEVIATIONS.md, Behaviors): an index out of the range of the list
                // inserts nothing and the action returns false; the reference throws.
                let Ok(index) = usize::try_from(self.index()) else {
                    return false;
                };
                return list.insert(index, &Some(item));
            }
        }

        false
    }
}
