//! Ported from `Composition/CompositionAnimationDefinition.cs`.

use super::composition_animation_helpers::{ScalarKeyFrame, Vector3KeyFrame};
use ferroui_base::numerics::{Vector2, Vector3};

/// The key frames and initial state of one animation of the composition catalogs.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference's constructor with optional named
/// parameters is a struct literal with `..Default::default()` here; the getters are those of
/// the reference.
#[derive(Clone, Debug, Default)]
pub(crate) struct CompositionAnimationDefinition {
    pub(crate) offset_frames: Option<Vec<Vector3KeyFrame>>,
    pub(crate) scale_frames: Option<Vec<Vector3KeyFrame>>,
    pub(crate) opacity_frames: Option<Vec<ScalarKeyFrame>>,
    pub(crate) rotation_frames: Option<Vec<ScalarKeyFrame>>,
    pub(crate) anchor: Option<Vector2>,
    pub(crate) initial_offset: Option<Vector3>,
    pub(crate) initial_scale: Option<Vector3>,
    pub(crate) initial_opacity: Option<f32>,
    pub(crate) initial_rotation: Option<f32>,
    pub(crate) ensure_center_point: bool,
}

impl CompositionAnimationDefinition {
    pub(crate) fn offset_frames(&self) -> Option<&[Vector3KeyFrame]> {
        self.offset_frames.as_deref()
    }

    pub(crate) fn scale_frames(&self) -> Option<&[Vector3KeyFrame]> {
        self.scale_frames.as_deref()
    }

    pub(crate) fn opacity_frames(&self) -> Option<&[ScalarKeyFrame]> {
        self.opacity_frames.as_deref()
    }

    pub(crate) fn rotation_frames(&self) -> Option<&[ScalarKeyFrame]> {
        self.rotation_frames.as_deref()
    }

    pub(crate) fn anchor(&self) -> Option<Vector2> {
        self.anchor
    }

    pub(crate) fn initial_offset(&self) -> Option<Vector3> {
        self.initial_offset
    }

    pub(crate) fn initial_scale(&self) -> Option<Vector3> {
        self.initial_scale
    }

    pub(crate) fn initial_opacity(&self) -> Option<f32> {
        self.initial_opacity
    }

    pub(crate) fn initial_rotation(&self) -> Option<f32> {
        self.initial_rotation
    }

    pub(crate) fn ensure_center_point(&self) -> bool {
        self.ensure_center_point
    }
}
