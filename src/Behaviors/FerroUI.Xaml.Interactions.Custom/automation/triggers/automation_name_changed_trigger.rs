//! Port of `Automation/Triggers/AutomationNameChangedTrigger.cs`.

use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectExtensions, FerroObjectImpl, Ref, StaticType,
    StyledElementImpl,
};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingTrigger, DisposingTriggerImpl, Interaction, StyledElementBehaviorImpl,
};
use std::rc::Rc;

/// Executes actions when the `AutomationProperties.Name` property of the associated control
/// changes (and once when the trigger is attached, as the observable of the property starts with
/// its current value).
#[repr(C)]
pub struct AutomationNameChangedTrigger {
    base: DisposingTrigger,
}

ferro_class!(AutomationNameChangedTrigger: DisposingTrigger);
ferro_class_info!(AutomationNameChangedTrigger { new: AutomationNameChangedTrigger::new });
ferroui_base::ferro_impl_classes!(
    AutomationNameChangedTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl DisposingTriggerImpl for AutomationNameChangedTrigger {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            let trigger = this.to_ref().downgrade();
            FerroObjectExtensions::get_observable(&*associated_object as &ferroui_base::FerroObject, AutomationProperties::name_property()).subscribe_fn(
                move |_name: Option<String>| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.execute();
                    }
                },
            )
        }
    }
}

impl AutomationNameChangedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DisposingTrigger::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn execute(&self) {
        if !self.is_enabled() {
            return;
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), None);
    }
}
