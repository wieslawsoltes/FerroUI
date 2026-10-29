//! Port of `InputElement/Triggers/HoldingTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBaseOfT, RoutedEventTriggerBaseOfTImpl, RoutedEventTriggerBase};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// A trigger of [`InputElement::holding_event`] (a hold): while the associated visual is in the
/// visual tree, the event runs the actions with its args (`HoldingRoutedEventArgs`) as the parameter.
#[repr(C)]
pub struct HoldingTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(HoldingTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(HoldingTrigger { new: HoldingTrigger::new });
ferroui_base::ferro_impl_classes!(
    HoldingTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for HoldingTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::holding_event().as_routed_event()
        }
    }
}

impl HoldingTrigger {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<HoldingTrigger>(RoutingStrategies::BUBBLE);
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
