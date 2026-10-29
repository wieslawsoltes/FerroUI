//! Port of `PointerReleasedEventBehavior.cs`.

use ferroui_base::input::{InputElement, PointerReleasedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that handles the [`InputElement::pointer_released_event`].
///
/// The class is abstract: a deriving class overrides [`PointerReleasedEventBehaviorImpl::on_pointer_released`].
#[repr(C)]
pub struct PointerReleasedEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    pointer_released_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    PointerReleasedEventBehavior: InteractiveBehaviorBase, virtuals PointerReleasedEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when a pointer is released over the associated control.
        fn on_pointer_released(this, sender: &Interactive, e: &PointerReleasedEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(PointerReleasedEventBehavior: FerroObjectImpl, StyledElementImpl);

impl PointerReleasedEventBehaviorImpl for PointerReleasedEventBehavior {
    fn on_pointer_released(_this: &Self, _sender: &Interactive, _e: &PointerReleasedEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for PointerReleasedEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

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
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.pointer_released_handler.take() {
                associated_object.remove_handler(InputElement::pointer_released_event(), handler);
            }
        }
    }
}

impl PointerReleasedEventBehavior {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        InteractiveBehaviorBase::routing_strategies_property().override_default_value::<PointerReleasedEventBehavior>(
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), pointer_released_handler: Cell::new(None) }
    }

    fn pointer_released(&self, sender: &Interactive, e: &PointerReleasedEventArgs) {
        self.on_pointer_released(sender, e);
    }
}
