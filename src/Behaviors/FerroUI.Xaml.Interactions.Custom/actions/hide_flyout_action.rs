//! Port of `Actions/HideFlyoutAction.cs`.

use super::control_of;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl, FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty};
use ferroui_controls::Control;
use ferroui_controls::primitives::FlyoutBase;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Hides the flyout of the target control (or of the sender).
#[repr(C)]
pub struct HideFlyoutAction {
    base: StyledElementAction,
}

ferro_class!(HideFlyoutAction: StyledElementAction);
ferro_class_info!(HideFlyoutAction {
    new: HideFlyoutAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(HideFlyoutAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl HideFlyoutAction {
        /// Identifies the `Flyout` property.
        pub fn flyout_property() -> StyledProperty<Option<Ref<FlyoutBase>>> {
            FerroProperty::register::<HideFlyoutAction, _>("Flyout", None)
        }

        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own its target, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<HideFlyoutAction, _>("TargetControl", None)
        }
    }
}

impl StyledElementActionImpl for HideFlyoutAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let control = this.target_control().or_else(|| control_of(sender));
        let Some(control) = control else {
            return Some(Rc::new(false));
        };

        let Some(flyout) = this.flyout().or_else(|| FlyoutBase::get_attached_flyout(&control)) else {
            return Some(Rc::new(false));
        };

        flyout.hide();

        Some(Rc::new(true))
    }
}

impl HideFlyoutAction {
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

    /// Gets or sets the flyout. If not set, the flyout attached to the control is used. This is a
    /// property of the object model.
    pub fn flyout(&self) -> Option<Ref<FlyoutBase>> {
        self.get_value(Self::flyout_property())
    }

    pub fn set_flyout(&self, value: impl Into<Nullable<FlyoutBase>>) {
        self.set_value(Self::flyout_property(), value.into().0)
    }
}
