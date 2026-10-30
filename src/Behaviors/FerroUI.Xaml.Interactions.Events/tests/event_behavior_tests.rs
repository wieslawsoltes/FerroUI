//! Not from the reference: the behaviors of the crate call their hook when their event is raised
//! on the associated object, with the associated object as the sender, until the associated
//! object leaves the visual tree.

use super::support::{object, start_app, Host};
use crate::*;
use ferroui_base::input::text_input::TextInputMethodClientRequestedEventArgs;
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, KeyEventArgs, PointerCaptureLostEventArgs, PointerEventArgs,
    PointerPressedEventArgs, PointerReleasedEventArgs, PointerWheelEventArgs, ScrollGestureEndedEventArgs,
    ScrollGestureEventArgs, TappedEventArgs, TextInputEventArgs,
};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutingStrategies};
use ferroui_base::{ferro_class, instantiate, FerroObject, FerroObjectImpl, Ref, StyledElementImpl};
use ferroui_xaml_interactivity::StyledElementBehaviorImpl;
use std::cell::{Cell, RefCell};

/// A behavior deriving from `$base` whose hook `$hook` counts its calls and records the sender.
macro_rules! test_behavior {
    ($name:ident : $base:ident, $impl_:ident, $hook:ident($args:ty)) => {
        #[repr(C)]
        struct $name {
            base: $base,
            count: Cell<i32>,
            sender: RefCell<Option<Ref<Interactive>>>,
        }

        ferro_class!($name: $base);
        ferroui_base::ferro_impl_classes!($name: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

        impl $impl_ for $name {
            fn $hook(this: &Self, sender: &Interactive, _e: $args) {
                this.count.set(this.count.get() + 1);
                *this.sender.borrow_mut() = Some(sender.to_ref());
            }
        }

        impl $name {
            fn new() -> Ref<Self> {
                instantiate(Self { base: $base::construct(), count: Cell::new(0), sender: RefCell::new(None) })
            }

            fn count(&self) -> i32 {
                self.count.get()
            }

            fn sender(&self) -> Option<Ref<Interactive>> {
                self.sender.borrow().clone()
            }
        }
    };
}

test_behavior!(TestDoubleTapped: DoubleTappedEventBehavior, DoubleTappedEventBehaviorImpl, on_double_tapped(&dyn IRoutedEventArgs));
test_behavior!(TestGotFocus: GotFocusEventBehavior, GotFocusEventBehaviorImpl, on_got_focus(&FocusChangedEventArgs));
test_behavior!(TestKeyDown: KeyDownEventBehavior, KeyDownEventBehaviorImpl, on_key_down(&KeyEventArgs));
test_behavior!(TestKeyUp: KeyUpEventBehavior, KeyUpEventBehaviorImpl, on_key_up(&KeyEventArgs));
test_behavior!(TestLostFocus: LostFocusEventBehavior, LostFocusEventBehaviorImpl, on_lost_focus(&FocusChangedEventArgs));
test_behavior!(TestPointerCaptureLost: PointerCaptureLostEventBehavior, PointerCaptureLostEventBehaviorImpl, on_pointer_capture_lost(&PointerCaptureLostEventArgs));
test_behavior!(TestPointerEntered: PointerEnteredEventBehavior, PointerEnteredEventBehaviorImpl, on_pointer_enter(&PointerEventArgs));
test_behavior!(TestPointerExited: PointerExitedEventBehavior, PointerExitedEventBehaviorImpl, on_pointer_leave(&PointerEventArgs));
test_behavior!(TestPointerMoved: PointerMovedEventBehavior, PointerMovedEventBehaviorImpl, on_pointer_moved(&PointerEventArgs));
test_behavior!(TestPointerPressed: PointerPressedEventBehavior, PointerPressedEventBehaviorImpl, on_pointer_pressed(&PointerPressedEventArgs));
test_behavior!(TestPointerReleased: PointerReleasedEventBehavior, PointerReleasedEventBehaviorImpl, on_pointer_released(&PointerReleasedEventArgs));
test_behavior!(TestPointerWheelChanged: PointerWheelChangedEventBehavior, PointerWheelChangedEventBehaviorImpl, on_pointer_wheel_changed(&PointerWheelEventArgs));
test_behavior!(TestRightTapped: RightTappedEventBehavior, RightTappedEventBehaviorImpl, on_right_tapped(&dyn IRoutedEventArgs));
test_behavior!(TestScrollGestureEnded: ScrollGestureEndedEventBehavior, ScrollGestureEndedEventBehaviorImpl, on_scroll_gesture_ended(&ScrollGestureEndedEventArgs));
test_behavior!(TestScrollGesture: ScrollGestureEventBehavior, ScrollGestureEventBehaviorImpl, on_scroll_gesture(&ScrollGestureEventArgs));
test_behavior!(TestTapped: TappedEventBehavior, TappedEventBehaviorImpl, on_tapped(&dyn IRoutedEventArgs));
test_behavior!(TestTextInput: TextInputEventBehavior, TextInputEventBehaviorImpl, on_text_input(&TextInputEventArgs));
test_behavior!(TestTextInputMethodClientRequested: TextInputMethodClientRequestedEventBehavior, TextInputMethodClientRequestedEventBehaviorImpl, on_text_input_method_client_requested(&TextInputMethodClientRequestedEventArgs));

/// Shows a host with `behavior` attached to its border, raises the event with `raise` and checks
/// that the hook ran with the border as the sender, then removes the border from the visual tree
/// and checks that raising the event again does not call the hook.
fn assert_behavior_calls_hook(
    behavior: Ref<FerroObject>,
    count: impl Fn() -> i32,
    sender: impl Fn() -> Option<Ref<Interactive>>,
    raise: impl Fn(&Host),
) {
    let host = Host::new();
    host.add_behavior(behavior);
    host.window.show();

    assert_eq!(0, count());

    raise(&host);

    assert!(count() >= 1, "the hook runs when the event is raised");
    let border: Ref<Interactive> = host.border.clone().upcast();
    assert_eq!(Some(border), sender(), "the sender is the associated object");

    host.detach_border();
    let calls = count();
    raise(&host);

    assert_eq!(calls, count(), "the handler is removed when the border leaves the visual tree");

    host.window.close();
}

/// Runs [`assert_behavior_calls_hook`] for a behavior made by `$new` with the event raised by
/// `$raise`.
macro_rules! behavior_test {
    ($test:ident, $type_:ident, |$host:ident| $raise:expr) => {
        #[test]
        fn $test() {
            let _app = start_app();
            let behavior = $type_::new();
            let counted = behavior.clone();
            let recorded = behavior.clone();
            assert_behavior_calls_hook(
                object(&behavior),
                move || counted.count(),
                move || recorded.sender(),
                |$host: &Host| {
                    $raise;
                },
            );
        }
    };
}

behavior_test!(double_tapped_event_behavior_calls_on_double_tapped, TestDoubleTapped, |host| host
    .raise(host.tapped_event_args(InputElement::double_tapped_event())));
behavior_test!(got_focus_event_behavior_calls_on_got_focus, TestGotFocus, |host| host
    .raise(host.focus_changed_event_args(InputElement::got_focus_event())));
behavior_test!(key_down_event_behavior_calls_on_key_down, TestKeyDown, |host| host
    .raise(host.key_event_args(InputElement::key_down_event())));
behavior_test!(key_up_event_behavior_calls_on_key_up, TestKeyUp, |host| host
    .raise(host.key_event_args(InputElement::key_up_event())));
behavior_test!(lost_focus_event_behavior_calls_on_lost_focus, TestLostFocus, |host| host
    .raise(host.focus_changed_event_args(InputElement::lost_focus_event())));
behavior_test!(pointer_capture_lost_event_behavior_calls_on_pointer_capture_lost, TestPointerCaptureLost, |host| host
    .raise(host.pointer_capture_lost_event_args()));
behavior_test!(pointer_entered_event_behavior_calls_on_pointer_enter, TestPointerEntered, |host| host
    .raise(host.pointer_event_args(InputElement::pointer_entered_event())));
behavior_test!(pointer_exited_event_behavior_calls_on_pointer_leave, TestPointerExited, |host| host
    .raise(host.pointer_event_args(InputElement::pointer_exited_event())));
behavior_test!(pointer_moved_event_behavior_calls_on_pointer_moved, TestPointerMoved, |host| host
    .raise(host.pointer_event_args(InputElement::pointer_moved_event())));
behavior_test!(pointer_pressed_event_behavior_calls_on_pointer_pressed, TestPointerPressed, |host| host
    .raise(host.pointer_pressed_event_args()));
behavior_test!(pointer_released_event_behavior_calls_on_pointer_released, TestPointerReleased, |host| host
    .raise(host.pointer_released_event_args()));
behavior_test!(pointer_wheel_changed_event_behavior_calls_on_pointer_wheel_changed, TestPointerWheelChanged, |host| host
    .raise(host.pointer_wheel_event_args()));
behavior_test!(right_tapped_event_behavior_calls_on_right_tapped, TestRightTapped, |host| host
    .raise(host.tapped_event_args(InputElement::right_tapped_event())));
behavior_test!(scroll_gesture_ended_event_behavior_calls_on_scroll_gesture_ended, TestScrollGestureEnded, |host| host
    .raise(host.scroll_gesture_ended_event_args()));
behavior_test!(scroll_gesture_event_behavior_calls_on_scroll_gesture, TestScrollGesture, |host| host
    .raise(host.scroll_gesture_event_args()));
behavior_test!(tapped_event_behavior_calls_on_tapped, TestTapped, |host| host
    .raise(host.tapped_event_args(InputElement::tapped_event())));
behavior_test!(text_input_event_behavior_calls_on_text_input, TestTextInput, |host| host
    .raise(host.text_input_event_args()));
behavior_test!(
    text_input_method_client_requested_event_behavior_calls_on_text_input_method_client_requested,
    TestTextInputMethodClientRequested,
    |host| host.raise(host.text_input_method_client_requested_event_args())
);

/// A `PointerEventsBehavior` that counts the calls of each of its hooks.
#[repr(C)]
struct TestPointerEvents {
    base: PointerEventsBehavior,
    pressed: Cell<i32>,
    released: Cell<i32>,
    moved: Cell<i32>,
}

ferro_class!(TestPointerEvents: PointerEventsBehavior);
ferroui_base::ferro_impl_classes!(TestPointerEvents: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

impl PointerEventsBehaviorImpl for TestPointerEvents {
    fn on_pointer_pressed(this: &Self, _sender: &Interactive, _e: &PointerPressedEventArgs) {
        this.pressed.set(this.pressed.get() + 1);
    }

    fn on_pointer_released(this: &Self, _sender: &Interactive, _e: &PointerReleasedEventArgs) {
        this.released.set(this.released.get() + 1);
    }

    fn on_pointer_moved(this: &Self, _sender: &Interactive, _e: &PointerEventArgs) {
        this.moved.set(this.moved.get() + 1);
    }
}

impl TestPointerEvents {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: PointerEventsBehavior::construct(),
            pressed: Cell::new(0),
            released: Cell::new(0),
            moved: Cell::new(0),
        })
    }
}

#[test]
fn pointer_events_behavior_calls_the_hook_of_each_pointer_event() {
    let _app = start_app();
    let host = Host::new();
    let behavior = TestPointerEvents::new();
    host.add_behavior(object(&behavior));
    host.window.show();

    host.raise(host.pointer_pressed_event_args());
    let pressed = behavior.pressed.get();
    assert!(pressed >= 1);
    assert_eq!(0, behavior.released.get());
    assert_eq!(0, behavior.moved.get());

    host.raise(host.pointer_released_event_args());
    let released = behavior.released.get();
    assert!(released >= 1);
    assert_eq!(pressed, behavior.pressed.get());
    assert_eq!(0, behavior.moved.get());

    host.raise(host.pointer_event_args(InputElement::pointer_moved_event()));
    let moved = behavior.moved.get();
    assert!(moved >= 1);
    assert_eq!(pressed, behavior.pressed.get());
    assert_eq!(released, behavior.released.get());

    host.detach_border();
    host.raise(host.pointer_pressed_event_args());
    host.raise(host.pointer_released_event_args());
    host.raise(host.pointer_event_args(InputElement::pointer_moved_event()));

    assert_eq!(pressed, behavior.pressed.get());
    assert_eq!(released, behavior.released.get());
    assert_eq!(moved, behavior.moved.get());

    host.window.close();
}

#[test]
fn behavior_does_not_call_the_hook_for_other_events() {
    let _app = start_app();
    let host = Host::new();
    let behavior = TestKeyDown::new();
    host.add_behavior(object(&behavior));
    host.window.show();

    host.raise(host.key_event_args(InputElement::key_up_event()));
    host.raise(host.text_input_event_args());

    assert_eq!(0, behavior.count());

    host.window.close();
}

#[test]
fn routing_strategies_default_to_the_routes_of_the_reference() {
    let _app = start_app();
    let bubble = RoutingStrategies::BUBBLE;
    let direct = RoutingStrategies::DIRECT;
    let tunnel_bubble = RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE;

    assert_eq!(bubble, TestDoubleTapped::new().routing_strategies());
    assert_eq!(bubble, TestGotFocus::new().routing_strategies());
    assert_eq!(tunnel_bubble, TestKeyDown::new().routing_strategies());
    assert_eq!(tunnel_bubble, TestKeyUp::new().routing_strategies());
    assert_eq!(bubble, TestLostFocus::new().routing_strategies());
    assert_eq!(direct, TestPointerCaptureLost::new().routing_strategies());
    assert_eq!(direct, TestPointerEntered::new().routing_strategies());
    assert_eq!(tunnel_bubble, TestPointerEvents::new().routing_strategies());
    assert_eq!(direct, TestPointerExited::new().routing_strategies());
    assert_eq!(tunnel_bubble, TestPointerMoved::new().routing_strategies());
    assert_eq!(tunnel_bubble, TestPointerPressed::new().routing_strategies());
    assert_eq!(tunnel_bubble, TestPointerReleased::new().routing_strategies());
    assert_eq!(tunnel_bubble, TestPointerWheelChanged::new().routing_strategies());
    assert_eq!(bubble, TestRightTapped::new().routing_strategies());
    assert_eq!(bubble, TestScrollGestureEnded::new().routing_strategies());
    assert_eq!(bubble, TestScrollGesture::new().routing_strategies());
    assert_eq!(bubble, TestTapped::new().routing_strategies());
    assert_eq!(tunnel_bubble, TestTextInput::new().routing_strategies());
    assert_eq!(tunnel_bubble, TestTextInputMethodClientRequested::new().routing_strategies());
}

/// A tapped behavior that records whether the args it receives are the args of a tap.
#[repr(C)]
struct ArgsTapped {
    base: TappedEventBehavior,
    is_tapped_args: Cell<bool>,
}

ferro_class!(ArgsTapped: TappedEventBehavior);
ferroui_base::ferro_impl_classes!(ArgsTapped: FerroObjectImpl, StyledElementImpl, StyledElementBehaviorImpl);

impl TappedEventBehaviorImpl for ArgsTapped {
    fn on_tapped(this: &Self, _sender: &Interactive, e: &dyn IRoutedEventArgs) {
        this.is_tapped_args.set(e.is::<TappedEventArgs>());
    }
}

#[test]
fn tapped_event_behavior_hook_receives_the_tapped_args() {
    let _app = start_app();
    let host = Host::new();
    let behavior = instantiate(ArgsTapped { base: TappedEventBehavior::construct(), is_tapped_args: Cell::new(false) });
    host.add_behavior(object(&behavior));
    host.window.show();

    host.raise(host.tapped_event_args(InputElement::tapped_event()));

    // The reference passes the `TappedEventArgs` as `RoutedEventArgs`; a deriving class can cast
    // them back, as here.
    assert!(behavior.is_tapped_args.get());

    host.window.close();
}
