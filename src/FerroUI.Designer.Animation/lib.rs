//! Renderer-independent animation authoring. Times are seconds; colors are unpremultiplied sRGB.
//! Samples can be applied to a live preview without modifying authored XAML, and recording adds
//! keyframes rather than baking transient preview values into document attributes.
#[cfg(feature = "native-values")]
mod brush_value;
#[cfg(feature = "native-values")]
mod brush_curve_channel;
mod clip_timing;
mod curve;
mod curve_channel;
mod track_curve_channel;
mod curve_metric;
mod easing;
mod interactions;
mod keyframe_draft;
mod keyframe_edits;
mod keyframe_clipboard;
mod keyframe_mirror;
mod keyframe_selection;
mod preview_markers;
mod preview_range;
mod program_edit;
mod program_locator;
mod programs;
mod property_transitions;
mod states;
mod timeline;
#[cfg(feature = "native-values")]
mod transform_list_value;
#[cfg(feature = "native-values")]
mod transform_curve_channel;
mod value;
mod xaml;

#[cfg(feature = "native-preview")]
pub mod native;

pub use clip_timing::{ClipClock, ClipSpan, clip_clock_delays, clip_delays, xaml_clip_delay, xaml_clip_delay_edit};
pub use curve::{CurvePoint, CurveWindow};
pub use curve_channel::CurveChannel;
pub use track_curve_channel::TrackCurveChannel;
pub use curve_metric::CurveMetric;
pub use easing::{keyframe_easing_presets, Easing, KeyframeEasingPreset};
pub use interactions::{
    BehaviorAction, BehaviorActionSource, BehaviorEdit, BehaviorModel, BehaviorSource, BehaviorTrigger,
};
pub use keyframe_draft::{KeyframeDraftPreview, KeyframeEdit};
pub use keyframe_edits::{KeyframeGroupEdit, KeyframeLocation};
pub use keyframe_clipboard::{CopiedKeyframeTrack, KeyframeClipboard, KeyframeDestination, KeyframePaste};
pub use keyframe_selection::{KeyframeSelection, KeyframeSelectionPlan};
pub use preview_range::PreviewRange;
pub use program_edit::compose_animation_edit;
pub use program_locator::AnimationProgramLocator;
pub use preview_markers::{PreviewMarker, PreviewMarkerEdit, PreviewMarkers};
pub use programs::{
    AnimationClipEdit, AnimationClipPlan,
    fragment as animation_fragment, native_fragment, rekey_copied_programs, AnimationCodec,
    AnimationCodecRegistry, AnimationProgram, AnimationProgramIdentity, AnimationPrograms,
    NativeAnimationCodec,
};
pub use property_transitions::{
    AuthoredPropertyTransition, PropertyTransitionCollectionEdit, PropertyTransitionCommand,
    PropertyTransitionList, PROPERTY_TRANSITION_TYPES,
    PropertyTransitionTiming, XamlTransitionTarget,
};
pub use states::{
    StateGroupEdit, StatePreviewGroup, StatePreviewRoute,
    resolve_state_target, rewrite_design_metadata, state_value, ProjectedStateGroup,
    ProjectedStateRoute, ProjectedStateTrigger, StateSetter, StateTransition, VisualState, VisualStateGroup,
};
pub use timeline::{
    CompiledTimeline, FillMode, Keyframe, PlaybackBehavior, PlaybackDirection, Player, Repeat,
    Sample, Timeline, TimelineTimingEdit, Track,
};
#[cfg(feature = "native-values")]
pub use transform_list_value::{AnimationTransformList, AnimationTransformOperation, TransformOperationKind};
#[cfg(feature = "native-values")]
pub use transform_curve_channel::TransformCurveChannel;
#[cfg(feature = "native-values")]
pub use brush_value::{
    AnimationBrush, AnimationGradient, AnimationGradientStop, BrushForm, GradientDistance,
    GradientPoint, GradientShape, GradientSpread,
};
#[cfg(feature = "native-values")]
pub use brush_curve_channel::{BrushCurveChannel, BrushField};
pub use value::{AnimationEffect, AnimationValue, EffectForm, RelativeUnit, ValueKind};
#[cfg(feature = "native-values")]
pub use value::{AnimationBoxShadow, AnimationBoxShadows};
pub use xaml::{AnimationImport, EffectColorResolver, EffectLiteralResolver, ImportOptions, native_key_time};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnimationError(pub String);
impl std::fmt::Display for AnimationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}
impl std::error::Error for AnimationError {}
pub type Result<T> = std::result::Result<T, AnimationError>;

pub(crate) fn require(condition: bool, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(AnimationError(message.into()))
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod keyframe_transfer_tests;

#[cfg(test)]
mod program_tests;

#[cfg(test)]
mod effect_tests;
#[cfg(test)]
mod effect_object_tests;
