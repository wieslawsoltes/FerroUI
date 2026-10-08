use crate::platform::LtrbRect;

/// Receives debugging information from a composition target while it renders
/// a frame.
pub trait ICompositionTargetDebugEvents: Send + Sync {
    /// The number of visuals rendered during the frame.
    fn rendered_visuals(&self) -> i32;

    /// Sets the number of visuals rendered during the frame.
    fn set_rendered_visuals(&self, value: i32);

    /// The number of visuals visited during the frame.
    fn visited_visuals(&self) -> i32;

    /// Sets the number of visuals visited during the frame.
    fn set_visited_visuals(&self, value: i32);

    /// Called for every rectangle that gets invalidated.
    fn rect_invalidated(&self, rc: LtrbRect);
}
