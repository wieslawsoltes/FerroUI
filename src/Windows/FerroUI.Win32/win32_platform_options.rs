//! The options of the Windows backend.

use ferroui_base::platform::{IPlatformGraphics, PlatformGraphicsDeviceAdapterDescription};
use std::rc::Rc;
use std::sync::Arc;

/// Represents the rendering mode for platform graphics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Win32RenderingMode {
    /// The toolkit is rendered into a framebuffer.
    Software = 1,
    /// Enables ANGLE EGL for Windows with GPU rendering.
    AngleEgl = 2,
    /// The toolkit would try to use native Windows OpenGL with GPU
    /// rendering.
    Wgl = 3,
    /// The toolkit would try to use native Windows Vulkan with GPU
    /// rendering.
    Vulkan = 4,
}

/// Represents the DPI Awareness for the application.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Win32DpiAwareness {
    /// The application is DPI unaware.
    Unaware,
    /// The application is system DPI aware. It will query DPI once and will
    /// not adjust to new DPI changes.
    SystemDpiAware,
    /// The application is per-monitor DPI aware. It adjust its scale factor
    /// whenever DPI changes.
    #[default]
    PerMonitorDpiAware,
}

/// Represents the Win32 window composition mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum Win32CompositionMode {
    /// Render to a texture inside the Windows.UI.Composition tree.
    ///
    /// Supported on Windows 10 build 17134 and above. Ignored on other
    /// versions. This is recommended option, as it allows window acrylic
    /// effects and high refresh rate rendering. Can only be applied with
    /// [`Win32RenderingMode::AngleEgl`].
    WinUIComposition = 1,
    /// Render to a texture inside the DirectComposition tree.
    ///
    /// Supported on Windows 8 and above. Ignored on other versions. Can
    /// only be applied with [`Win32RenderingMode::AngleEgl`].
    DirectComposition = 2,
    /// When [`LowLatencyDxgiSwapChain`](Self::LowLatencyDxgiSwapChain) is
    /// active, renders to a swap chain using a DXGI waitable object, which
    /// is non blocking, and gives the lowest input latency.
    ///
    /// Requires Windows 8 and above. Can only be applied with
    /// [`Win32RenderingMode::AngleEgl`].
    LowLatencyDxgiSwapChain = 3,
    /// The window renders to a redirection surface.
    ///
    /// This option is kept only for compatibility with older systems. Some
    /// features (like transparency) might not work.
    RedirectionSurface = 4,
}

/// The callback that chooses a graphics adapter: it is given the adapters
/// of the system and returns the index of the one to use.
pub type GraphicsAdapterSelectionCallback = Rc<dyn Fn(&[PlatformGraphicsDeviceAdapterDescription]) -> i32>;

/// Platform-specific options which apply to Windows.
#[derive(Clone)]
pub struct Win32PlatformOptions {
    /// Embeds popups to the window when set to true. The default value is
    /// false.
    pub overlay_popups: bool,

    /// Gets or sets Win32 rendering mode priority. The first available mode
    /// is used; the default is ANGLE EGL with a software fallback.
    pub rendering_mode: Vec<Win32RenderingMode>,

    /// Gets or sets Win32 composition mode priority. The first available
    /// mode is used.
    pub composition_mode: Vec<Win32CompositionMode>,

    /// When [`composition_mode`](Self::composition_mode) is set to
    /// [`Win32CompositionMode::WinUIComposition`], create rounded corner
    /// blur brushes. If set to `None` the brushes will be created using
    /// default settings (sharp corners).
    pub win_ui_composition_backdrop_corner_radius: Option<f32>,

    /// Render directly on the UI thread instead of using a dedicated render
    /// thread. This can be usable if your device don't have multiple cores
    /// to begin with. This setting is false by default.
    pub should_render_on_ui_thread: bool,

    /// Provides a way to use a custom-implemented graphics context such as
    /// a custom ISkiaGpu. When this property is set,
    /// [`rendering_mode`](Self::rendering_mode) is ignored and
    /// [`composition_mode`](Self::composition_mode) only accepts null or
    /// [`Win32CompositionMode::RedirectionSurface`].
    pub custom_platform_graphics: Option<Arc<dyn IPlatformGraphics>>,

    /// Gets or sets the application's DPI awareness.
    pub dpi_awareness: Win32DpiAwareness,

    /// Provides a callback to select a graphics adapter (with the DirectX
    /// and Vulkan rendering modes).
    pub graphics_adapter_selection_callback: Option<GraphicsAdapterSelectionCallback>,
}

impl Default for Win32PlatformOptions {
    fn default() -> Self {
        Self {
            overlay_popups: false,
            rendering_mode: vec![Win32RenderingMode::AngleEgl, Win32RenderingMode::Software],
            composition_mode: vec![
                Win32CompositionMode::WinUIComposition,
                Win32CompositionMode::DirectComposition,
                Win32CompositionMode::RedirectionSurface,
            ],
            win_ui_composition_backdrop_corner_radius: None,
            should_render_on_ui_thread: false,
            custom_platform_graphics: None,
            dpi_awareness: Win32DpiAwareness::PerMonitorDpiAware,
            graphics_adapter_selection_callback: None,
        }
    }
}

impl Win32PlatformOptions {
    /// Whether the options are valid with a custom graphics context: the
    /// composition mode has to leave the redirection surface in place.
    pub(crate) fn custom_platform_graphics_is_compatible(&self) -> bool {
        self.composition_mode.contains(&Win32CompositionMode::RedirectionSurface)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_those_of_the_reference() {
        let options = Win32PlatformOptions::default();
        assert!(!options.overlay_popups);
        assert_eq!(options.rendering_mode, [Win32RenderingMode::AngleEgl, Win32RenderingMode::Software]);
        assert_eq!(
            options.composition_mode,
            [
                Win32CompositionMode::WinUIComposition,
                Win32CompositionMode::DirectComposition,
                Win32CompositionMode::RedirectionSurface
            ]
        );
        assert_eq!(options.win_ui_composition_backdrop_corner_radius, None);
        assert!(!options.should_render_on_ui_thread);
        assert!(options.custom_platform_graphics.is_none());
        assert_eq!(options.dpi_awareness, Win32DpiAwareness::PerMonitorDpiAware);
        assert!(options.graphics_adapter_selection_callback.is_none());
    }

    #[test]
    fn the_values_of_the_modes_are_those_of_the_reference() {
        assert_eq!(Win32RenderingMode::Software as i32, 1);
        assert_eq!(Win32RenderingMode::AngleEgl as i32, 2);
        assert_eq!(Win32RenderingMode::Wgl as i32, 3);
        assert_eq!(Win32RenderingMode::Vulkan as i32, 4);
        assert_eq!(Win32CompositionMode::WinUIComposition as i32, 1);
        assert_eq!(Win32CompositionMode::DirectComposition as i32, 2);
        assert_eq!(Win32CompositionMode::LowLatencyDxgiSwapChain as i32, 3);
        assert_eq!(Win32CompositionMode::RedirectionSurface as i32, 4);
        assert_eq!(Win32DpiAwareness::Unaware as i32, 0);
        assert_eq!(Win32DpiAwareness::PerMonitorDpiAware as i32, 2);
    }

    #[test]
    fn a_custom_graphics_context_needs_the_redirection_surface() {
        let mut options = Win32PlatformOptions::default();
        assert!(options.custom_platform_graphics_is_compatible());
        options.composition_mode = vec![Win32CompositionMode::WinUIComposition];
        assert!(!options.custom_platform_graphics_is_compatible());
    }
}
