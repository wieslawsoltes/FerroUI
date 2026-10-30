//! ferroui-xaml-interactions-events
//!
//! Behaviors and triggers of the input routed events of FerroUI, ported from the
//! `Xaml.Behaviors.Interactions.Events` project of Xaml.Behaviors (see `NOTICE.md`):
//!
//! - a trigger per input event ([`TappedEventTrigger`], [`KeyDownEventTrigger`],
//!   [`PointerPressedEventTrigger`], [`DropEventTrigger`], ..), which runs its actions with the
//!   args of the event as their parameter;
//! - an abstract behavior per input event ([`TappedEventBehavior`], [`KeyDownEventBehavior`],
//!   ..), whose deriving classes override the hook of the event (`on_tapped`, `on_key_down`, ..);
//! - [`PointerEventsTrigger`] and [`PointerEventsBehavior`], which listen for the pressed,
//!   released and moved pointer events together.
//!
//! The behaviors and triggers add their handler to the associated object when it is attached to
//! the visual tree and remove it when it is detached, with the routing strategies of their
//! `RoutingStrategies` property (by default those of the event they listen for).
//!
//! The namespace of the crate is `FerroUI.Xaml.Interactions.Events`, mapped into the default XML
//! namespace of the framework: documents name its types without a prefix once
//! [`register_types`] ran.

mod double_tapped_event_behavior;
mod double_tapped_event_trigger;
mod drag_enter_event_trigger;
mod drag_leave_event_trigger;
mod drag_over_event_trigger;
mod drop_event_trigger;
mod got_focus_event_behavior;
mod got_focus_event_trigger;
mod key_down_event_behavior;
mod key_down_event_trigger;
mod key_up_event_behavior;
mod key_up_event_trigger;
mod lost_focus_event_behavior;
mod lost_focus_event_trigger;
mod pointer_capture_lost_event_behavior;
mod pointer_capture_lost_event_trigger;
mod pointer_entered_event_behavior;
mod pointer_entered_event_trigger;
mod pointer_events_behavior;
mod pointer_events_trigger;
mod pointer_exited_event_behavior;
mod pointer_exited_event_trigger;
mod pointer_moved_event_behavior;
mod pointer_moved_event_trigger;
mod pointer_pressed_event_behavior;
mod pointer_pressed_event_trigger;
mod pointer_released_event_behavior;
mod pointer_released_event_trigger;
mod pointer_wheel_changed_event_behavior;
mod pointer_wheel_changed_event_trigger;
mod right_tapped_event_behavior;
mod right_tapped_event_trigger;
mod scroll_gesture_ended_event_behavior;
mod scroll_gesture_ended_event_trigger;
mod scroll_gesture_event_behavior;
mod scroll_gesture_event_trigger;
mod tapped_event_behavior;
mod tapped_event_trigger;
mod text_input_event_behavior;
mod text_input_event_trigger;
mod text_input_method_client_requested_event_behavior;
mod text_input_method_client_requested_event_trigger;
mod register_types;
mod rust_paths;

pub use crate::double_tapped_event_behavior::{DoubleTappedEventBehavior, DoubleTappedEventBehaviorImpl, DoubleTappedEventBehaviorImplExt, DoubleTappedEventBehaviorVTable};
pub use crate::double_tapped_event_trigger::DoubleTappedEventTrigger;
pub use crate::drag_enter_event_trigger::DragEnterEventTrigger;
pub use crate::drag_leave_event_trigger::DragLeaveEventTrigger;
pub use crate::drag_over_event_trigger::DragOverEventTrigger;
pub use crate::drop_event_trigger::DropEventTrigger;
pub use crate::got_focus_event_behavior::{GotFocusEventBehavior, GotFocusEventBehaviorImpl, GotFocusEventBehaviorImplExt, GotFocusEventBehaviorVTable};
pub use crate::got_focus_event_trigger::GotFocusEventTrigger;
pub use crate::key_down_event_behavior::{KeyDownEventBehavior, KeyDownEventBehaviorImpl, KeyDownEventBehaviorImplExt, KeyDownEventBehaviorVTable};
pub use crate::key_down_event_trigger::KeyDownEventTrigger;
pub use crate::key_up_event_behavior::{KeyUpEventBehavior, KeyUpEventBehaviorImpl, KeyUpEventBehaviorImplExt, KeyUpEventBehaviorVTable};
pub use crate::key_up_event_trigger::KeyUpEventTrigger;
pub use crate::lost_focus_event_behavior::{LostFocusEventBehavior, LostFocusEventBehaviorImpl, LostFocusEventBehaviorImplExt, LostFocusEventBehaviorVTable};
pub use crate::lost_focus_event_trigger::LostFocusEventTrigger;
pub use crate::pointer_capture_lost_event_behavior::{PointerCaptureLostEventBehavior, PointerCaptureLostEventBehaviorImpl, PointerCaptureLostEventBehaviorImplExt, PointerCaptureLostEventBehaviorVTable};
pub use crate::pointer_capture_lost_event_trigger::PointerCaptureLostEventTrigger;
pub use crate::pointer_entered_event_behavior::{PointerEnteredEventBehavior, PointerEnteredEventBehaviorImpl, PointerEnteredEventBehaviorImplExt, PointerEnteredEventBehaviorVTable};
pub use crate::pointer_entered_event_trigger::PointerEnteredEventTrigger;
pub use crate::pointer_events_behavior::{PointerEventsBehavior, PointerEventsBehaviorImpl, PointerEventsBehaviorImplExt, PointerEventsBehaviorVTable};
pub use crate::pointer_events_trigger::PointerEventsTrigger;
pub use crate::pointer_exited_event_behavior::{PointerExitedEventBehavior, PointerExitedEventBehaviorImpl, PointerExitedEventBehaviorImplExt, PointerExitedEventBehaviorVTable};
pub use crate::pointer_exited_event_trigger::PointerExitedEventTrigger;
pub use crate::pointer_moved_event_behavior::{PointerMovedEventBehavior, PointerMovedEventBehaviorImpl, PointerMovedEventBehaviorImplExt, PointerMovedEventBehaviorVTable};
pub use crate::pointer_moved_event_trigger::PointerMovedEventTrigger;
pub use crate::pointer_pressed_event_behavior::{PointerPressedEventBehavior, PointerPressedEventBehaviorImpl, PointerPressedEventBehaviorImplExt, PointerPressedEventBehaviorVTable};
pub use crate::pointer_pressed_event_trigger::PointerPressedEventTrigger;
pub use crate::pointer_released_event_behavior::{PointerReleasedEventBehavior, PointerReleasedEventBehaviorImpl, PointerReleasedEventBehaviorImplExt, PointerReleasedEventBehaviorVTable};
pub use crate::pointer_released_event_trigger::PointerReleasedEventTrigger;
pub use crate::pointer_wheel_changed_event_behavior::{PointerWheelChangedEventBehavior, PointerWheelChangedEventBehaviorImpl, PointerWheelChangedEventBehaviorImplExt, PointerWheelChangedEventBehaviorVTable};
pub use crate::pointer_wheel_changed_event_trigger::PointerWheelChangedEventTrigger;
pub use crate::right_tapped_event_behavior::{RightTappedEventBehavior, RightTappedEventBehaviorImpl, RightTappedEventBehaviorImplExt, RightTappedEventBehaviorVTable};
pub use crate::right_tapped_event_trigger::RightTappedEventTrigger;
pub use crate::scroll_gesture_ended_event_behavior::{ScrollGestureEndedEventBehavior, ScrollGestureEndedEventBehaviorImpl, ScrollGestureEndedEventBehaviorImplExt, ScrollGestureEndedEventBehaviorVTable};
pub use crate::scroll_gesture_ended_event_trigger::ScrollGestureEndedEventTrigger;
pub use crate::scroll_gesture_event_behavior::{ScrollGestureEventBehavior, ScrollGestureEventBehaviorImpl, ScrollGestureEventBehaviorImplExt, ScrollGestureEventBehaviorVTable};
pub use crate::scroll_gesture_event_trigger::ScrollGestureEventTrigger;
pub use crate::tapped_event_behavior::{TappedEventBehavior, TappedEventBehaviorImpl, TappedEventBehaviorImplExt, TappedEventBehaviorVTable};
pub use crate::tapped_event_trigger::TappedEventTrigger;
pub use crate::text_input_event_behavior::{TextInputEventBehavior, TextInputEventBehaviorImpl, TextInputEventBehaviorImplExt, TextInputEventBehaviorVTable};
pub use crate::text_input_event_trigger::TextInputEventTrigger;
pub use crate::text_input_method_client_requested_event_behavior::{TextInputMethodClientRequestedEventBehavior, TextInputMethodClientRequestedEventBehaviorImpl, TextInputMethodClientRequestedEventBehaviorImplExt, TextInputMethodClientRequestedEventBehaviorVTable};
pub use crate::text_input_method_client_requested_event_trigger::TextInputMethodClientRequestedEventTrigger;
pub use crate::register_types::{register_types, ASSEMBLY};

#[cfg(test)]
mod tests;
