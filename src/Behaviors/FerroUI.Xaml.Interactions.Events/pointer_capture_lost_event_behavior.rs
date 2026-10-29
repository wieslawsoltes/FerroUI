//! Port of `PointerCaptureLostEventBehavior.cs`.

use ferroui_base::input::{InputElement, PointerCaptureLostEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that listens for the [`InputElement::pointer_capture_lost_event`].
///
/// The class is abstract: a deriving class overrides [`PointerCaptureLostEventBehaviorImpl::on_pointer_capture_lost`].
#[repr(C)]
pub struct PointerCaptureLostEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    pointer_capture_lost_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    PointerCaptureLostEventBehavior: InteractiveBehaviorBase, virtuals PointerCaptureLostEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when pointer capture is lost on the associated control.
        fn on_pointer_capture_lost(this, sender: &Interactive, e: &PointerCaptureLostEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(PointerCaptureLostEventBehavior: FerroObjectImpl, StyledElementImpl);

impl PointerCaptureLostEventBehaviorImpl for PointerCaptureLostEventBehavior {
    fn on_pointer_capture_lost(_this: &Self, _sender: &Interactive, _e: &PointerCaptureLostEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for PointerCaptureLostEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::pointer_capture_lost_event(),
                move |sender: &Interactive, e: &PointerCaptureLostEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.pointer_capture_lost(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.pointer_capture_lost_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.pointer_capture_lost_handler.take() {
                associated_object.remove_handler(InputElement::pointer_capture_lost_event(), handler);
            }
        }
    }
}

impl PointerCaptureLostEventBehavior {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        InteractiveBehaviorBase::routing_strategies_property().override_default_value::<PointerCaptureLostEventBehavior>(
            RoutingStrategies::DIRECT,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), pointer_capture_lost_handler: Cell::new(None) }
    }

    fn pointer_capture_lost(&self, sender: &Interactive, e: &PointerCaptureLostEventArgs) {
        self.on_pointer_capture_lost(sender, e);
    }
}
