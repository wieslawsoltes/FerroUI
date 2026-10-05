use ferroui_base::PixelPoint;

/// Provides [`PixelPoint`] data for events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelPointEventArgs {
    point: PixelPoint,
}

impl PixelPointEventArgs {
    /// Creates the event args.
    pub fn new(point: PixelPoint) -> Self {
        Self { point }
    }

    /// Gets the [`PixelPoint`] data.
    pub fn point(&self) -> PixelPoint {
        self.point
    }
}
