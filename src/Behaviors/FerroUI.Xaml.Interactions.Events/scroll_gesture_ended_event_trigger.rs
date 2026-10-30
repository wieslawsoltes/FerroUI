//! Port of `ScrollGestureEndedEventTrigger.cs`.

use ferroui_base::input::{InputElement, ScrollGestureEndedEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventHandlerToken};
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveTriggerBase, StyledElementBehaviorImpl};
use std::cell::Cell;
use std::rc::Rc;

/// Trigger that listens for the [`InputElement::scroll_gesture_ended_event`].
#[repr(C)]
pub struct ScrollGestureEndedEventTrigger {
    base: InteractiveTriggerBase,
    // The handlers added to the associated object (the reference removes its delegates).
    on_scroll_gesture_ended_handler: Cell<Option<RoutedEventHandlerToken>>,
}

ferro_class!(ScrollGestureEndedEventTrigger: InteractiveTriggerBase);
ferroui_base::ferro_class_info!(ScrollGestureEndedEventTrigger { new: ScrollGestureEndedEventTrigger::new });
ferroui_base::ferro_impl_classes!(ScrollGestureEndedEventTrigger: FerroObjectImpl, StyledElementImpl);

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ScrollGestureEndedEventTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let trigger = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::scroll_gesture_ended_event(),
                move |_sender: &Interactive, e: &ScrollGestureEndedEventArgs| {
                    if let Some(trigger) = trigger.upgrade() {
                        trigger.on_scroll_gesture_ended(e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.on_scroll_gesture_ended_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.on_scroll_gesture_ended_handler.take() {
                associated_object.remove_handler(InputElement::scroll_gesture_ended_event(), handler);
            }
        }
    }
}

impl ScrollGestureEndedEventTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveTriggerBase::construct(), on_scroll_gesture_ended_handler: Cell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn on_scroll_gesture_ended(&self, e: &dyn IRoutedEventArgs) {
        // The args are the parameter of the actions (a shared handle to the live args).
        let parameter: BoxedValue = Rc::new(e.share());
        self.execute(Some(&parameter));
    }
}
