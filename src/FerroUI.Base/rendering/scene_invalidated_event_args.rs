use crate::Rect;

/// Provides data for the scene invalidated event of a renderer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneInvalidatedEventArgs {
    dirty_rect: Rect,
}

impl SceneInvalidatedEventArgs {
    pub fn new(dirty_rect: Rect) -> Self {
        Self { dirty_rect }
    }

    /// The invalidated area.
    pub fn dirty_rect(&self) -> Rect {
        self.dirty_rect
    }
}
