//! Port of `Transitions/ClearTransitionsAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElement, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_behaviors_animations::TransitionOperations;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Clears the transitions collection.
#[repr(C)]
pub struct ClearTransitionsAction {
    base: StyledElementAction,
}

ferro_class!(ClearTransitionsAction: StyledElementAction);
ferro_class_info!(ClearTransitionsAction {
    new: ClearTransitionsAction::new,
    markup: {
        property_attributes: [
            StyledElement: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ClearTransitionsAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ClearTransitionsAction {
        /// Identifies the `StyledElement` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the element, so the value
        /// is an element reference.
        pub fn styled_element_property() -> StyledProperty<Option<ElementRef<StyledElement>>> {
            ValueTypes::register_element_ref::<StyledElement>();
            FerroProperty::register::<ClearTransitionsAction, _>("StyledElement", None)
        }
    }
}

impl StyledElementActionImpl for ClearTransitionsAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let target = this.styled_element().or_else(|| crate::part_c::sender_as::<StyledElement>(sender));
        Some(Rc::new(TransitionOperations::clear(target.as_deref())))
    }
}

impl ClearTransitionsAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target styled element. This is a property of the object model; markup
    /// resolves a name given for it to the named element.
    pub fn styled_element(&self) -> Option<Ref<StyledElement>> {
        ElementRef::resolve(&self.get_value(Self::styled_element_property()))
    }

    pub fn set_styled_element(&self, value: impl Into<Nullable<StyledElement>>) {
        self.set_value(Self::styled_element_property(), ElementRef::from_nullable(value.into().0))
    }
}
