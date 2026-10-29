//! Port of `Transitions/AddTransitionAction.cs`.

use ferroui_base::animation::TransitionBase;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElement, StyledElementImpl, StyledProperty,
};
use ferroui_xaml_behaviors_animations::TransitionOperations;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Adds a [`TransitionBase`] to the transitions collection on the target element.
#[repr(C)]
pub struct AddTransitionAction {
    base: StyledElementAction,
}

ferro_class!(AddTransitionAction: StyledElementAction);
ferro_class_info!(AddTransitionAction {
    new: AddTransitionAction::new,
    markup: {
        property_attributes: [
            StyledElement: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(AddTransitionAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl AddTransitionAction {
        /// Identifies the `Transition` property.
        pub fn transition_property() -> StyledProperty<Option<Ref<TransitionBase>>> {
            FerroProperty::register::<AddTransitionAction, _>("Transition", None)
        }

        /// Identifies the `StyledElement` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the element, so the value
        /// is an element reference.
        pub fn styled_element_property() -> StyledProperty<Option<ElementRef<StyledElement>>> {
            ValueTypes::register_element_ref::<StyledElement>();
            FerroProperty::register::<AddTransitionAction, _>("StyledElement", None)
        }
    }
}

impl StyledElementActionImpl for AddTransitionAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let target = this.styled_element().or_else(|| crate::part_c::sender_as::<StyledElement>(sender));
        let transition = this.transition();
        Some(Rc::new(TransitionOperations::add(target.as_deref(), transition.as_ref())))
    }
}

impl AddTransitionAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the transition to add. This is a property of the object model.
    pub fn transition(&self) -> Option<Ref<TransitionBase>> {
        self.get_value(Self::transition_property())
    }

    pub fn set_transition(&self, value: Option<Ref<TransitionBase>>) {
        self.set_value(Self::transition_property(), value)
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
