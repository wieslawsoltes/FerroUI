use super::InputElement;
use crate::interactivity::RoutedEventArgs;
use crate::{ferro_routed_event_args, Vector};
use std::sync::atomic::{AtomicI32, Ordering};

static NEXT_ID: AtomicI32 = AtomicI32::new(1);

/// Provides data for the pull gesture event.
#[derive(Clone)]
pub struct PullGestureEventArgs {
    base: RoutedEventArgs,
    id: i32,
    delta: Vector,
    pull_direction: PullDirection,
}

ferro_routed_event_args!(PullGestureEventArgs: RoutedEventArgs);

impl PullGestureEventArgs {
    /// Creates pull gesture event args.
    pub fn new(id: i32, delta: Vector, pull_direction: PullDirection) -> Self {
        Self {
            base: RoutedEventArgs::with_event(InputElement::pull_gesture_event()),
            id,
            delta,
            pull_direction,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn get_next_free_id() -> i32 {
        NEXT_ID.fetch_add(1, Ordering::Relaxed)
    }

    /// The identifier of the gesture.
    #[inline]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// The distance pulled so far.
    #[inline]
    pub fn delta(&self) -> Vector {
        self.delta
    }

    /// The direction of the pull.
    #[inline]
    pub fn pull_direction(&self) -> PullDirection {
        self.pull_direction
    }
}

/// Provides data for the pull gesture ended event.
#[derive(Clone)]
pub struct PullGestureEndedEventArgs {
    base: RoutedEventArgs,
    id: i32,
    pull_direction: PullDirection,
}

ferro_routed_event_args!(PullGestureEndedEventArgs: RoutedEventArgs);

impl PullGestureEndedEventArgs {
    /// Creates pull gesture ended event args.
    pub fn new(id: i32, pull_direction: PullDirection) -> Self {
        Self { base: RoutedEventArgs::with_event(InputElement::pull_gesture_ended_event()), id, pull_direction }
    }

    /// The identifier of the gesture.
    #[inline]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// The direction of the pull.
    #[inline]
    pub fn pull_direction(&self) -> PullDirection {
        self.pull_direction
    }
}

/// The direction of a pull gesture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PullDirection {
    TopToBottom,
    BottomToTop,
    LeftToRight,
    RightToLeft,
}
