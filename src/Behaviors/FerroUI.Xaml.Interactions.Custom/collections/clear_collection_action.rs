//! Port of `Collections/ClearCollectionAction.cs`.

use crate::part_b::items_control::writable_items::writable_items;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroProperty, Ref, StyledProperty,
};
use ferroui_controls::ItemsSource;
use ferroui_xaml_interactivity::IAction;
use std::rc::Rc;

/// Clears all items from a target list when invoked.
///
/// The action returns true when the list was cleared; else false.
#[repr(C)]
pub struct ClearCollectionAction {
    base: FerroObject,
}

ferro_class!(ClearCollectionAction: FerroObject);
ferro_class_info!(ClearCollectionAction {
    new: ClearCollectionAction::new,
    markup: {
        property_attributes: [
            Target: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ClearCollectionAction: FerroObjectImpl);

ferro_properties! {
    impl ClearCollectionAction {
        /// Identifies the `Target` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the target (`IList` of the reference) is an
        /// untyped list of the framework (`ItemsSource`); a `FerroList` or an
        /// `ObservableCollection` of the item types the items actions of the port change (see
        /// `ItemsControl/`) is cleared, another list is read only.
        pub fn target_property() -> StyledProperty<Option<ItemsSource>> {
            FerroProperty::register::<ClearCollectionAction, _>("Target", None)
        }
    }
}

impl ClearCollectionAction {
    /// The static constructor: the class implements the action contract.
    fn static_constructor() {
        <dyn IAction>::register::<ClearCollectionAction>();
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the collection to clear.
    pub fn target(&self) -> Option<ItemsSource> {
        self.get_value(Self::target_property())
    }

    pub fn set_target(&self, value: Option<ItemsSource>) {
        self.set_value(Self::target_property(), value)
    }

    fn execute_core(&self) -> bool {
        let Some(target) = self.target() else {
            return false;
        };
        let Some(list) = writable_items(&target) else {
            return false;
        };

        list.clear();
        true
    }
}

impl IAction for ClearCollectionAction {
    /// Executes the action.
    ///
    /// Returns true if the collection was cleared; else false.
    fn execute(&self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(self.execute_core()))
    }
}
