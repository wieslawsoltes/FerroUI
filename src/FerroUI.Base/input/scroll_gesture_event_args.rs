use super::InputElement;
use crate::interactivity::RoutedEventArgs;
use crate::{ferro_routed_event_args, Vector};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicI32, Ordering};

static NEXT_ID: AtomicI32 = AtomicI32::new(1);

/// Provides data for the scroll gesture event.
#[derive(Clone)]
pub struct ScrollGestureEventArgs {
    base: RoutedEventArgs,
    id: i32,
    delta: Vector,
    should_end_scroll_gesture: Rc<Cell<bool>>,
}

ferro_routed_event_args!(ScrollGestureEventArgs: RoutedEventArgs);

impl ScrollGestureEventArgs {
    /// Creates scroll gesture event args.
    pub fn new(id: i32, delta: Vector) -> Self {
        Self {
            base: RoutedEventArgs::with_event(InputElement::scroll_gesture_event()),
            id,
            delta,
            should_end_scroll_gesture: Rc::new(Cell::new(false)),
        }
    }

    /// Returns a new gesture identifier.
    pub fn get_next_free_id() -> i32 {
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    }

    /// The identifier of the gesture.
    #[inline]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// The distance scrolled since the previous event.
    #[inline]
    pub fn delta(&self) -> Vector {
        self.delta
    }

    /// When set, the scroll gesture recognizer should stop its current
    /// active scroll gesture.
    #[inline]
    pub fn should_end_scroll_gesture(&self) -> bool {
        self.should_end_scroll_gesture.get()
    }

    #[inline]
    pub fn set_should_end_scroll_gesture(&self, value: bool) {
        self.should_end_scroll_gesture.set(value)
    }
}

/// Provides data for the scroll gesture ended event.
#[derive(Clone)]
pub struct ScrollGestureEndedEventArgs {
    base: RoutedEventArgs,
    id: i32,
}

ferro_routed_event_args!(ScrollGestureEndedEventArgs: RoutedEventArgs);

impl ScrollGestureEndedEventArgs {
    /// Creates scroll gesture ended event args.
    pub fn new(id: i32) -> Self {
        Self { base: RoutedEventArgs::with_event(InputElement::scroll_gesture_ended_event()), id }
    }

    /// The identifier of the gesture.
    #[inline]
    pub fn id(&self) -> i32 {
        self.id
    }
}

/// Provides data for the scroll gesture inertia starting event.
#[derive(Clone)]
pub struct ScrollGestureInertiaStartingEventArgs {
    base: RoutedEventArgs,
    id: i32,
    inertia: Vector,
}

ferro_routed_event_args!(ScrollGestureInertiaStartingEventArgs: RoutedEventArgs);

impl ScrollGestureInertiaStartingEventArgs {
    #[allow(dead_code)]
    pub(crate) fn new(id: i32, inertia: Vector) -> Self {
        Self {
            base: RoutedEventArgs::with_event(InputElement::scroll_gesture_inertia_starting_event()),
            id,
            inertia,
        }
    }

    /// The identifier of the gesture.
    #[inline]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// The velocity the inertia starts with.
    #[inline]
    pub fn inertia(&self) -> Vector {
        self.inertia
    }
}
