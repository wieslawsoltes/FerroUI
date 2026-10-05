use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::server::{CompositionProperty, IServerClockItem, ServerObjectId};
use std::time::Duration;

/// A running animation of one property of a server object.
///
/// An instance is created on the UI thread by the animation it belongs to,
/// crosses to the server in the batch that starts it, and is driven there.
/// It names its target by id: the UI thread never holds a server object.
pub trait IAnimationInstance: IServerClockItem + 'static {
    /// The object whose property is animated.
    fn target_object(&self) -> ServerObjectId;

    /// The value of the animation at `now`.
    fn evaluate(&self, now: Duration, current_value: ExpressionVariant) -> ExpressionVariant;

    /// Attaches the instance to the property it animates.
    fn initialize(&self, started_at: Duration, starting_value: ExpressionVariant, property: &'static CompositionProperty);

    fn activate(&self);

    fn deactivate(&self);

    fn invalidate(&self);
}
