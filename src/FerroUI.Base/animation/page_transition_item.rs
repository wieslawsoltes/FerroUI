use crate::{Ref, Visual};

/// Describes a single visible page within a carousel viewport.
#[derive(Clone, Debug, PartialEq)]
pub struct PageTransitionItem {
    /// The index of the page.
    pub index: i32,
    /// The visual of the page.
    pub visual: Ref<Visual>,
    /// The offset of the page from the center of the viewport, in pages.
    pub viewport_center_offset: f64,
}

impl PageTransitionItem {
    pub fn new(index: i32, visual: Ref<Visual>, viewport_center_offset: f64) -> Self {
        Self { index, visual, viewport_center_offset }
    }
}
