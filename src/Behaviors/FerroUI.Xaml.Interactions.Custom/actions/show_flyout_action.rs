//! Port of `Actions/ShowFlyoutAction.cs`.

use super::control_of;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl, FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty};
use ferroui_controls::Control;
use ferroui_controls::primitives::FlyoutBase;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Shows the flyout of the target control (or of the sender) at that control.
#[repr(C)]
pub struct ShowFlyoutAction {
    base: StyledElementAction,
}

ferro_class!(ShowFlyoutAction: StyledElementAction);
ferro_class_info!(ShowFlyoutAction {
    new: ShowFlyoutAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(ShowFlyoutAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl ShowFlyoutAction {
        /// Identifies the `Flyout` property.
        pub fn flyout_property() -> StyledProperty<Option<Ref<FlyoutBase>>> {
            FerroProperty::register::<ShowFlyoutAction, _>("Flyout", None)
        }

        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own its target, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<ShowFlyoutAction, _>("TargetControl", None)
        }
    }
}

impl StyledElementActionImpl for ShowFlyoutAction {
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

        flyout.show_at(&control);

        Some(Rc::new(true))
    }
}

impl ShowFlyoutAction {
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
