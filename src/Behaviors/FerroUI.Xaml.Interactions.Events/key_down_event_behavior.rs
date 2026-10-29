//! Port of `KeyDownEventBehavior.cs`.

use ferroui_base::input::{InputElement, KeyEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that listens for the [`InputElement::key_down_event`].
///
/// The class is abstract: a deriving class overrides [`KeyDownEventBehaviorImpl::on_key_down`].
#[repr(C)]
pub struct KeyDownEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    key_down_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    KeyDownEventBehavior: InteractiveBehaviorBase, virtuals KeyDownEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when a key is pressed while the associated control has focus.
        fn on_key_down(this, sender: &Interactive, e: &KeyEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(KeyDownEventBehavior: FerroObjectImpl, StyledElementImpl);

impl KeyDownEventBehaviorImpl for KeyDownEventBehavior {
    fn on_key_down(_this: &Self, _sender: &Interactive, _e: &KeyEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for KeyDownEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::key_down_event(),
                move |sender: &Interactive, e: &KeyEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.key_down(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.key_down_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.key_down_handler.take() {
                associated_object.remove_handler(InputElement::key_down_event(), handler);
            }
        }
    }
}

impl KeyDownEventBehavior {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        InteractiveBehaviorBase::routing_strategies_property().override_default_value::<KeyDownEventBehavior>(
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), key_down_handler: Cell::new(None) }
    }

    fn key_down(&self, sender: &Interactive, e: &KeyEventArgs) {
        self.on_key_down(sender, e);
    }
}
