use crate::skia_platform::SkiaPlatform;
use ferroui_base::{FerroLocator, LocatorExtensions};

/// The default of [`SkiaOptions::max_gpu_resource_size_bytes`]: 128 MiB.
pub const DEFAULT_MAX_GPU_RESOURCE_SIZE_BYTES: i64 = 128 * 1024 * 1024;

/// The default of [`SkiaOptions::frame_pipelining_bands`].
pub const DEFAULT_FRAME_PIPELINING_BANDS: usize = 2;

/// The most bands [`SkiaOptions::frame_pipelining_bands`] gives a frame.
pub const MAX_FRAME_PIPELINING_BANDS: usize = 16;

/// Whether the frame layer of a window draws a frame while the one before
/// it is presented ([`SkiaOptions::frame_pipelining`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkiaFramePipelining {
    /// Every frame is drawn by the render thread, as upstream draws it.
    Off,
    /// A layer whose frames are late pipelines them, and draws directly
    /// again once they are light. The default.
    #[default]
    Adaptive,
    /// Every frame of a frame layer is pipelined, whatever it costs: for
    /// tests and measurements.
    On,
}

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

    /// Whether a GPU render target makes the GPU pipelines of the effects
    /// the themes register (`effects::register_effect_prewarm`, the acrylic
    /// of the WinUI theme) after the first frame of a window, by drawing them
    /// once offscreen, rather than when a frame first draws them (#437). On
    /// Graphite the pipelines are compiled on a thread of their own, from a
    /// recorder of their own; on Ganesh by the render thread, between two
    /// frames. `true` by default (docs/porting/gpu-rendering.md, "Pipelines
    /// made ahead of their first draw").
    pub prewarm_effect_pipelines: bool,

    /// Whether the frame layer of a window may draw a frame while the one
    /// before it is presented (Graphite on Metal; no other GPU backend has
    /// such a layer, and the option does nothing there).
    ///
    /// A frame with many draws costs the render thread more than a refresh
    /// interval, and the frame rate halves. A layer whose frames are late
    /// hands them, recorded as pictures, to threads that each draw a band
    /// of the layer with a Graphite recorder of their own
    /// ([`frame_pipelining_bands`](Self::frame_pipelining_bands)); the render thread
    /// presents the layer as the frame before left it and is free for the
    /// next frame. Such a frame reaches the screen one frame later than it
    /// was drawn. A layer whose frames are light, or need what only the
    /// render thread draws (image filters, leases of the Skia API, frames of
    /// GPU producers), draws them directly, as upstream does.
    /// [`Adaptive`](SkiaFramePipelining::Adaptive) by default
    /// (docs/porting/desktop-performance.md, section 9.33).
    pub frame_pipelining: SkiaFramePipelining,

    /// How many horizontal bands a pipelined frame is divided into
    /// ([`frame_pipelining`](Self::frame_pipelining)), each drawn by a
    /// thread of its own with a Graphite recorder of its own.
    ///
    /// [`DEFAULT_FRAME_PIPELINING_BANDS`] (two) by default; a value outside
    /// `1..=`[`MAX_FRAME_PIPELINING_BANDS`] is brought into that range. One
    /// band draws the whole frame on one thread beside the render thread.
    /// More bands spread the recording of a frame over more cores; each
    /// band has a glyph atlas of its own and draws what crosses its
    /// boundaries once more, and a band is at least 64 pixels high, so a
    /// low layer has fewer. The threads are made by the first layer of a
    /// GPU context that pipelines a frame, as many as that layer has bands
    /// (docs/porting/desktop-performance.md, section 9.33).
    pub frame_pipelining_bands: usize,
}

impl Default for SkiaOptions {
    fn default() -> Self {
        Self {
            max_gpu_resource_size_bytes: Some(DEFAULT_MAX_GPU_RESOURCE_SIZE_BYTES),
            use_opacity_save_layer: false,
            use_stencil_buffers: None,
            prewarm_effect_pipelines: true,
            frame_pipelining: SkiaFramePipelining::Adaptive,
            frame_pipelining_bands: DEFAULT_FRAME_PIPELINING_BANDS,
        }
    }
}

impl SkiaOptions {
    /// Creates the default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// The options in force on this thread: the ones registered with its
    /// services, else the ones the backend was initialized with (the render
    /// thread has no services), else the defaults.
    pub(crate) fn current() -> Self {
        FerroLocator::current()
            .get_service::<SkiaOptions>()
            .map(|options| *options)
            .or_else(SkiaPlatform::options)
            .unwrap_or_default()
    }

    /// The number of bands of a pipelined frame the options ask for:
    /// [`frame_pipelining_bands`](Self::frame_pipelining_bands) within
    /// `1..=`[`MAX_FRAME_PIPELINING_BANDS`].
    pub fn effective_frame_pipelining_bands(&self) -> usize {
        self.frame_pipelining_bands.clamp(1, MAX_FRAME_PIPELINING_BANDS)
    }

    /// Stencil buffers let Skia choose multisample-based path rendering,
    /// which quantizes edge coverage and visibly degrades vector geometry
    /// anti-aliasing on GPU backends. They are a performance opt-in, so
    /// anything other than an explicit `true` avoids them.
    pub fn should_avoid_stencil_buffers(use_stencil_buffers: Option<bool>) -> bool {
        use_stencil_buffers != Some(true)
    }
}
