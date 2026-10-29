//! Port of `Gestures/PointerTouchPadGestureMagnifyGestureTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBaseOfT, RoutedEventTriggerBaseOfTImpl, RoutedEventTriggerBase};
use ferroui_base::input::InputElement;
use ferroui_base::interactivity::{RoutedEvent, RoutingStrategies};
use ferroui_base::{ferro_class, ferro_class_info, instantiate, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// A trigger of [`InputElement::pointer_touch_pad_gesture_magnify_event`] (a magnify gesture of a touch pad): while the associated visual is in the
/// visual tree, the event runs the actions with its args (`PointerDeltaEventArgs`) as the parameter.
#[repr(C)]
pub struct PointerTouchPadGestureMagnifyGestureTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(PointerTouchPadGestureMagnifyGestureTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(PointerTouchPadGestureMagnifyGestureTrigger { new: PointerTouchPadGestureMagnifyGestureTrigger::new });
ferroui_base::ferro_impl_classes!(
    PointerTouchPadGestureMagnifyGestureTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for PointerTouchPadGestureMagnifyGestureTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::pointer_touch_pad_gesture_magnify_event().as_routed_event()
        }
    }
}

impl PointerTouchPadGestureMagnifyGestureTrigger {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<PointerTouchPadGestureMagnifyGestureTrigger>(RoutingStrategies::BUBBLE);
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
