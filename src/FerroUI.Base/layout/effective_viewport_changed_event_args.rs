use crate::Rect;

/// Provides data for the effective viewport changed event of a layoutable.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EffectiveViewportChangedEventArgs {
    effective_viewport: Rect,
}

impl EffectiveViewportChangedEventArgs {
    pub fn new(effective_viewport: Rect) -> Self {
        Self { effective_viewport }
    }

    /// The rect representing the effective viewport.
    ///
    /// The effective viewport is expressed in the coordinate system of the
    /// control that the event is raised on.
    pub fn effective_viewport(&self) -> Rect {
        self.effective_viewport
    }
}
