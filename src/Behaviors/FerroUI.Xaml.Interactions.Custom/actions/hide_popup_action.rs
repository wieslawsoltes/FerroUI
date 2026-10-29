//! Port of `Actions/HidePopupAction.cs`.

use super::control_of;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl, FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty};
use ferroui_controls::Control;
use ferroui_controls::primitives::Popup;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Closes the popup.
#[repr(C)]
pub struct HidePopupAction {
    base: StyledElementAction,
}

ferro_class!(HidePopupAction: StyledElementAction);
ferro_class_info!(HidePopupAction {
    new: HidePopupAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(HidePopupAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl HidePopupAction {
        /// Identifies the `Popup` property.
        pub fn popup_property() -> StyledProperty<Option<Ref<Popup>>> {
            FerroProperty::register::<HidePopupAction, _>("Popup", None)
        }

        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own its target, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<HidePopupAction, _>("TargetControl", None)
        }
    }
}

impl StyledElementActionImpl for HidePopupAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let control = this.target_control().or_else(|| control_of(sender));
        let popup = this.popup();
        let (Some(_control), Some(popup)) = (control, popup) else {
            return Some(Rc::new(false));
        };

        popup.close();
        Some(Rc::new(true))
    }
}

impl HidePopupAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target control. If not set, the sender is used. Markup resolves a name
    /// given for it to the named element.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the popup. This is a property of the object model.
    pub fn popup(&self) -> Option<Ref<Popup>> {
        self.get_value(Self::popup_property())
    }

    pub fn set_popup(&self, value: impl Into<Nullable<Popup>>) {
        self.set_value(Self::popup_property(), value.into().0)
    }
}
