use super::IDrawingContextImpl;
use crate::rendering::composition::CompositionTransparencyLevel;
use crate::{PixelSize, RenderTargetCorruptedException, RenderTargetNotReadyException, Size};
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
    NotReady(RenderTargetNotReadyException),
    /// The target can no longer be drawn to.
    Corrupted(RenderTargetCorruptedException),
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

impl From<RenderTargetNotReadyException> for RenderTargetError {
    fn from(error: RenderTargetNotReadyException) -> Self {
        RenderTargetError::NotReady(error)
    }
}

impl From<RenderTargetCorruptedException> for RenderTargetError {
    fn from(error: RenderTargetCorruptedException) -> Self {
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

    /// Releases the render target.
    fn dispose(&self);
}
