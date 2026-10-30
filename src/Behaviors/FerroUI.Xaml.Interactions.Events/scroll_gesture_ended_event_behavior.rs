//! Port of `ScrollGestureEndedEventBehavior.cs`.

use ferroui_base::input::{InputElement, ScrollGestureEndedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that handles the [`InputElement::scroll_gesture_ended_event`].
///
/// The class is abstract: a deriving class overrides [`ScrollGestureEndedEventBehaviorImpl::on_scroll_gesture_ended`].
#[repr(C)]
pub struct ScrollGestureEndedEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    scroll_gesture_ended_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    ScrollGestureEndedEventBehavior: InteractiveBehaviorBase, virtuals ScrollGestureEndedEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when a scroll gesture ends on the associated control.
        fn on_scroll_gesture_ended(this, sender: &Interactive, e: &ScrollGestureEndedEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(ScrollGestureEndedEventBehavior: FerroObjectImpl, StyledElementImpl);

impl ScrollGestureEndedEventBehaviorImpl for ScrollGestureEndedEventBehavior {
    fn on_scroll_gesture_ended(_this: &Self, _sender: &Interactive, _e: &ScrollGestureEndedEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ScrollGestureEndedEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::scroll_gesture_ended_event(),
                move |sender: &Interactive, e: &ScrollGestureEndedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.scroll_gesture_ended(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.scroll_gesture_ended_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.scroll_gesture_ended_handler.take() {
                associated_object.remove_handler(InputElement::scroll_gesture_ended_event(), handler);
            }
        }
    }
}

impl ScrollGestureEndedEventBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), scroll_gesture_ended_handler: Cell::new(None) }
    }

    // Deviation (DEVIATIONS.md, Behaviors): the reference declares the handler and the hook
    // with the args of the scroll gesture (`ScrollGestureEventArgs`), which the event never
    // carries (its args are `ScrollGestureEndedEventArgs`, not derived from them); here they
    // take the args of the event.
    fn scroll_gesture_ended(&self, sender: &Interactive, e: &ScrollGestureEndedEventArgs) {
        self.on_scroll_gesture_ended(sender, e);
    }
}
