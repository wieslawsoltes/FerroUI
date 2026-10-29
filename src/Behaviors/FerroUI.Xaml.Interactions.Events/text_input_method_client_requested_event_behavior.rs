//! Port of `TextInputMethodClientRequestedEventBehavior.cs`.

use ferroui_base::input::InputElement;
use ferroui_base::input::text_input::TextInputMethodClientRequestedEventArgs;
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that handles the [`InputElement::text_input_method_client_requested_event`].
///
/// The class is abstract: a deriving class overrides [`TextInputMethodClientRequestedEventBehaviorImpl::on_text_input_method_client_requested`].
#[repr(C)]
pub struct TextInputMethodClientRequestedEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    text_input_method_client_requested_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    TextInputMethodClientRequestedEventBehavior: InteractiveBehaviorBase, virtuals TextInputMethodClientRequestedEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when a text input method client is requested for the associated control.
        fn on_text_input_method_client_requested(this, sender: &Interactive, e: &TextInputMethodClientRequestedEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(TextInputMethodClientRequestedEventBehavior: FerroObjectImpl, StyledElementImpl);

impl TextInputMethodClientRequestedEventBehaviorImpl for TextInputMethodClientRequestedEventBehavior {
    fn on_text_input_method_client_requested(_this: &Self, _sender: &Interactive, _e: &TextInputMethodClientRequestedEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for TextInputMethodClientRequestedEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::text_input_method_client_requested_event(),
                move |sender: &Interactive, e: &TextInputMethodClientRequestedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.text_input_method_client_requested(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.text_input_method_client_requested_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.text_input_method_client_requested_handler.take() {
                associated_object.remove_handler(InputElement::text_input_method_client_requested_event(), handler);
            }
        }
    }
}

impl TextInputMethodClientRequestedEventBehavior {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        InteractiveBehaviorBase::routing_strategies_property().override_default_value::<TextInputMethodClientRequestedEventBehavior>(
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), text_input_method_client_requested_handler: Cell::new(None) }
    }

    fn text_input_method_client_requested(&self, sender: &Interactive, e: &TextInputMethodClientRequestedEventArgs) {
        self.on_text_input_method_client_requested(sender, e);
    }
}
