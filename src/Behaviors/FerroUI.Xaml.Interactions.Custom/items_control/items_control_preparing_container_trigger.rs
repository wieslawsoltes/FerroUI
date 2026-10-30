//! Port of `ItemsControl/ItemsControlPreparingContainerTrigger.cs`.

use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType, StyledElementImpl,
};
use ferroui_controls::{ContainerPreparedEventArgs, ItemsControl};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::cell::RefCell;
use std::rc::Rc;

/// A behavior that listens for the [`ItemsControl::preparing_container`] event (`PreparingContainer`) on its source
/// and executes its actions when that event is fired.
///
/// The args of the event are the parameter of the actions.
#[repr(C)]
pub struct ItemsControlPreparingContainerTrigger {
    base: StyledElementTriggerOfT,
    // The subscription to the event of the associated object (the reference removes its delegate).
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(ItemsControlPreparingContainerTrigger: StyledElementTriggerOfT);
ferro_class_info!(ItemsControlPreparingContainerTrigger { new: ItemsControlPreparingContainerTrigger::new });
ferroui_base::ferro_impl_classes!(ItemsControlPreparingContainerTrigger: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ItemsControlPreparingContainerTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            if let Some(associated_object) = this.associated_object_as::<ItemsControl>() {
                let trigger = this.to_ref().downgrade();
                let subscription = associated_object.preparing_container(move |e: &ContainerPreparedEventArgs| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.items_control_on_preparing_container(e);
                    }
                });
                let previous = this.subscription.borrow_mut().replace(subscription);
                if let Some(previous) = previous {
                    previous.dispose();
                }
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let subscription = this.subscription.borrow_mut().take();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
        }
    }
}

impl ItemsControlPreparingContainerTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementTriggerOfT::construct(<ItemsControl as StaticType>::TYPE),
            subscription: RefCell::new(None),
        }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn items_control_on_preparing_container(&self, e: &ContainerPreparedEventArgs) {
        let parameter: BoxedValue = Rc::new(e.clone());
        self.execute(Some(&parameter));
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        if !self.is_enabled() {
            return;
        }

        if let Some(associated_object) = self.associated_object() {
            let sender: BoxedValue = Rc::new(associated_object);
            Interaction::execute_actions(Some(&sender), self.actions().as_ref(), parameter);
        }
    }
}
