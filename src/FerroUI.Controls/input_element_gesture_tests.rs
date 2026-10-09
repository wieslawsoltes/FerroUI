use crate::Border;
use ferroui_base::input::{InputElement, SwipeGestureEndedEventArgs};
use ferroui_base::Vector;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn swipe_gesture_ended_public_event_can_be_observed() {
    let target = Border::new();
    // The args the handler received, by address: the args are passed by
    // reference.
    let received: Rc<Cell<Option<*const SwipeGestureEndedEventArgs>>> = Rc::new(Cell::new(None));

    let r = received.clone();
    target.swipe_gesture_ended(move |_, e| r.set(Some(e as *const SwipeGestureEndedEventArgs)));

    let args = SwipeGestureEndedEventArgs::new(42, Vector::new(12.0, 34.0));
    target.raise_event(&args);

    assert_eq!(Some(&args as *const SwipeGestureEndedEventArgs), received.get());
    assert_eq!(args.routed_event().unwrap(), *InputElement::swipe_gesture_ended_event());
}
