//! Port of `InputElement/Triggers/KeyTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBaseImpl, RoutedEventTriggerBaseOfT, RoutedEventTriggerBaseOfTImpl};
use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyGesture};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEvent};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty,
};
use ferroui_xaml_interactivity::{DisposingTriggerImpl, StyledElementBehaviorImpl};

/// Specifies which keyboard event will fire the trigger.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference nests this enumeration in [`KeyTrigger`]
/// (`KeyTrigger.FiredOn`); Rust has no nested types, so it is an enumeration of the module of the
/// trigger.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FiredOn {
    /// Trigger on the [`InputElement::key_down_event`].
    #[default]
    KeyDown,

    /// Trigger on the [`InputElement::key_up_event`].
    KeyUp,
}

/// A trigger that listens for key events and executes its actions when the specified key or
/// gesture is detected.
#[repr(C)]
pub struct KeyTrigger {
    base: RoutedEventTriggerBaseOfT,
}

ferro_class!(KeyTrigger: RoutedEventTriggerBaseOfT);
ferro_class_info!(KeyTrigger { new: KeyTrigger::new });
ferroui_base::ferro_impl_classes!(
    KeyTrigger: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
    AttachedToVisualTreeTriggerBaseImpl,
);

ferro_properties! {
    impl KeyTrigger {
        /// Identifies the [`key`](KeyTrigger::key) property.
        pub fn key_property() -> StyledProperty<Option<Key>> {
            FerroProperty::register::<KeyTrigger, _>("Key", None)
        }

        /// Identifies the [`gesture`](KeyTrigger::gesture) property.
        pub fn gesture_property() -> StyledProperty<Option<KeyGesture>> {
            FerroProperty::register::<KeyTrigger, _>("Gesture", None)
        }

        /// Identifies the [`event`](KeyTrigger::event) property.
        pub fn event_property() -> StyledProperty<FiredOn> {
            FerroProperty::register::<KeyTrigger, _>("Event", FiredOn::KeyDown)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl RoutedEventTriggerBaseOfTImpl for KeyTrigger {
        fn routed_event(this: &Self) -> RoutedEvent {
            if this.event() == FiredOn::KeyDown {
                InputElement::key_down_event().as_routed_event()
            } else {
                InputElement::key_up_event().as_routed_event()
            }
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

impl KeyTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: RoutedEventTriggerBaseOfT::construct() }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the key to listen for. This is a property of the object model.
    pub fn key(&self) -> Option<Key> {
        self.get_value(Self::key_property())
    }

    pub fn set_key(&self, value: Option<Key>) {
        self.set_value(Self::key_property(), value)
    }

    /// Gets or sets the key gesture to match. This is a property of the object model.
    pub fn gesture(&self) -> Option<KeyGesture> {
        self.get_value(Self::gesture_property())
    }

    pub fn set_gesture(&self, value: Option<KeyGesture>) {
        self.set_value(Self::gesture_property(), value)
    }

    /// Gets or sets which key event fires the trigger. This is a property of the object model.
    pub fn event(&self) -> FiredOn {
        self.get_value(Self::event_property())
    }

    pub fn set_event(&self, value: FiredOn) {
        self.set_value(Self::event_property(), value)
    }
}
