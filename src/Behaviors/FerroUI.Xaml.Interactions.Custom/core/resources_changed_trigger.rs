//! Port of `Core/ResourcesChangedTrigger.cs`.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType, StyledElement, StyledElementImpl};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::rc::Rc;

/// A trigger that executes its actions when the resources of the associated object changed.
///
/// The actions run in a job of the dispatcher, with no parameter.
#[repr(C)]
pub struct ResourcesChangedTrigger {
    base: StyledElementTriggerOfT,
}

ferro_class!(ResourcesChangedTrigger: StyledElementTriggerOfT);
ferro_class_info!(ResourcesChangedTrigger { new: ResourcesChangedTrigger::new });
ferroui_base::ferro_impl_classes!(ResourcesChangedTrigger: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ResourcesChangedTrigger {
        fn on_resources_changed_event(this: &Self) {
            let trigger: Ref<Self> = this.to_ref();
            Dispatcher::ui_thread().post_local(move || trigger.execute(None), DispatcherPriority::DEFAULT);
        }
    }
}

impl ResourcesChangedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<StyledElement as StaticType>::TYPE) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        Interaction::execute_actions(sender.as_ref(), self.actions().as_ref(), parameter);
    }
}
