//! The options of the DRM output (the port of `DrmOutputOptions.cs`).

use crate::DrmConnectorType;
use ferroui_base::media::Color;
use ferroui_base::platform::SurfaceOrientation;
use ferroui_base::PixelSize;

/// DRM Output Options
#[derive(Clone, Debug, PartialEq)]
pub struct DrmOutputOptions {
    /// Scaling factor.
    /// Default: 1.0
    pub scaling: f64,

    /// The orientation of the screen relative to the frame buffer memory orientation
    /// Default: Normal
    pub orientation: SurfaceOrientation,

    /// If true an two cycle buffer swapping is processed at init.
    /// Default: True
    pub enable_initial_buffer_swapping: bool,

    /// Color for [`enable_initial_buffer_swapping`](Self::enable_initial_buffer_swapping)
    /// Default: R0 G0 B0 A0
    pub initial_buffer_swapping_color: Color,

    /// Specifies the video mode with which the DrmOutput should be created, if it is not found it will fallback to the preferred mode.
    /// If null, the preferred mode will be used.
    pub video_mode: Option<PixelSize>,

    /// Specifies whether our connector is HDMI-A, DVI, DisplayPort, etc.
    /// If null, the preferred connector will be used.
    pub connector_type: Option<DrmConnectorType>,

    /// Specifies the connector type ID used with [`connector_type`](Self::connector_type).
    /// If null, the preferred connector type ID will be used.
    pub connector_type_id: Option<u32>,
    /// Enables partial presentation where EGL supports it, rotated outputs included: a frame
    /// repaints what changed in its buffer and is presented with the region that changed
    /// (upstream 070c3be990). `None`, the default, lets the platform decide (currently: off).
    pub use_partial_presentation: Option<bool>,
}

impl Default for DrmOutputOptions {
    fn default() -> Self {
        Self {
            scaling: 1.0,
            orientation: SurfaceOrientation::Rotation0,
            enable_initial_buffer_swapping: true,
            initial_buffer_swapping_color: Color::new(0, 0, 0, 0),
            video_mode: None,
            connector_type: None,
            connector_type_id: None,
            use_partial_presentation: None,
        }
    }
}
