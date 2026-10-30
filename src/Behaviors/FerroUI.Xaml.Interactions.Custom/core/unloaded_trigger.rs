//! Port of `Core/UnloadedTrigger.cs`.

use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType, StyledElementImpl};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::rc::Rc;

/// A trigger that executes its actions when the associated control is unloaded.
///
/// The actions run with no parameter.
#[repr(C)]
pub struct UnloadedTrigger {
    base: StyledElementTriggerOfT,
}

ferro_class!(UnloadedTrigger: StyledElementTriggerOfT);
ferro_class_info!(UnloadedTrigger { new: UnloadedTrigger::new });
ferroui_base::ferro_impl_classes!(UnloadedTrigger: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for UnloadedTrigger {
        fn on_unloaded(this: &Self) {
            this.execute(None);
        }
    }
}

impl UnloadedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<Control as StaticType>::TYPE) }
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
