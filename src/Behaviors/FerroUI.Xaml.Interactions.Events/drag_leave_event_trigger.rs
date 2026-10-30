//! Port of `DragLeaveEventTrigger.cs`.

use ferroui_base::input::{DragDrop, DragEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventHandlerToken};
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveTriggerBase, StyledElementBehaviorImpl};
use std::cell::Cell;
use std::rc::Rc;

/// Trigger that listens for the [`DragDrop::drag_leave_event`].
///
/// The class is sealed in the reference.
#[repr(C)]
pub struct DragLeaveEventTrigger {
    base: InteractiveTriggerBase,
    // The handlers added to the associated object (the reference removes its delegates).
    on_drag_leave_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(DragLeaveEventTrigger: InteractiveTriggerBase);
ferroui_base::ferro_class_info!(DragLeaveEventTrigger { new: DragLeaveEventTrigger::new });
ferroui_base::ferro_impl_classes!(DragLeaveEventTrigger: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for DragLeaveEventTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let trigger = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                DragDrop::drag_leave_event(),
                move |_sender: &Interactive, e: &DragEventArgs| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.on_drag_leave(e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.on_drag_leave_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.on_drag_leave_handler.take() {
                associated_object.remove_handler(DragDrop::drag_leave_event(), handler);
            }
        }
    }
}

impl DragLeaveEventTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveTriggerBase::construct(), on_drag_leave_handler: Cell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn on_drag_leave(&self, e: &dyn IRoutedEventArgs) {
        // The args are the parameter of the actions (a shared handle to the live args).
        let parameter: BoxedValue = Rc::new(e.share());
        self.execute(Some(&parameter));
    }
}
