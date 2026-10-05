use bitflags::bitflags;

bitflags! {
    /// Represents the various types of overlays that can be drawn by a
    /// renderer.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
    pub struct RendererDebugOverlays: i32 {
        /// Do not draw any overlay.
        const NONE = 0;

        /// Draw a FPS counter.
        const FPS = 1 << 0;

        /// Draw invalidated rectangles each frame.
        const DIRTY_RECTS = 1 << 1;

        /// Draw a graph of past layout times.
        const LAYOUT_TIME_GRAPH = 1 << 2;

        /// Draw a graph of past render times.
        const RENDER_TIME_GRAPH = 1 << 3;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_match_the_reference() {
        assert_eq!(RendererDebugOverlays::NONE.bits(), 0);
        assert_eq!(RendererDebugOverlays::FPS.bits(), 1);
        assert_eq!(RendererDebugOverlays::DIRTY_RECTS.bits(), 2);
        assert_eq!(RendererDebugOverlays::LAYOUT_TIME_GRAPH.bits(), 4);
        assert_eq!(RendererDebugOverlays::RENDER_TIME_GRAPH.bits(), 8);
        assert_eq!(RendererDebugOverlays::default(), RendererDebugOverlays::NONE);
    }
}
