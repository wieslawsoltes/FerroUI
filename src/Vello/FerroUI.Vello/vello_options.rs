/// The renderer of the Vello project a scene is drawn with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VelloRenderingMode {
    /// `vello_cpu`: the sparse-strips renderer on the processor. Needs no
    /// graphics device.
    Cpu,
    /// The sparse-strips renderer with the strips rasterized and composed
    /// on the GPU (`vello_gpu`, which was `vello_hybrid` until its
    /// release 0.3): needs no compute shaders.
    Hybrid,
    /// `vello`: the compute-shader renderer on `wgpu`.
    Gpu,
}

/// The number of rendering modes.
const MODE_COUNT: usize = 3;

/// Options for the Vello rendering subsystem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VelloOptions {
    /// The rendering modes in the order they are tried: the first one that
    /// is available draws. `None` ends the list.
    ///
    /// Only the CPU mode is built so far; a list without it has no mode
    /// that can draw.
    pub rendering_modes: [Option<VelloRenderingMode>; MODE_COUNT],

    /// Use a layer to apply opacity, so that every visual is rendered to
    /// its own layer when opacity is applied.
    pub use_opacity_save_layer: bool,
}

impl Default for VelloOptions {
    fn default() -> Self {
        Self {
            // The order a desktop wants once every mode is built; until
            // then the first two are skipped as not available.
            rendering_modes: [
                Some(VelloRenderingMode::Gpu),
                Some(VelloRenderingMode::Hybrid),
                Some(VelloRenderingMode::Cpu),
            ],
            use_opacity_save_layer: false,
        }
    }
}

impl VelloOptions {
    /// Creates the default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// The options with one rendering mode and no other to fall back to.
    pub fn with_rendering_mode(mode: VelloRenderingMode) -> Self {
        Self { rendering_modes: [Some(mode), None, None], ..Self::default() }
    }

    /// The rendering modes in the order they are tried.
    pub fn rendering_mode_order(&self) -> Vec<VelloRenderingMode> {
        self.rendering_modes.iter().map_while(|mode| *mode).collect()
    }
}
