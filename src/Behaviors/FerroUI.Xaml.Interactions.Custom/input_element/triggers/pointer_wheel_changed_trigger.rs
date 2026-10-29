//! Port of `InputElement/Triggers/PointerWheelChangedTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBaseOfT, RoutedEventTriggerBaseOfTImpl, RoutedEventTriggerBase};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// A trigger of [`InputElement::pointer_wheel_changed_event`] (a turn of the mouse wheel): while the associated visual is in the
/// visual tree, the event runs the actions with its args (`PointerWheelEventArgs`) as the parameter.
#[repr(C)]
pub struct PointerWheelChangedTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(PointerWheelChangedTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(PointerWheelChangedTrigger { new: PointerWheelChangedTrigger::new });
ferroui_base::ferro_impl_classes!(
    PointerWheelChangedTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for PointerWheelChangedTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::pointer_wheel_changed_event().as_routed_event()
        }
    }
}

impl PointerWheelChangedTrigger {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<PointerWheelChangedTrigger>(RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE);
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
