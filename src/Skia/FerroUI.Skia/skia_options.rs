/// Options for the Skia rendering subsystem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkiaOptions {
    /// The maximum number of bytes for video memory to store textures and
    /// resources.
    ///
    /// This is set by default to the minimum value that seems to make
    /// rendering comfortable on a range of hardware. `None` means the Skia
    /// default.
    pub max_gpu_resource_size_bytes: Option<i64>,

    /// Use a save layer to apply opacity, so that every visual is rendered to
    /// its own layer when opacity is applied.
    pub use_opacity_save_layer: bool,

    /// Whether Skia may use stencil buffers. Only an explicit `Some(true)`
    /// enables them; see [`should_avoid_stencil_buffers`](Self::should_avoid_stencil_buffers).
    pub use_stencil_buffers: Option<bool>,
}

impl Default for SkiaOptions {
    fn default() -> Self {
        Self {
            // ~28mb: 12x 1024 x 600 textures.
            max_gpu_resource_size_bytes: Some(1024 * 600 * 4 * 12),
            use_opacity_save_layer: false,
            use_stencil_buffers: None,
        }
    }
}

impl SkiaOptions {
    /// Creates the default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Stencil buffers let Skia choose multisample-based path rendering,
    /// which quantizes edge coverage and visibly degrades vector geometry
    /// anti-aliasing on GPU backends. They are a performance opt-in, so
    /// anything other than an explicit `true` avoids them.
    pub fn should_avoid_stencil_buffers(use_stencil_buffers: Option<bool>) -> bool {
        use_stencil_buffers != Some(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stencil_buffers_are_avoided_unless_explicitly_enabled() {
        for (use_stencil_buffers, expected) in [(None, true), (Some(false), true), (Some(true), false)] {
            assert_eq!(expected, SkiaOptions::should_avoid_stencil_buffers(use_stencil_buffers));
        }
    }
}
