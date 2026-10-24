//! Port of `KeyUpEventBehavior.cs`.

use ferroui_base::input::{InputElement, KeyEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that listens for the [`InputElement::key_up_event`].
///
/// The class is abstract: a deriving class overrides [`KeyUpEventBehaviorImpl::on_key_up`].
#[repr(C)]
pub struct KeyUpEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    key_up_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    KeyUpEventBehavior: InteractiveBehaviorBase, virtuals KeyUpEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when a key is released while the associated control has focus.
        fn on_key_up(this, sender: &Interactive, e: &KeyEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(KeyUpEventBehavior: FerroObjectImpl, StyledElementImpl);

impl KeyUpEventBehaviorImpl for KeyUpEventBehavior {
    fn on_key_up(_this: &Self, _sender: &Interactive, _e: &KeyEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for KeyUpEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::key_up_event(),
                move |sender: &Interactive, e: &KeyEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.key_up(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.key_up_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.key_up_handler.take() {
                associated_object.remove_handler(InputElement::key_up_event(), handler);
            }
        }
    }
}

impl KeyUpEventBehavior {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        InteractiveBehaviorBase::routing_strategies_property().override_default_value::<KeyUpEventBehavior>(
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), key_up_handler: Cell::new(None) }
    }

    fn key_up(&self, sender: &Interactive, e: &KeyEventArgs) {
        self.on_key_up(sender, e);
    }
}
