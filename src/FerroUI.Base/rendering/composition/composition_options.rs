/// Options of the composition renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CompositionOptions {
    /// Enables more accurate tracking of dirty rects by utilizing regions if
    /// supported by the underlying drawing context.
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
