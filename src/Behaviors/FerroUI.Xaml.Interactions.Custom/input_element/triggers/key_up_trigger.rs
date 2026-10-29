//! Port of `InputElement/Triggers/KeyUpTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBaseOfT, RoutedEventTriggerBaseOfTImpl};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyGesture};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEvent};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// A trigger of [`InputElement::key_up_event`]: runs the actions with the key args as the
/// parameter when the key or the key gesture is released (any key when neither is set).
#[repr(C)]
pub struct KeyUpTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(KeyUpTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(KeyUpTrigger { new: KeyUpTrigger::new });
ferroui_base::ferro_impl_classes!(
    KeyUpTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferro_properties! {
    impl KeyUpTrigger {
        /// Identifies the `Key` property.
        pub fn key_property() -> StyledProperty<Option<Key>> {
            FerroProperty::register::<KeyUpTrigger, _>("Key", None)
        }

        /// Identifies the `Gesture` property.
        pub fn gesture_property() -> StyledProperty<Option<KeyGesture>> {
            FerroProperty::register::<KeyUpTrigger, _>("Gesture", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for KeyUpTrigger {
        fn routed_event(_this: &Self) -> RoutedEvent {
            InputElement::key_up_event().as_routed_event()
        }

        fn handler(this: &Self, _sender: &Interactive, e: &dyn IRoutedEventArgs) {
            if !this.is_enabled() {
                return;
            }

            let Some(key_event) = e.downcast_ref::<KeyEventArgs>() else {
                return;
            };

            let is_key_set = this.is_set(Self::key_property());
            let is_gesture_set = this.is_set(Self::gesture_property());
            let key = this.key();
            let gesture = this.gesture();
            let have_key = is_key_set && key.is_some_and(|key| key_event.key == key);
            let have_gesture = is_gesture_set && gesture.is_some_and(|gesture| gesture.matches(Some(key_event)));

            if (!is_key_set && !is_gesture_set) || have_key || have_gesture {
                this.execute(e);
            }
        }
    }
}

impl KeyUpTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: RoutedEventTriggerBaseOfT::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the key that fires the trigger.
    pub fn key(&self) -> Option<Key> {
        self.get_value(Self::key_property())
    }

    pub fn set_key(&self, value: Option<Key>) {
        self.set_value(Self::key_property(), value)
    }

    /// Gets or sets the key gesture that fires the trigger.
    pub fn gesture(&self) -> Option<KeyGesture> {
        self.get_value(Self::gesture_property())
    }

    pub fn set_gesture(&self, value: Option<KeyGesture>) {
        self.set_value(Self::gesture_property(), value)
    }
}
