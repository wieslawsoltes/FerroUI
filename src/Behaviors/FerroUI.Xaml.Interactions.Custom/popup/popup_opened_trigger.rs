//! Port of `Popup/PopupOpenedTrigger.cs`.

use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::EventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, instantiate, BoxedValue, FerroObjectImpl, Ref, StaticType, StyledElementImpl,
};
use ferroui_controls::primitives::Popup;
use ferroui_xaml_interactivity::{Interaction, StyledElementBehaviorImpl, StyledElementTriggerOfT};
use std::cell::RefCell;
use std::rc::Rc;

/// Executes actions when the opened event of the associated [`Popup`] is raised.
///
/// The args of the event (`EventArgs.Empty`) are the parameter of the actions.
#[repr(C)]
pub struct PopupOpenedTrigger {
    base: StyledElementTriggerOfT,
    // The subscription of the opened event (`Opened -= OnOpened` of the reference).
    opened: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(PopupOpenedTrigger: StyledElementTriggerOfT);
ferro_class_info!(PopupOpenedTrigger { new: PopupOpenedTrigger::new });
ferroui_base::ferro_impl_classes!(PopupOpenedTrigger: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for PopupOpenedTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            if let Some(associated_object) = this.associated_object_as::<Popup>() {
                this.unsubscribe();
                let trigger = this.to_ref().downgrade();
                let subscription = associated_object.opened(move || {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.on_opened();
                    }
                });
                *this.opened.borrow_mut() = Some(subscription);
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.unsubscribe();
        }
    }
}

impl PopupOpenedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementTriggerOfT::construct(<Popup as StaticType>::TYPE), opened: RefCell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn unsubscribe(&self) {
        let subscription = self.opened.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }

    fn on_opened(&self) {
        let parameter: BoxedValue = Rc::new(EventArgs::EMPTY);
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
