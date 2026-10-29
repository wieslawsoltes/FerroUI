//! Port of `Actions/SetEnabledAction.cs`.

use super::control_of;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl, FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty};
use ferroui_controls::Control;
use ferroui_base::input::InputElement;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Sets the `IsEnabled` property of the target control (or of the sender).
#[repr(C)]
pub struct SetEnabledAction {
    base: StyledElementAction,
}

ferro_class!(SetEnabledAction: StyledElementAction);
ferro_class_info!(SetEnabledAction {
    new: SetEnabledAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(SetEnabledAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetEnabledAction {
        /// Identifies the `IsEnabledValue` property.
        pub fn is_enabled_value_property() -> StyledProperty<bool> {
            FerroProperty::register::<SetEnabledAction, _>("IsEnabledValue", true)
        }

        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own its target, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<SetEnabledAction, _>("TargetControl", None)
        }
    }
}

impl StyledElementActionImpl for SetEnabledAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let control = this.target_control().or_else(|| control_of(sender));
        let Some(control) = control else {
            return Some(Rc::new(false));
        };

        control.set_current_value(InputElement::is_enabled_property(), this.is_enabled_value());
        Some(Rc::new(true))
    }
}

impl SetEnabledAction {
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

    /// Gets or sets the value assigned to `IsEnabled`. This is a property of the object model.
    pub fn is_enabled_value(&self) -> bool {
        self.get_value(Self::is_enabled_value_property())
    }

    pub fn set_is_enabled_value(&self, value: bool) {
        self.set_value(Self::is_enabled_value_property(), value)
    }
}
