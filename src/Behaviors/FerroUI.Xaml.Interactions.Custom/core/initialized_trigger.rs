//! Port of `Core/InitializedTrigger.cs`.

use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType, StyledElement, StyledElementImpl};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::rc::Rc;

/// A trigger that executes its actions when the associated object is initialized.
///
/// The actions run with no parameter.
#[repr(C)]
pub struct InitializedTrigger {
    base: StyledElementTriggerOfT,
}

ferro_class!(InitializedTrigger: StyledElementTriggerOfT);
ferro_class_info!(InitializedTrigger { new: InitializedTrigger::new });
ferroui_base::ferro_impl_classes!(InitializedTrigger: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for InitializedTrigger {
        fn on_initialized_event(this: &Self) {
            this.execute(None);
        }
    }
}

impl InitializedTrigger {
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
