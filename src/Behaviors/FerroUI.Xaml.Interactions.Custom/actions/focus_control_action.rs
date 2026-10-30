//! Port of `Actions/FocusControlAction.cs`.

use super::control_of;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::{ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl, FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty};
use ferroui_controls::Control;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// Focuses the target control (or the sender), in a job of the dispatcher.
#[repr(C)]
pub struct FocusControlAction {
    base: StyledElementAction,
}

ferro_class!(FocusControlAction: StyledElementAction);
ferro_class_info!(FocusControlAction {
    new: FocusControlAction::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(FocusControlAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl FocusControlAction {
        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the action does not own its target, so the value
        /// is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<FocusControlAction, _>("TargetControl", None)
        }
    }
}

impl StyledElementActionImpl for FocusControlAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let control = this.target_control().or_else(|| control_of(sender));
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(control) = control {
                    control.focus();
                }
            },
            DispatcherPriority::DEFAULT,
        );
        None
    }
}

impl FocusControlAction {
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
}
