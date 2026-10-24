//! Port of `PointerEnteredEventBehavior.cs`.

use ferroui_base::input::{InputElement, PointerEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that handles the [`InputElement::pointer_entered_event`].
///
/// The class is abstract: a deriving class overrides [`PointerEnteredEventBehaviorImpl::on_pointer_enter`].
#[repr(C)]
pub struct PointerEnteredEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    pointer_entered_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    PointerEnteredEventBehavior: InteractiveBehaviorBase, virtuals PointerEnteredEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when a pointer enters the associated control.
        fn on_pointer_enter(this, sender: &Interactive, e: &PointerEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(PointerEnteredEventBehavior: FerroObjectImpl, StyledElementImpl);

impl PointerEnteredEventBehaviorImpl for PointerEnteredEventBehavior {
    fn on_pointer_enter(_this: &Self, _sender: &Interactive, _e: &PointerEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for PointerEnteredEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::pointer_entered_event(),
                move |sender: &Interactive, e: &PointerEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.pointer_entered(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.pointer_entered_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.pointer_entered_handler.take() {
                associated_object.remove_handler(InputElement::pointer_entered_event(), handler);
            }
        }
    }
}

impl PointerEnteredEventBehavior {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        InteractiveBehaviorBase::routing_strategies_property().override_default_value::<PointerEnteredEventBehavior>(
            RoutingStrategies::DIRECT,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), pointer_entered_handler: Cell::new(None) }
    }

    fn pointer_entered(&self, sender: &Interactive, e: &PointerEventArgs) {
        self.on_pointer_enter(sender, e);
    }
}
