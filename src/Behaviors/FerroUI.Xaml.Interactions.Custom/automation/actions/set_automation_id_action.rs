//! Port of `Automation/Actions/SetAutomationIdAction.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::Control;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Sets the `AutomationProperties.AutomationId` property on the target control when executed.
#[repr(C)]
pub struct SetAutomationIdAction {
    base: StyledElementAction,
}

ferro_class!(SetAutomationIdAction: StyledElementAction);
ferro_class_info!(SetAutomationIdAction {
    new: SetAutomationIdAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(SetAutomationIdAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SetAutomationIdAction {
        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own the control, so the
        /// value is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<SetAutomationIdAction, _>("TargetControl", None)
        }

        /// Identifies the `AutomationId` property.
        pub fn automation_id_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<SetAutomationIdAction, _>("AutomationId", None)
        }
    }
}

impl StyledElementActionImpl for SetAutomationIdAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let control = this
            .target_control()
            .or_else(|| sender.and_then(object_of).and_then(|sender| sender.cast::<Control>()));
        let Some(control) = control else {
            return Some(Rc::new(false));
        };

        AutomationProperties::set_automation_id(&control, this.automation_id().as_deref());
        Some(Rc::new(true))
    }
}

impl SetAutomationIdAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the target control. Markup resolves a name given for it to the named element.
    /// This is a property of the object model.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the automation id value. This is a property of the object model.
    pub fn automation_id(&self) -> Option<String> {
        self.get_value(Self::automation_id_property())
    }

    pub fn set_automation_id(&self, value: Option<String>) {
        self.set_value(Self::automation_id_property(), value)
    }
}
