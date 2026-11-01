/// The default of [`SkiaOptions::max_gpu_resource_size_bytes`]: 128 MiB.
pub const DEFAULT_MAX_GPU_RESOURCE_SIZE_BYTES: i64 = 128 * 1024 * 1024;

/// Options for the Skia rendering subsystem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkiaOptions {
    /// The maximum number of bytes for video memory to store textures and
    /// resources.
    ///
    /// 128 MiB by default, the budget of the image textures of Vello
    /// (upstream's 28 MiB evicted the textures of images that scrolled out
    /// of view, which were uploaded again when they came back:
    /// docs/porting/gpu-rendering.md, "The image textures of Ganesh").
    /// `None` means the Skia default.
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
            max_gpu_resource_size_bytes: Some(DEFAULT_MAX_GPU_RESOURCE_SIZE_BYTES),
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
