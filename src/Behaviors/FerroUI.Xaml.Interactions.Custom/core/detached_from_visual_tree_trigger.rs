//! Port of `Core/DetachedFromVisualTreeTrigger.cs`.

use ferroui_base::{ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType, StyledElementImpl, Visual};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::rc::Rc;

/// A trigger that executes its actions when the associated object is detached from the visual tree.
///
/// The actions run with no parameter.
#[repr(C)]
pub struct DetachedFromVisualTreeTrigger {
    base: StyledElementTriggerOfT,
}

ferro_class!(DetachedFromVisualTreeTrigger: StyledElementTriggerOfT);
ferro_class_info!(DetachedFromVisualTreeTrigger { new: DetachedFromVisualTreeTrigger::new });
ferroui_base::ferro_impl_classes!(DetachedFromVisualTreeTrigger: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for DetachedFromVisualTreeTrigger {
        fn on_detached_from_visual_tree(this: &Self) {
            this.execute(None);
        }
    }
}

impl DetachedFromVisualTreeTrigger {
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
