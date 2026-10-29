//! Port of `ItemsControl/ItemsControlContainerIndexChangedTrigger.cs`.

use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType, StyledElementImpl,
};
use ferroui_controls::{ContainerIndexChangedEventArgs, ItemsControl};
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::cell::RefCell;
use std::rc::Rc;

/// A behavior that listens for the [`ItemsControl::container_index_changed`] event
/// (`ContainerIndexChanged`) on its source and executes its actions when that event is fired.
///
/// The args of the event are the parameter of the actions.
#[repr(C)]
pub struct ItemsControlContainerIndexChangedTrigger {
    base: StyledElementTriggerOfT,
    // The subscription to the event of the associated object (the reference removes its delegate).
    subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(ItemsControlContainerIndexChangedTrigger: StyledElementTriggerOfT);
ferro_class_info!(ItemsControlContainerIndexChangedTrigger { new: ItemsControlContainerIndexChangedTrigger::new });
ferroui_base::ferro_impl_classes!(ItemsControlContainerIndexChangedTrigger: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ItemsControlContainerIndexChangedTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            if let Some(associated_object) = this.associated_object_as::<ItemsControl>() {
                let trigger = this.to_ref().downgrade();
                let subscription = associated_object.container_index_changed(move |e: &ContainerIndexChangedEventArgs| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.items_control_on_container_index_changed(e);
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

impl ItemsControlContainerIndexChangedTrigger {
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

    fn items_control_on_container_index_changed(&self, e: &ContainerIndexChangedEventArgs) {
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
