//! Port of `ScrollGestureEventBehavior.cs`.

use ferroui_base::input::{InputElement, ScrollGestureEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that handles the [`InputElement::scroll_gesture_event`].
///
/// The class is abstract: a deriving class overrides [`ScrollGestureEventBehaviorImpl::on_scroll_gesture`].
#[repr(C)]
pub struct ScrollGestureEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    scroll_gesture_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    ScrollGestureEventBehavior: InteractiveBehaviorBase, virtuals ScrollGestureEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when a scroll gesture occurs on the associated control.
        fn on_scroll_gesture(this, sender: &Interactive, e: &ScrollGestureEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(ScrollGestureEventBehavior: FerroObjectImpl, StyledElementImpl);

impl ScrollGestureEventBehaviorImpl for ScrollGestureEventBehavior {
    fn on_scroll_gesture(_this: &Self, _sender: &Interactive, _e: &ScrollGestureEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ScrollGestureEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::scroll_gesture_event(),
                move |sender: &Interactive, e: &ScrollGestureEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.scroll_gesture(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.scroll_gesture_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.scroll_gesture_handler.take() {
                associated_object.remove_handler(InputElement::scroll_gesture_event(), handler);
            }
        }
    }
}

impl ScrollGestureEventBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), scroll_gesture_handler: Cell::new(None) }
    }

    fn scroll_gesture(&self, sender: &Interactive, e: &ScrollGestureEventArgs) {
        self.on_scroll_gesture(sender, e);
    }
}
