//! Port of `Gestures/ScrollGestureInertiaStartingGestureTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBaseOfT, RoutedEventTriggerBaseOfTImpl, RoutedEventTriggerBase};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// A trigger of [`InputElement::scroll_gesture_inertia_starting_event`] (the start of the inertia of a scroll gesture): while the associated visual is in the
/// visual tree, the event runs the actions with its args (`ScrollGestureInertiaStartingEventArgs`) as the parameter.
#[repr(C)]
pub struct ScrollGestureInertiaStartingGestureTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(ScrollGestureInertiaStartingGestureTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(ScrollGestureInertiaStartingGestureTrigger { new: ScrollGestureInertiaStartingGestureTrigger::new });
ferroui_base::ferro_impl_classes!(
    ScrollGestureInertiaStartingGestureTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for ScrollGestureInertiaStartingGestureTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::scroll_gesture_inertia_starting_event().as_routed_event()
        }
    }
}

impl ScrollGestureInertiaStartingGestureTrigger {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<ScrollGestureInertiaStartingGestureTrigger>(RoutingStrategies::BUBBLE);
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
