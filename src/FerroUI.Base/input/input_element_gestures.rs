//! The gesture-related part of [`InputElement`]: gesture events and the
//! attached properties that configure gesture recognition.

use super::input_element::routed_event_accessor;
use super::{
    ContextRequestedEventArgs, HoldingRoutedEventArgs, HoldingState, InputElement, PinchEndedEventArgs,
    PinchEventArgs, PointerDeltaEventArgs, PullGestureEndedEventArgs, PullGestureEventArgs,
    ScrollGestureEndedEventArgs, ScrollGestureEventArgs, ScrollGestureInertiaStartingEventArgs,
    SwipeGestureEndedEventArgs, SwipeGestureEventArgs, TappedEventArgs,
};
use crate::interactivity::{
    Interactive, RoutedEvent, RoutedEventArgs, RoutedEventHandlerToken, RoutingStrategies,
};
use crate::{ferro_property, ferro_routed_event, AttachedProperty, FerroProperty, StyledElement};

crate::ferro_properties! { impl InputElement, fn register_gesture_properties {
    ferro_property!(
        /// Defines the `IsHoldingEnabled` attached property.
        pub fn is_holding_enabled_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<InputElement, StyledElement, _>("IsHoldingEnabled", true)
        }
    );

    ferro_property!(
        /// Defines the `IsHoldWithMouseEnabled` attached property.
        pub fn is_hold_with_mouse_enabled_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<InputElement, StyledElement, _>("IsHoldWithMouseEnabled", false)
        }
    );
} }

impl InputElement {
    ferro_routed_event!(
        /// Defines the `Pinch` event.
        pub fn pinch_event() -> RoutedEvent<PinchEventArgs> {
            RoutedEvent::register::<InputElement, _>("Pinch", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `PinchEnded` event.
        pub fn pinch_ended_event() -> RoutedEvent<PinchEndedEventArgs> {
            RoutedEvent::register::<InputElement, _>("PinchEnded", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `PullGesture` event.
        pub fn pull_gesture_event() -> RoutedEvent<PullGestureEventArgs> {
            RoutedEvent::register::<InputElement, _>("PullGesture", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `PullGestureEnded` event.
        pub fn pull_gesture_ended_event() -> RoutedEvent<PullGestureEndedEventArgs> {
            RoutedEvent::register::<InputElement, _>("PullGestureEnded", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `SwipeGesture` event.
        pub fn swipe_gesture_event() -> RoutedEvent<SwipeGestureEventArgs> {
            RoutedEvent::register::<InputElement, _>("SwipeGesture", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `SwipeGestureEnded` event.
        pub fn swipe_gesture_ended_event() -> RoutedEvent<SwipeGestureEndedEventArgs> {
            RoutedEvent::register::<InputElement, _>("SwipeGestureEnded", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `ScrollGesture` event.
        pub fn scroll_gesture_event() -> RoutedEvent<ScrollGestureEventArgs> {
            RoutedEvent::register::<InputElement, _>("ScrollGesture", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `ScrollGestureInertiaStarting` event.
        pub fn scroll_gesture_inertia_starting_event() -> RoutedEvent<ScrollGestureInertiaStartingEventArgs> {
            RoutedEvent::register::<InputElement, _>("ScrollGestureInertiaStarting", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `ScrollGestureEnded` event.
        pub fn scroll_gesture_ended_event() -> RoutedEvent<ScrollGestureEndedEventArgs> {
            RoutedEvent::register::<InputElement, _>("ScrollGestureEnded", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerTouchPadGestureMagnify` event.
        pub fn pointer_touch_pad_gesture_magnify_event() -> RoutedEvent<PointerDeltaEventArgs> {
            RoutedEvent::register::<InputElement, _>("PointerTouchPadGestureMagnify", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerTouchPadGestureRotate` event.
        pub fn pointer_touch_pad_gesture_rotate_event() -> RoutedEvent<PointerDeltaEventArgs> {
            RoutedEvent::register::<InputElement, _>("PointerTouchPadGestureRotate", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `PointerTouchPadGestureSwipe` event.
        pub fn pointer_touch_pad_gesture_swipe_event() -> RoutedEvent<PointerDeltaEventArgs> {
            RoutedEvent::register::<InputElement, _>("PointerTouchPadGestureSwipe", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `Tapped` event.
        pub fn tapped_event() -> RoutedEvent<TappedEventArgs> {
            RoutedEvent::register::<InputElement, _>("Tapped", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `RightTapped` event.
        pub fn right_tapped_event() -> RoutedEvent<TappedEventArgs> {
            RoutedEvent::register::<InputElement, _>("RightTapped", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `Holding` event.
        pub fn holding_event() -> RoutedEvent<HoldingRoutedEventArgs> {
            RoutedEvent::register::<InputElement, _>("Holding", RoutingStrategies::BUBBLE)
        }
    );

    ferro_routed_event!(
        /// Defines the `DoubleTapped` event.
        pub fn double_tapped_event() -> RoutedEvent<TappedEventArgs> {
            RoutedEvent::register::<InputElement, _>("DoubleTapped", RoutingStrategies::BUBBLE)
        }
    );

    /// Registers the gesture events that have no class handler.
    pub(super) fn register_gesture_events() {
        Self::pinch_event();
        Self::pinch_ended_event();
        Self::pull_gesture_event();
        Self::pull_gesture_ended_event();
        Self::swipe_gesture_event();
        Self::swipe_gesture_ended_event();
        Self::scroll_gesture_event();
        Self::scroll_gesture_inertia_starting_event();
        Self::scroll_gesture_ended_event();
        Self::pointer_touch_pad_gesture_magnify_event();
        Self::pointer_touch_pad_gesture_rotate_event();
        Self::pointer_touch_pad_gesture_swipe_event();
    }

    /// Gets whether the holding gesture is enabled on an element.
    pub fn get_is_holding_enabled(element: &StyledElement) -> bool {
        element.get_value(Self::is_holding_enabled_property())
    }

    /// Sets whether the holding gesture is enabled on an element.
    pub fn set_is_holding_enabled(element: &StyledElement, value: bool) {
        element.set_value(Self::is_holding_enabled_property(), value)
    }

    /// Gets whether the holding gesture can be performed with the mouse on
    /// an element.
    pub fn get_is_hold_with_mouse_enabled(element: &StyledElement) -> bool {
        element.get_value(Self::is_hold_with_mouse_enabled_property())
    }

    /// Sets whether the holding gesture can be performed with the mouse on
    /// an element.
    pub fn set_is_hold_with_mouse_enabled(element: &StyledElement, value: bool) {
        element.set_value(Self::is_hold_with_mouse_enabled_property(), value)
    }

    routed_event_accessor!(
        /// Occurs when the user moves two contact points closer together.
        pinch, pinch_event, PinchEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user releases both contact points used in a
        /// pinch gesture.
        pinch_ended, pinch_ended_event, PinchEndedEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user drags from the edge of a control.
        pull_gesture, pull_gesture_event, PullGestureEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user releases the pointer after a pull gesture.
        pull_gesture_ended, pull_gesture_ended_event, PullGestureEndedEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user continuously moves the pointer in the same
        /// direction within the control's boundaries.
        scroll_gesture, scroll_gesture_event, ScrollGestureEventArgs
    );

    routed_event_accessor!(
        /// Occurs within the control's boundaries when a scroll gesture's
        /// inertia begins.
        scroll_gesture_inertia_starting, scroll_gesture_inertia_starting_event, ScrollGestureInertiaStartingEventArgs
    );

    routed_event_accessor!(
        /// Occurs when a scroll gesture ends.
        scroll_gesture_ended, scroll_gesture_ended_event, ScrollGestureEndedEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user moves two contact points away from each
        /// other on a touchpad.
        pointer_touch_pad_gesture_magnify, pointer_touch_pad_gesture_magnify_event, PointerDeltaEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user places two contact points and moves them in
        /// a circular motion on a touchpad.
        pointer_touch_pad_gesture_rotate, pointer_touch_pad_gesture_rotate_event, PointerDeltaEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user rapidly drags the pointer in a single
        /// direction across the control.
        swipe_gesture, swipe_gesture_event, SwipeGestureEventArgs
    );

    routed_event_accessor!(
        /// Occurs when a swipe gesture ends on the control.
        swipe_gesture_ended, swipe_gesture_ended_event, SwipeGestureEndedEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user rapidly drags the pointer in a single
        /// direction on a touchpad.
        pointer_touch_pad_gesture_swipe, pointer_touch_pad_gesture_swipe_event, PointerDeltaEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user briefly contacts and releases a single
        /// point on the control.
        tapped, tapped_event, TappedEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user briefly contacts and releases a single
        /// point on the control using the secondary action.
        right_tapped, right_tapped_event, TappedEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user makes a single contact, then maintains
        /// contact beyond a given time threshold without releasing or
        /// making another contact.
        holding, holding_event, HoldingRoutedEventArgs
    );

    routed_event_accessor!(
        /// Occurs when the user briefly contacts and releases twice on a
        /// single point, without significant movement between contacts.
        double_tapped, double_tapped_event, TappedEventArgs
    );

    pub(super) fn on_preview_holding(sender: &Interactive, e: &HoldingRoutedEventArgs) {
        let Some(input_element) = sender.downcast_ref::<InputElement>() else { return };

        input_element.raise_event(e);

        if !e.handled() && e.holding_state() == HoldingState::Started {
            let context_event = ContextRequestedEventArgs::from_holding(e);
            input_element.raise_event(&context_event);
            e.set_handled(context_event.handled());

            if context_event.handled() {
                input_element.is_context_menu_on_holding.set(true);
            }
        } else if e.holding_state() == HoldingState::Canceled && input_element.is_context_menu_on_holding.get() {
            let args = RoutedEventArgs::with_event_and_source(
                InputElement::context_canceled_event(),
                input_element.to_ref(),
            );
            input_element.raise_event(&args);
            input_element.is_context_menu_on_holding.set(false);
        } else if e.holding_state() == HoldingState::Completed {
            input_element.is_context_menu_on_holding.set(false);
        }
    }
}
