//! Port of `PointerExitedEventBehavior.cs`.

use ferroui_base::input::{InputElement, PointerEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that handles the [`InputElement::pointer_exited_event`].
///
/// The class is abstract: a deriving class overrides [`PointerExitedEventBehaviorImpl::on_pointer_leave`].
#[repr(C)]
pub struct PointerExitedEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    pointer_exited_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    PointerExitedEventBehavior: InteractiveBehaviorBase, virtuals PointerExitedEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when a pointer leaves the associated control.
        fn on_pointer_leave(this, sender: &Interactive, e: &PointerEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(PointerExitedEventBehavior: FerroObjectImpl, StyledElementImpl);

impl PointerExitedEventBehaviorImpl for PointerExitedEventBehavior {
    fn on_pointer_leave(_this: &Self, _sender: &Interactive, _e: &PointerEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for PointerExitedEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::pointer_exited_event(),
                move |sender: &Interactive, e: &PointerEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.pointer_exited(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.pointer_exited_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.pointer_exited_handler.take() {
                associated_object.remove_handler(InputElement::pointer_exited_event(), handler);
            }
        }
    }
}

impl PointerExitedEventBehavior {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        InteractiveBehaviorBase::routing_strategies_property().override_default_value::<PointerExitedEventBehavior>(
            RoutingStrategies::DIRECT,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), pointer_exited_handler: Cell::new(None) }
    }

    fn pointer_exited(&self, sender: &Interactive, e: &PointerEventArgs) {
        self.on_pointer_leave(sender, e);
    }
}
