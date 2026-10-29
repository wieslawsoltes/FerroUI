//! Port of `TextInputEventBehavior.cs`.

use ferroui_base::input::{InputElement, TextInputEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken, RoutingStrategies};
use ferroui_base::{ferro_class, FerroObjectImpl, StyledElementImpl};
use ferroui_xaml_interactivity::{InteractiveBehaviorBase, StyledElementBehaviorImpl};
use std::cell::Cell;

/// Behavior that listens for the [`InputElement::text_input_event`].
///
/// The class is abstract: a deriving class overrides [`TextInputEventBehaviorImpl::on_text_input`].
#[repr(C)]
pub struct TextInputEventBehavior {
    base: InteractiveBehaviorBase,
    // The handler added to the associated object (the reference removes its delegate).
    text_input_handler: Cell<Option<RoutedEventHandlerToken>>,
}

// Deviation (DEVIATIONS.md, Behaviors): the `object? sender` of the hooks is the element the handler
// is attached to (`&Interactive`), as the routed event handlers of the framework receive it.
ferro_class! {
    TextInputEventBehavior: InteractiveBehaviorBase, virtuals TextInputEventBehaviorImpl: StyledElementBehaviorImpl {
        /// Called when text input is received by the associated control.
        fn on_text_input(this, sender: &Interactive, e: &TextInputEventArgs);
    }
}

ferroui_base::ferro_impl_classes!(TextInputEventBehavior: FerroObjectImpl, StyledElementImpl);

impl TextInputEventBehaviorImpl for TextInputEventBehavior {
    fn on_text_input(_this: &Self, _sender: &Interactive, _e: &TextInputEventArgs) {}
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for TextInputEventBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = associated_object.add_handler_with(
                InputElement::text_input_event(),
                move |sender: &Interactive, e: &TextInputEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.text_input(sender, e);
                    }
                },
                this.routing_strategies(),
                false,
            );
            this.text_input_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Interactive>() else {
                return;
            };

            if let Some(handler) = this.text_input_handler.take() {
                associated_object.remove_handler(InputElement::text_input_event(), handler);
            }
        }
    }
}

impl TextInputEventBehavior {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        InteractiveBehaviorBase::routing_strategies_property().override_default_value::<TextInputEventBehavior>(
            RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
        );
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveBehaviorBase::construct(), text_input_handler: Cell::new(None) }
    }

    fn text_input(&self, sender: &Interactive, e: &TextInputEventArgs) {
        self.on_text_input(sender, e);
    }
}
