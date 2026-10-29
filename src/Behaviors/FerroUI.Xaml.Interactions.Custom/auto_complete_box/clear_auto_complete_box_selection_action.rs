//! Port of `AutoCompleteBox/ClearAutoCompleteBoxSelectionAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::AutoCompleteBox;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Clears the selection and text of an [`AutoCompleteBox`].
#[repr(C)]
pub struct ClearAutoCompleteBoxSelectionAction {
    base: StyledElementAction,
}

ferro_class!(ClearAutoCompleteBoxSelectionAction: StyledElementAction);
ferro_class_info!(ClearAutoCompleteBoxSelectionAction {
    new: ClearAutoCompleteBoxSelectionAction::new,
    markup: {
        property_attributes: [
            AutoCompleteBox: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ClearAutoCompleteBoxSelectionAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ClearAutoCompleteBoxSelectionAction {
        /// Identifies the `AutoCompleteBox` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the box, so the value is
        /// an element reference.
        pub fn auto_complete_box_property() -> StyledProperty<Option<ElementRef<AutoCompleteBox>>> {
            ValueTypes::register_element_ref::<AutoCompleteBox>();
            FerroProperty::register::<ClearAutoCompleteBoxSelectionAction, _>("AutoCompleteBox", None)
        }
    }
}

impl StyledElementActionImpl for ClearAutoCompleteBoxSelectionAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let auto_complete_box = this
            .auto_complete_box()
            .or_else(|| sender.and_then(object_of).and_then(|sender| sender.cast::<AutoCompleteBox>()));
        let Some(auto_complete_box) = auto_complete_box else {
            return Some(Rc::new(false));
        };

        auto_complete_box.set_selected_item(None);
        auto_complete_box.set_text(Some(""));

        None
    }
}

impl ClearAutoCompleteBoxSelectionAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target [`AutoCompleteBox`]. This is a property of the object model. Markup
    /// resolves a name given for it to the named element.
    pub fn auto_complete_box(&self) -> Option<Ref<AutoCompleteBox>> {
        ElementRef::resolve(&self.get_value(Self::auto_complete_box_property()))
    }

    pub fn set_auto_complete_box(&self, value: impl Into<Nullable<AutoCompleteBox>>) {
        self.set_value(Self::auto_complete_box_property(), ElementRef::from_nullable(value.into().0))
    }
}
