//! Port of `Core/AttachedToVisualTreeTrigger.cs`.

use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType, StyledElementImpl, Visual};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::rc::Rc;

/// A trigger that executes when the associated object is attached to the visual tree.
///
/// The actions run with no parameter.
#[repr(C)]
pub struct AttachedToVisualTreeTrigger {
    base: StyledElementTriggerOfT,
}

ferro_class!(AttachedToVisualTreeTrigger: StyledElementTriggerOfT);
ferro_class_info!(AttachedToVisualTreeTrigger { new: AttachedToVisualTreeTrigger::new });
ferroui_base::ferro_impl_classes!(AttachedToVisualTreeTrigger: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for AttachedToVisualTreeTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            this.execute(None);
        }
    }
}

impl AttachedToVisualTreeTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<Visual as StaticType>::TYPE) }
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
