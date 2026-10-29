//! Port of `Collections/AddRangeAction.cs`.

use crate::part_b::items_control::writable_items::writable_items;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};
use ferroui_controls::ItemsSource;
use ferroui_xaml_interactivity::IAction;
use std::rc::Rc;

/// Adds a range of items to a target list when invoked.
///
/// The action returns true when the items were added; else false.
#[repr(C)]
pub struct AddRangeAction {
    base: FerroObject,
}

ferro_class!(AddRangeAction: FerroObject);
ferro_class_info!(AddRangeAction {
    new: AddRangeAction::new,
    markup: {
        content: Items,
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(AddRangeAction: FerroObjectImpl);

ferro_properties! {
    impl AddRangeAction {
        /// Identifies the `Items` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the items (`IEnumerable` of the reference) are an
        /// untyped list of the framework (`ItemsSource`).
        pub fn items_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<AddRangeAction, _>("Items", None)
        }

        /// Identifies the `Target` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the target (`IList` of the reference) is an
        /// untyped list of the framework (`ItemsSource`); the items are added to a `FerroList`
        /// or an `ObservableCollection` of the item types the items actions of the port change
        /// (see `ItemsControl/`), another list is read only.
        pub fn target_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<AddRangeAction, _>("Target", None)
        }
    }
}

impl AddRangeAction {
    /// The static constructor: the class implements the action contract.
    fn static_constructor() {
        <dyn IAction>::register::<AddRangeAction>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the items to add.
    pub fn items(&self) -> Option<ItemsSource> {
        self.get_value(Self::items_property())
    }

    pub fn set_items(&self, value: Option<ItemsSource>) {
        self.set_value(Self::items_property(), value)
    }

    /// Gets or sets the collection to add items to.
    pub fn target(&self) -> Option<ItemsSource> {
        self.get_value(Self::target_property())
    }

    pub fn set_target(&self, value: Option<ItemsSource>) {
        self.set_value(Self::target_property(), value)
    }

    fn execute_core(&self) -> bool {
        let (Some(target), Some(items)) = (self.target(), self.items()) else {
            return false;
        };
        let Some(list) = writable_items(&target) else {
            return false;
        };

        // The items are read before any is added: the target may be the list of the items.
        let items: Vec<Option<BoxedValue>> = items.iter().collect();
        for item in &items {
            list.add(item);
        }

        true
    }
}

impl IAction for AddRangeAction {
    /// Executes the action.
    ///
    /// Returns true if the items were added; else false.
    fn execute(&self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(self.execute_core()))
    }
}
