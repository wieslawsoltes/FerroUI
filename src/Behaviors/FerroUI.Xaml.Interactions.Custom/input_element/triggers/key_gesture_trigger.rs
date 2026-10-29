//! Port of `InputElement/Triggers/KeyGestureTrigger.cs`.

use crate::{
    AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBase, RoutedEventTriggerBaseOfT,
    RoutedEventTriggerBaseOfTImpl,
};
use ferroui_base::input::{InputElement, KeyEventArgs, KeyGesture};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEvent, RoutingStrategies};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// Trigger that listens for a key gesture.
#[repr(C)]
pub struct KeyGestureTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(KeyGestureTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(KeyGestureTrigger { new: KeyGestureTrigger::new });
ferroui_base::ferro_impl_classes!(
    KeyGestureTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferro_properties! {
    impl KeyGestureTrigger {
        /// Identifies the [`gesture`](KeyGestureTrigger::gesture) property.
        pub fn gesture_property() -> StyledProperty<Option<KeyGesture>> {
            FerroProperty::register::<KeyGestureTrigger, _>("Gesture", None)
        }

        /// Identifies the [`fired_on`](KeyGestureTrigger::fired_on) property.
        pub fn fired_on_property() -> StyledProperty<KeyGestureTriggerFiredOn> {
            FerroProperty::register::<KeyGestureTrigger, _>("FiredOn", KeyGestureTriggerFiredOn::KeyDown)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for KeyGestureTrigger {
        fn routed_event(this: &Self) -> RoutedEvent {
            if this.fired_on() == KeyGestureTriggerFiredOn::KeyUp {
                InputElement::key_up_event().as_routed_event()
            } else {
                InputElement::key_down_event().as_routed_event()
            }
        }

        fn handler(this: &Self, _sender: &Interactive, e: &dyn IRoutedEventArgs) {
            if !this.is_enabled() {
                return;
            }

            let Some(key_event) = e.downcast_ref::<KeyEventArgs>() else {
                return;
            };

            let gesture = this.gesture();
            if gesture.is_none_or(|gesture| gesture.matches(Some(key_event))) {
                this.execute(e);
            }
        }
    }
}

impl KeyGestureTrigger {
    /// The static constructor: the default routing strategies of the class.
    fn static_constructor() {
        RoutedEventTriggerBase::event_routing_strategy_property()
            .override_default_value::<KeyGestureTrigger>(RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: RoutedEventTriggerBaseOfT::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the gesture that will fire the trigger.
    pub fn gesture(&self) -> Option<KeyGesture> {
        self.get_value(Self::gesture_property())
    }

    pub fn set_gesture(&self, value: Option<KeyGesture>) {
        self.set_value(Self::gesture_property(), value)
    }

    /// Gets or sets whether the trigger reacts on key down or key up.
    pub fn fired_on(&self) -> KeyGestureTriggerFiredOn {
        self.get_value(Self::fired_on_property())
    }

    pub fn set_fired_on(&self, value: KeyGestureTriggerFiredOn) {
        self.set_value(Self::fired_on_property(), value)
    }
}

/// Specifies when a [`KeyGestureTrigger`] should be fired.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum KeyGestureTriggerFiredOn {
    /// Trigger on key down.
    #[default]
    KeyDown,
    /// Trigger on key up.
    KeyUp,
}
