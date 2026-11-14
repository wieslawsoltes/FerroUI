use super::IDrawingContextImpl;
use crate::rendering::composition::CompositionTransparencyLevel;
use crate::{PixelSize, RenderTargetCorruptedError, RenderTargetNotReadyError, Size};
use std::any::Any;
use std::error::Error;
use std::fmt;

/// Why a frame could not be begun on a render target: the two exceptions
/// the compositor of the reference catches around the creation of a
/// drawing context. Neither is a fault: a target that is not ready is
/// asked again later, and a corrupted one is released and created anew.
#[derive(Clone, Debug)]
pub enum RenderTargetError {
    /// The target cannot be drawn to yet.
    NotReady(RenderTargetNotReadyError),
    /// The target can no longer be drawn to.
    Corrupted(RenderTargetCorruptedError),
}

impl fmt::Display for RenderTargetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderTargetError::NotReady(error) => error.fmt(f),
            RenderTargetError::Corrupted(error) => error.fmt(f),
        }
    }
}

impl Error for RenderTargetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            RenderTargetError::NotReady(error) => Some(error),
            RenderTargetError::Corrupted(error) => Some(error),
        }
    }
}

impl From<RenderTargetNotReadyError> for RenderTargetError {
    fn from(error: RenderTargetNotReadyError) -> Self {
        RenderTargetError::NotReady(error)
    }
}

impl From<RenderTargetCorruptedError> for RenderTargetError {
    fn from(error: RenderTargetCorruptedError) -> Self {
        RenderTargetError::Corrupted(error)
    }
}

/// The readiness of a platform render target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlatformRenderTargetState {
    pub is_ready: bool,
    pub will_wake_up_render_loop_when_ready: bool,
    pub is_corrupted: bool,
}

impl PlatformRenderTargetState {
    pub const READY: Self = Self { is_ready: true, will_wake_up_render_loop_when_ready: false, is_corrupted: false };
    pub const NOT_READY_TRY_LATER: Self =
        Self { is_ready: false, will_wake_up_render_loop_when_ready: false, is_corrupted: false };
    pub const CORRUPTED: Self = Self { is_ready: true, will_wake_up_render_loop_when_ready: false, is_corrupted: true };
    pub const DISPOSED: Self = Self { is_ready: false, will_wake_up_render_loop_when_ready: false, is_corrupted: true };
    pub const NOT_READY_WILL_WAKEUP_RENDER_LOOP: Self =
        Self { is_ready: false, will_wake_up_render_loop_when_ready: true, is_corrupted: false };
}

/// Static properties of a render target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderTargetProperties {
    /// Whether the render target retains the contents of the previous frame
    /// when a new drawing context is created.
    pub retains_previous_frame_contents: bool,
    /// Whether it's worth to render directly to this target rather than
    /// through an intermediate surface.
    pub is_suitable_for_direct_rendering: bool,
}

/// Properties of one drawing context created from a render target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderTargetDrawingContextProperties {
    /// Whether the previous frame's contents are available in this context.
    pub previous_frame_is_retained: bool,
}

/// Describes the scene a drawing context is requested for.
#[derive(Clone)]
pub struct RenderTargetSceneInfo {
    /// The scene size in device pixels.
    pub size: PixelSize,
    /// The scaling from logical units to device pixels.
    pub scaling: f64,
    /// The scene size in logical units.
    pub logical_size: Size,
    /// The transparency level the scene is composed with.
    pub transparency_level: CompositionTransparencyLevel,
    /// Backend-specific information about the scene.
    pub platform_specific_scene_info: Option<std::sync::Arc<dyn Any + Send + Sync>>,
}

impl RenderTargetSceneInfo {
    pub fn new(size: PixelSize, scaling: f64, transparency_level: CompositionTransparencyLevel) -> Self {
        Self {
            size,
            scaling,
            logical_size: size.to_size(scaling),
            transparency_level,
            platform_specific_scene_info: None,
        }
    }
}

/// Represents a render target: something a scene can be drawn to.
pub trait IRenderTarget {
    /// The properties of this render target.
    fn properties(&self) -> RenderTargetProperties;

    /// Creates a drawing context for a rendering session.
    fn create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> (Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties);

    /// Creates a drawing context for a rendering session, or says that the
    /// target is not ready or is corrupted: what the reference reports by
    /// throwing `RenderTargetNotReadyException` or
    /// `RenderTargetCorruptedException` out of `CreateDrawingContext`. The
    /// compositor calls this member. A target that cannot report either
    /// keeps the default.
    fn try_create_drawing_context(
        &self,
        scene_info: &RenderTargetSceneInfo,
    ) -> Result<(Box<dyn IDrawingContextImpl>, RenderTargetDrawingContextProperties), RenderTargetError> {
        Ok(self.create_drawing_context(scene_info))
    }

    /// The readiness of the underlying platform target.
    fn platform_render_target_state(&self) -> PlatformRenderTargetState {
        PlatformRenderTargetState::READY
    }

    /// Prepares a frame whose layer is drawn before the drawing context of
    /// the target is created, and returns whether the target allows that:
    /// a layer its drawing contexts created can be drawn while no drawing
    /// context of the target exists, and whatever the target sets up for
    /// the layers of a frame when it creates its drawing context is set up
    /// by this call. `false` by default.
    ///
    /// The compositor asks before every frame it would draw that way, and
    /// then creates the drawing context of the target only to present the
    /// layer: a target whose surface is waited for (a swapchain image, the
    /// drawable of a view) gives the time of the wait to the drawing, and
    /// is not asked at all by a tick that has nothing new to present
    /// (desktop-performance.md, section 9.51). The layer decides as well
    /// (`IDrawingContextLayerImpl::draws_ahead_of_target`).
    ///
    /// Not from upstream, where the drawing context of the target is
    /// created first.
    fn prepare_frame_ahead(&self, scene_info: &RenderTargetSceneInfo) -> bool {
        let _ = scene_info;
        false
    }

    /// Whether the target applies a clip of several rectangles to every
    /// draw under it on its own, where a clip of one rectangle costs
    /// nothing to speak of. Skia's Ganesh is such a target: it has a scissor
    /// for a rectangle, and for a region a mask of the size of each draw,
    /// which it rasterises and uploads for that draw. The composition
    /// target draws a frame of such a target one dirty rectangle at a
    /// time, each under a clip that is that rectangle
    /// (`IDirtyRectTracker::separate_rects`), and under the region of its
    /// dirty rectangles everywhere else (desktop-performance.md, section
    /// 9.56).
    ///
    /// Not from upstream, which clips every frame to the region.
    fn clips_regions_per_draw(&self) -> bool {
        false
    }

    /// Makes the GPU programs of what `paint` draws before a frame needs
    /// them (#437; `effects::register_effect_prewarm`): `paint` draws, once,
    /// with a drawing context of the kind the frames of the target draw
    /// with, onto an offscreen surface of `scene_info.size` pixels of the
    /// kind the frames draw the scene onto, which is never shown. Returns
    /// whether the target prewarmed; `paint` is not called otherwise.
    ///
    /// The compositor calls it between two frames. The default does
    /// nothing: a target that draws with the processor, or whose GPU
    /// programs are made up front, has nothing to prewarm.
    fn prewarm(&self, scene_info: &RenderTargetSceneInfo, paint: &mut dyn FnMut(&mut dyn IDrawingContextImpl)) -> bool {
        let _ = (scene_info, paint);
        false
    }

    /// Releases the render target.
    fn dispose(&self);
}
