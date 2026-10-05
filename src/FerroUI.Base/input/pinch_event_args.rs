use super::InputElement;
use crate::interactivity::RoutedEventArgs;
use crate::{ferro_routed_event_args, Point};

/// Provides data for the pinch gesture event.
#[derive(Clone)]
pub struct PinchEventArgs {
    base: RoutedEventArgs,
    scale: f64,
    scale_origin: Point,
    angle: f64,
    angle_delta: f64,
}

ferro_routed_event_args!(PinchEventArgs: RoutedEventArgs);

impl PinchEventArgs {
    /// Creates pinch event args with a scale and its origin.
    pub fn new(scale: f64, scale_origin: Point) -> Self {
        Self::with_angle(scale, scale_origin, 0.0, 0.0)
    }

    /// Creates pinch event args with a scale, its origin and the angle of
    /// the gesture.
    pub fn with_angle(scale: f64, scale_origin: Point, angle: f64, angle_delta: f64) -> Self {
        Self {
            base: RoutedEventArgs::with_event(InputElement::pinch_event()),
            scale,
            scale_origin,
            angle,
            angle_delta,
        }
    }

    /// The scale of the pinch relative to its start.
    #[inline]
    pub fn scale(&self) -> f64 {
        self.scale
    }

    /// The origin of the scale.
    #[inline]
    pub fn scale_origin(&self) -> Point {
        self.scale_origin
    }

    /// The angle of the pinch gesture, in degrees.
    ///
    /// A pinch gesture is the movement of two pressed points closer
    /// together. This property is the measured angle of the line between
    /// those two points. Remember zero degrees is a line pointing up.
    #[inline]
    pub fn angle(&self) -> f64 {
        self.angle
    }

    /// The difference from the previous and current pinch angle.
    ///
    /// The value includes the sign of rotation: positive for clockwise,
    /// negative counterclockwise.
    #[inline]
    pub fn angle_delta(&self) -> f64 {
        self.angle_delta
    }
}

/// Provides data for the pinch ended event.
#[derive(Clone)]
pub struct PinchEndedEventArgs {
    base: RoutedEventArgs,
}

ferro_routed_event_args!(PinchEndedEventArgs: RoutedEventArgs);

impl Default for PinchEndedEventArgs {
    fn default() -> Self {
        Self::new()
    }
}

impl PinchEndedEventArgs {
    /// Creates pinch ended event args.
    pub fn new() -> Self {
        Self { base: RoutedEventArgs::with_event(InputElement::pinch_ended_event()) }
    }
}
