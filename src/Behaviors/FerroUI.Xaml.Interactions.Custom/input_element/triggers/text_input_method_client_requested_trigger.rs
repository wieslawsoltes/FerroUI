//! Port of `InputElement/Triggers/TextInputMethodClientRequestedTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBaseOfT, RoutedEventTriggerBaseOfTImpl, RoutedEventTriggerBase};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// A trigger of [`InputElement::text_input_method_client_requested_event`] (the request of the client of an input method): while the associated visual is in the
/// visual tree, the event runs the actions with its args (`TextInputMethodClientRequestedEventArgs`) as the parameter.
#[repr(C)]
pub struct TextInputMethodClientRequestedTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(TextInputMethodClientRequestedTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(TextInputMethodClientRequestedTrigger { new: TextInputMethodClientRequestedTrigger::new });
ferroui_base::ferro_impl_classes!(
    TextInputMethodClientRequestedTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for TextInputMethodClientRequestedTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::text_input_method_client_requested_event().as_routed_event()
        }
    }
}

impl TextInputMethodClientRequestedTrigger {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<TextInputMethodClientRequestedTrigger>(RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: RoutedEventTriggerBaseOfT::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
