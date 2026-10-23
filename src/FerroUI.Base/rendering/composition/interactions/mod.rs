//! `Microsoft.UI.Composition.Interactions`: interaction trackers and the
//! visual interaction sources that drive them with input.
//!
//! An [`InteractionTracker`] holds a position and a scale. Input redirected
//! by a [`VisualInteractionSource`] moves them while interacting, inertia
//! (with its decay rates and inertia modifiers) carries them to rest after a
//! release, and requests (`try_update_*`) move them from code. Expression
//! animations reference the tracker to move visuals with it.
//!
//! The tracker runs on the UI thread and ticks with the animation frames of
//! the media context; see the deviations of `docs/porting/winui-composition.md`.

mod interaction_tracker;
mod interaction_tracker_inertia_modifier;
mod visual_interaction_source;

pub use interaction_tracker::{
    CompositionInteractionSourceCollection, IInteractionTrackerOwner, InteractionTracker, InteractionTrackerAnimation,
    InteractionTrackerClampingOption, InteractionTrackerCustomAnimationStateEnteredArgs,
    InteractionTrackerIdleStateEnteredArgs, InteractionTrackerInertiaStateEnteredArgs,
    InteractionTrackerInteractingStateEnteredArgs, InteractionTrackerRequestIgnoredArgs, InteractionTrackerState,
    InteractionTrackerValuesChangedArgs, DEFAULT_INERTIA_DECAY_RATE,
};
pub use interaction_tracker_inertia_modifier::{
    InteractionTrackerInertiaModifier, InteractionTrackerInertiaMotion, InteractionTrackerInertiaRestingValue,
};
pub use visual_interaction_source::{
    InteractionChainingMode, InteractionSourceConfiguration, InteractionSourceMode, InteractionSourceRedirectionMode,
    VisualInteractionSource, VisualInteractionSourceRedirectionMode,
};

#[cfg(test)]
mod interaction_tracker_tests;
