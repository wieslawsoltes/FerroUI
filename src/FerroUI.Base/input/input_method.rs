use super::text_input::TextInputMethodClientRequeryRequestedEventArgs;
use super::InputElement;
use crate::interactivity::{Interactive, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies};
use crate::{ferro_property, ferro_routed_event, AttachedProperty, FerroProperty};

/// Defines the attached property and the routed event that connect an
/// input element to the input method of the platform.
pub struct InputMethod;

crate::ferro_static_type!(InputMethod);

crate::ferro_properties! { impl InputMethod {
    ferro_property!(
        /// A dependency property that enables alternative text inputs.
        pub fn is_input_method_enabled_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<InputMethod, InputElement, _>("IsInputMethodEnabled", true)
        }
    );
} }

impl InputMethod {
    /// Setter for the `IsInputMethodEnabled` attached property.
    pub fn set_is_input_method_enabled(target: &InputElement, value: bool) {
        target.set_value(Self::is_input_method_enabled_property(), value)
    }

    /// Getter for the `IsInputMethodEnabled` attached property.
    pub fn get_is_input_method_enabled(target: &InputElement) -> bool {
        target.get_value(Self::is_input_method_enabled_property())
    }

    ferro_routed_event!(
        /// Defines the `TextInputMethodClientRequeryRequested` event: raise
        /// it on an element to have the input system query the focused
        /// element for its text input method client again.
        pub fn text_input_method_client_requery_requested_event(
        ) -> RoutedEvent<TextInputMethodClientRequeryRequestedEventArgs> {
            RoutedEvent::register::<InputElement, _>(
                "TextInputMethodClientRequeryRequested",
                RoutingStrategies::BUBBLE,
            )
        }
    );

    /// Adds a handler for the `TextInputMethodClientRequeryRequested`
    /// attached event.
    pub fn add_text_input_method_client_requery_requested_handler(
        element: &Interactive,
        handler: impl Fn(&Interactive, &RoutedEventArgs) + 'static,
    ) -> RoutedEventHandlerToken {
        element.add_handler_as::<RoutedEventArgs, _>(
            Self::text_input_method_client_requery_requested_event(),
            handler,
            RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
            false,
        )
    }

    /// Removes a handler for the `TextInputMethodClientRequeryRequested`
    /// attached event.
    pub fn remove_text_input_method_client_requery_requested_handler(
        element: &Interactive,
        handler: RoutedEventHandlerToken,
    ) {
        element.remove_handler(Self::text_input_method_client_requery_requested_event(), handler);
    }
}
