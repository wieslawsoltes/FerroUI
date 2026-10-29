//! Port of `PointerEventsBehavior.cs`.

use ferroui_base::input::{InputElement, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that listens for multiple pointer events.
///
/// The class is abstract: a deriving class overrides the members of [`PointerEventsBehaviorImpl`].
#[repr(C)]
pub struct PointerEventsBehavior {
    base: InteractiveBehaviorBase,
    // The handlers added to the associated object (the reference removes its delegates).
    pointer_pressed_handler: Cell<Option<RoutedEventHandlerToken>>,
    pointer_released_handler: Cell<Option<RoutedEventHandlerToken>>,
    pointer_moved_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    PointerEventsBehavior: InteractiveBehaviorBase, virtuals PointerEventsBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when a pointer is pressed over the associated control.
        fn on_pointer_pressed(this, sender: &Interactive, e: &PointerPressedEventArgs);
        /// Called when a pointer is released over the associated control.
        fn on_pointer_released(this, sender: &Interactive, e: &PointerReleasedEventArgs);
        /// Called when a pointer moves over the associated control.
        fn on_pointer_moved(this, sender: &Interactive, e: &PointerEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(PointerEventsBehavior: FerroObjectImpl, StyledElementImpl);

impl PointerEventsBehaviorImpl for PointerEventsBehavior {
    fn on_pointer_pressed(_this: &Self, _sender: &Interactive, _e: &PointerPressedEventArgs) {}

    fn on_pointer_released(_this: &Self, _sender: &Interactive, _e: &PointerReleasedEventArgs) {}

    fn on_pointer_moved(_this: &Self, _sender: &Interactive, _e: &PointerEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for PointerEventsBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::pointer_pressed_event(),
                move |sender: &Interactive, e: &PointerPressedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.pointer_pressed(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.pointer_pressed_handler.set(Some(handler));

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::pointer_released_event(),
                move |sender: &Interactive, e: &PointerReleasedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.pointer_released(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.pointer_released_handler.set(Some(handler));

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::pointer_moved_event(),
                move |sender: &Interactive, e: &PointerEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.pointer_moved(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.pointer_moved_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.pointer_pressed_handler.take() {
                associated_object.remove_handler(InputElement::pointer_pressed_event(), handler);
            }
            if let Some(handler) = this.pointer_released_handler.take() {
                associated_object.remove_handler(InputElement::pointer_released_event(), handler);
            }
            if let Some(handler) = this.pointer_moved_handler.take() {
                associated_object.remove_handler(InputElement::pointer_moved_event(), handler);
            }
        }
    }
}

impl PointerEventsBehavior {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        InteractiveBehaviorBase::routing_strategies_property().override_default_value::<PointerEventsBehavior>(
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: InteractiveBehaviorBase::construct(),
            pointer_pressed_handler: Cell::new(None),
            pointer_released_handler: Cell::new(None),
            pointer_moved_handler: Cell::new(None),
        }
    }

    fn pointer_pressed(&self, sender: &Interactive, e: &PointerPressedEventArgs) {
        self.on_pointer_pressed(sender, e);
    }

    fn pointer_released(&self, sender: &Interactive, e: &PointerReleasedEventArgs) {
        self.on_pointer_released(sender, e);
    }

    fn pointer_moved(&self, sender: &Interactive, e: &PointerEventArgs) {
        self.on_pointer_moved(sender, e);
    }
}
