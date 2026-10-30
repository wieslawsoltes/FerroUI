//! Port of `PointerMovedEventBehavior.cs`.

use ferroui_base::input::{InputElement, PointerEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that handles the [`InputElement::pointer_moved_event`].
///
/// The class is abstract: a deriving class overrides [`PointerMovedEventBehaviorImpl::on_pointer_moved`].
#[repr(C)]
pub struct PointerMovedEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    pointer_moved_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    PointerMovedEventBehavior: InteractiveBehaviorBase, virtuals PointerMovedEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when a pointer moves over the associated control.
        fn on_pointer_moved(this, sender: &Interactive, e: &PointerEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(PointerMovedEventBehavior: FerroObjectImpl, StyledElementImpl);

impl PointerMovedEventBehaviorImpl for PointerMovedEventBehavior {
    fn on_pointer_moved(_this: &Self, _sender: &Interactive, _e: &PointerEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for PointerMovedEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

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

            if let Some(handler) = this.pointer_moved_handler.take() {
                associated_object.remove_handler(InputElement::pointer_moved_event(), handler);
            }
        }
    }
}

impl PointerMovedEventBehavior {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        InteractiveBehaviorBase::routing_strategies_property().override_default_value::<PointerMovedEventBehavior>(
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), pointer_moved_handler: Cell::new(None) }
    }

    fn pointer_moved(&self, sender: &Interactive, e: &PointerEventArgs) {
        self.on_pointer_moved(sender, e);
    }
}
