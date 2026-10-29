//! Port of `Button/InvokeButtonClickAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::automation::peers::ButtonAutomationPeer;
use ferroui_controls::Button;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Invokes the target button when executed.
#[repr(C)]
pub struct InvokeButtonClickAction {
    base: StyledElementAction,
}

ferro_class!(InvokeButtonClickAction: StyledElementAction);
ferro_class_info!(InvokeButtonClickAction {
    new: InvokeButtonClickAction::new,
    markup: {
        property_attributes: [
            TargetButton: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(InvokeButtonClickAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl InvokeButtonClickAction {
        /// Identifies the `TargetButton` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the button, so the value
        /// is an element reference.
        pub fn target_button_property() -> StyledProperty<Option<ElementRef<Button>>> {
            ValueTypes::register_element_ref::<Button>();
            FerroProperty::register::<InvokeButtonClickAction, _>("TargetButton", None)
        }
    }
}

impl StyledElementActionImpl for InvokeButtonClickAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let button = this.target_button().or_else(|| crate::part_c::sender_as::<Button>(sender));
        let Some(button) = button.filter(|button| button.is_effectively_enabled()) else {
            return Some(Rc::new(false));
        };

        let automation_peer = ButtonAutomationPeer::new(&button);
        // The button is effectively enabled, so the peer does not refuse the invocation.
        let _ = automation_peer.invoke();
        Some(Rc::new(true))
    }
}

impl InvokeButtonClickAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target button. This is a property of the object model; markup resolves a
    /// name given for it to the named button.
    pub fn target_button(&self) -> Option<Ref<Button>> {
        ElementRef::resolve(&self.get_value(Self::target_button_property()))
    }

    pub fn set_target_button(&self, value: impl Into<Nullable<Button>>) {
        self.set_value(Self::target_button_property(), ElementRef::from_nullable(value.into().0))
    }
}
