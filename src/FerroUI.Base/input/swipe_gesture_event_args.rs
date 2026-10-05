use super::{InputElement, SwipeDirection};
use crate::interactivity::RoutedEventArgs;
use crate::{ferro_routed_event_args, Vector};
use std::sync::atomic::{AtomicI32, Ordering};

static NEXT_ID: AtomicI32 = AtomicI32::new(0);

/// Provides data for swipe gesture events.
#[derive(Clone)]
pub struct SwipeGestureEventArgs {
    base: RoutedEventArgs,
    id: i32,
    delta: Vector,
    velocity: Vector,
    swipe_direction: SwipeDirection,
}

ferro_routed_event_args!(SwipeGestureEventArgs: RoutedEventArgs);

impl SwipeGestureEventArgs {
    /// Creates swipe gesture event args.
    ///
    /// `id` is the unique identifier for this gesture, `delta` the pixel
    /// delta since the last event and `velocity` the current swipe velocity
    /// in pixels per second.
    pub fn new(id: i32, delta: Vector, velocity: Vector) -> Self {
        let swipe_direction = if delta.x.abs() >= delta.y.abs() {
            if delta.x <= 0.0 {
                SwipeDirection::Right
            } else {
                SwipeDirection::Left
            }
        } else if delta.y <= 0.0 {
            SwipeDirection::Down
        } else {
            SwipeDirection::Up
        };

        Self {
            base: RoutedEventArgs::with_event(InputElement::swipe_gesture_event()),
            id,
            delta,
            velocity,
            swipe_direction,
        }
    }

    /// The unique identifier for this gesture sequence.
    #[inline]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// The pixel delta since the last event.
    #[inline]
    pub fn delta(&self) -> Vector {
        self.delta
    }

    /// The current swipe velocity in pixels per second.
    #[inline]
    pub fn velocity(&self) -> Vector {
        self.velocity
    }

    /// The direction of the dominant swipe axis.
    #[inline]
    pub fn swipe_direction(&self) -> SwipeDirection {
        self.swipe_direction
    }

    #[allow(dead_code)]
    pub(crate) fn get_next_free_id() -> i32 {
        NEXT_ID.fetch_add(1, Ordering::Relaxed) + 1
    }
}

/// Provides data for the swipe gesture ended event.
#[derive(Clone)]
pub struct SwipeGestureEndedEventArgs {
    base: RoutedEventArgs,
    id: i32,
    velocity: Vector,
}

ferro_routed_event_args!(SwipeGestureEndedEventArgs: RoutedEventArgs);

impl SwipeGestureEndedEventArgs {
    /// Creates swipe gesture ended event args.
    ///
    /// `id` is the unique identifier for this gesture and `velocity` the
    /// swipe velocity at release in pixels per second.
    pub fn new(id: i32, velocity: Vector) -> Self {
        Self { base: RoutedEventArgs::with_event(InputElement::swipe_gesture_ended_event()), id, velocity }
    }

    /// The unique identifier for this gesture sequence.
    #[inline]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// The swipe velocity at release in pixels per second.
    #[inline]
    pub fn velocity(&self) -> Vector {
        self.velocity
    }
}
