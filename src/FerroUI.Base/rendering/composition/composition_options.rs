/// Options of the composition renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CompositionOptions {
    /// Enables more accurate tracking of dirty rects by utilizing regions if
    /// supported by the underlying drawing context. `None` turns it on: only
    /// `Some(false)` keeps the single dirty rectangle (a deviation, see
    /// docs/porting/desktop-performance.md section 9.10).
    pub use_region_dirty_rect_clipping: Option<bool>,
    /// The maximum number of dirty rects to track when region clip is in
    /// use. Setting this to zero or a negative value removes the smarter
    /// algorithm and uses the underlying drawing context region support
    /// directly. The default value is 8.
    pub max_dirty_rects: Option<i32>,
    /// Controls the eagerness of merging dirty rects. Unstable: the default
    /// is subject to change.
    pub dirty_rect_merge_eagerness: Option<f64>,
    /// Enforces dirty contents to be rendered into an extra intermediate
    /// surface before being applied onto the saved frame.
    pub use_save_layer_root_clip: Option<bool>,
}

impl CompositionOptions {
    /// Whether a target tracks its dirty area as a set of rectangles clipped
    /// with a region, when the drawing context supports regions. On unless
    /// `use_region_dirty_rect_clipping` is `Some(false)`.
    pub fn region_dirty_rect_clipping(&self) -> bool {
        self.use_region_dirty_rect_clipping != Some(false)
    }
}

#[cfg(test)]
mod tests {
    use super::CompositionOptions;

    #[test]
    fn region_dirty_rect_clipping_is_on_unless_turned_off() {
        assert!(CompositionOptions::default().region_dirty_rect_clipping());
        let on = CompositionOptions { use_region_dirty_rect_clipping: Some(true), ..Default::default() };
        assert!(on.region_dirty_rect_clipping());
        let off = CompositionOptions { use_region_dirty_rect_clipping: Some(false), ..Default::default() };
        assert!(!off.region_dirty_rect_clipping());
    }
}
