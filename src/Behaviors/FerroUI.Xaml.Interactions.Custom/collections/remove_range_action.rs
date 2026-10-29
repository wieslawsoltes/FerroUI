//! Port of `Collections/RemoveRangeAction.cs`.

use crate::part_b::items_control::writable_items::writable_items;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};
use ferroui_controls::ItemsSource;
use ferroui_xaml_interactivity::IAction;
use std::rc::Rc;

/// Removes a range of items from a target list when invoked.
///
/// The action returns true when the items were removed (each item the list contains); else
/// false.
#[repr(C)]
pub struct RemoveRangeAction {
    base: FerroObject,
}

ferro_class!(RemoveRangeAction: FerroObject);
ferro_class_info!(RemoveRangeAction {
    new: RemoveRangeAction::new,
    markup: {
        content: Items,
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(RemoveRangeAction: FerroObjectImpl);

ferro_properties! {
    impl RemoveRangeAction {
        /// Identifies the `Items` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the items (`IEnumerable` of the reference) are an
        /// untyped list of the framework (`ItemsSource`).
        pub fn items_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<RemoveRangeAction, _>("Items", None)
        }

        /// Identifies the `Target` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the target (`IList` of the reference) is an
        /// untyped list of the framework (`ItemsSource`); the items are removed from a
        /// `FerroList` or an `ObservableCollection` of the item types the items actions of the
        /// port change (see `ItemsControl/`), another list is read only.
        pub fn target_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<RemoveRangeAction, _>("Target", None)
        }
    }
}

impl RemoveRangeAction {
    /// The static constructor: the class implements the action contract.
    fn static_constructor() {
        <dyn IAction>::register::<RemoveRangeAction>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the items to remove.
    pub fn items(&self) -> Option<ItemsSource> {
        self.get_value(Self::items_property())
    }

    pub fn set_items(&self, value: Option<ItemsSource>) {
        self.set_value(Self::items_property(), value)
    }

    /// Gets or sets the collection to remove items from.
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

        // The items are read before any is removed (`Items.Cast<object>().ToList()`).
        let items: Vec<Option<BoxedValue>> = items.iter().collect();

        for item in &items {
            let index = target.index_of(item);
            if index >= 0 {
                list.remove_at(index as usize);
            }
        }

        true
    }
}

impl IAction for RemoveRangeAction {
    /// Executes the action.
    ///
    /// Returns true if the target and the items are set; else false.
    fn execute(&self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(self.execute_core()))
    }
}
