//! Port of `Gestures/PullGestureGestureTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBaseOfT, RoutedEventTriggerBaseOfTImpl, RoutedEventTriggerBase};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// A trigger of [`InputElement::pull_gesture_event`] (a pull gesture): while the associated visual is in the
/// visual tree, the event runs the actions with its args (`PullGestureEventArgs`) as the parameter.
#[repr(C)]
pub struct PullGestureGestureTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(PullGestureGestureTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(PullGestureGestureTrigger { new: PullGestureGestureTrigger::new });
ferroui_base::ferro_impl_classes!(
    PullGestureGestureTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for PullGestureGestureTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::pull_gesture_event().as_routed_event()
        }
    }
}

impl PullGestureGestureTrigger {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<PullGestureGestureTrigger>(RoutingStrategies::BUBBLE);
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
