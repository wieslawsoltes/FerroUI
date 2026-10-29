//! Port of `Transitions/TransitionsChangedTrigger.cs`.

use ferroui_base::animation::Transitions;
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType, StyledElementImpl,
};
use ferroui_controls::Control;
use ferroui_xaml_behaviors_animations::TransitionOperations;
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingTrigger, DisposingTriggerImpl, Interaction, StyledElementBehaviorImpl,
};
use std::rc::Rc;

/// Executes actions whenever the transitions collection changes.
#[repr(C)]
pub struct TransitionsChangedTrigger {
    base: DisposingTrigger,
}

ferro_class!(TransitionsChangedTrigger: DisposingTrigger);
ferro_class_info!(TransitionsChangedTrigger { new: TransitionsChangedTrigger::new });
ferroui_base::ferro_impl_classes!(
    TransitionsChangedTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
);

ferroui_base::ferro_overrides! {
    impl DisposingTriggerImpl for TransitionsChangedTrigger {
        fn on_attached_override(this: &Self) -> Rc<dyn IDisposable> {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return DisposableAction::empty();
            };

            let trigger = this.to_ref().downgrade();
            TransitionOperations::observe(&associated_object, move |_transitions: Option<Transitions>| {
                let Some(trigger) = trigger.upgrade() else {
                    return;
                };
                Dispatcher::ui_thread().post_local(move || trigger.execute(None), DispatcherPriority::DEFAULT);
            })
        }
    }
}

impl TransitionsChangedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: DisposingTrigger::construct(<Control as StaticType>::TYPE) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        let Some(associated_object) = self.associated_object() else {
            return;
        };

        let sender: BoxedValue = Rc::new(associated_object);
        Interaction::execute_actions(Some(&sender), self.actions().as_ref(), parameter);
    }
}
