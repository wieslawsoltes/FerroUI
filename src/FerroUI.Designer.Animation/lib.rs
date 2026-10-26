//! Renderer-independent animation authoring. Times are seconds; colors are unpremultiplied sRGB.
//! Samples can be applied to a live preview without modifying authored XAML, and recording adds
//! keyframes rather than baking transient preview values into document attributes.
mod easing;
mod interactions;
mod programs;
mod states;
mod timeline;
mod value;
mod xaml;

pub use easing::Easing;
pub use interactions::{BehaviorAction, BehaviorModel, BehaviorTrigger};
pub use programs::{
    fragment as animation_fragment, native_fragment, rekey_copied_programs, AnimationCodec,
    AnimationCodecRegistry, AnimationProgram, AnimationPrograms,
};
pub use states::{
    resolve_state_target, rewrite_design_metadata, state_value, StateSetter, StateTransition,
    VisualState, VisualStateGroup,
};
pub use timeline::{
    CompiledTimeline, FillMode, Keyframe, PlaybackBehavior, PlaybackDirection, Player, Repeat,
    Sample, Timeline, Track,
};
pub use value::{AnimationValue, ValueKind};
pub use xaml::{AnimationImport, ImportOptions};

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
mod program_tests;
