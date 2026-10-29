//! Ported from `Animations/AnimationRunner.cs`.

use crate::IAnimationBuilder;
use ferroui_base::animation::{Animatable, Animation, AnimationTask};
use ferroui_base::threading::CancellationToken;
use ferroui_controls::Control;

/// Builds and runs animations independently of behaviors, actions, and triggers.
///
/// The `Task` of the reference is the [`AnimationTask`] of the run.
pub struct AnimationRunner;

impl AnimationRunner {
    /// Runs an animation on an animatable target. Returns a task that completes when the
    /// animation finishes.
    pub fn run_async(animation: &Animation, target: &Animatable) -> AnimationTask {
        animation.run_async(target, CancellationToken::none())
    }

    /// Starts an animation when both the animation and target are available. Returns `true` when
    /// the animation was started; otherwise, `false`.
    pub fn try_run(animation: Option<&Animation>, target: Option<&Animatable>) -> bool {
        Self::try_run_async(animation, target).is_some()
    }

    /// Starts an animation when both the animation and target are available and returns its
    /// completion task, or `None` when the animation could not be started.
    pub fn try_run_async(animation: Option<&Animation>, target: Option<&Animatable>) -> Option<AnimationTask> {
        match (animation, target) {
            (Some(animation), Some(target)) => Some(Self::run_async(animation, target)),
            _ => None,
        }
    }

    /// Selects an explicit animation or builds one and starts it on a control: `animation` is
    /// the preferred animation, `animation_builder` the fallback animation builder. Returns
    /// `true` when an animation was started; otherwise, `false`.
    pub fn try_build_and_run(
        control: Option<&Control>,
        animation: Option<&Animation>,
        animation_builder: Option<&dyn IAnimationBuilder>,
    ) -> bool {
        Self::try_build_and_run_async(control, animation, animation_builder).is_some()
    }

    /// Selects an explicit animation or builds one, starts it on a control, and returns its
    /// completion task, or `None` when no animation could be started.
    pub fn try_build_and_run_async(
        control: Option<&Control>,
        animation: Option<&Animation>,
        animation_builder: Option<&dyn IAnimationBuilder>,
    ) -> Option<AnimationTask> {
        let control = control?;

        let built;
        let selected_animation = match animation {
            Some(animation) => Some(animation),
            None => {
                built = animation_builder.and_then(|builder| builder.build(control));
                built.as_deref()
            }
        };
        let target: &Animatable = control;
        Self::try_run_async(selected_animation, Some(target))
    }
}
