//! Ported from `tests/Xaml.Behaviors.Animations.UnitTests/AnimationRunnerTests.cs`.

use super::support::{control, run_jobs, start_app};
use crate::{AnimationRunner, IAnimationBuilder};
use ferroui_base::animation::{Animatable, Animation, TimeSpan};
use ferroui_base::Ref;
use ferroui_controls::{Border, Control};
use std::cell::Cell;

#[derive(Default)]
struct NullAnimationBuilder {
    build_count: Cell<i32>,
}

impl NullAnimationBuilder {
    fn build_count(&self) -> i32 {
        self.build_count.get()
    }
}

impl IAnimationBuilder for NullAnimationBuilder {
    fn build(&self, _control: &Control) -> Option<Ref<Animation>> {
        self.build_count.set(self.build_count.get() + 1);
        None
    }
}

#[test]
fn try_run_returns_false_for_missing_inputs() {
    let _app = start_app();
    let border = Border::new();
    let target: &Animatable = &border;

    assert!(!AnimationRunner::try_run(None, None));
    assert!(!AnimationRunner::try_run(None, Some(target)));
    assert!(AnimationRunner::try_run_async(None, None).is_none());
    assert!(AnimationRunner::try_run_async(None, Some(target)).is_none());
}

#[test]
fn try_build_and_run_uses_builder_when_explicit_animation_is_missing() {
    let _app = start_app();
    let builder = NullAnimationBuilder::default();
    let border = Border::new();

    let started = AnimationRunner::try_build_and_run(Some(control(&border)), None, Some(&builder));

    assert!(!started);
    assert_eq!(builder.build_count(), 1);
}

#[test]
fn try_build_and_run_does_not_use_builder_without_target() {
    let _app = start_app();
    let builder = NullAnimationBuilder::default();

    let started = AnimationRunner::try_build_and_run(None, None, Some(&builder));

    assert!(!started);
    assert_eq!(builder.build_count(), 0);
}

#[test]
fn try_build_and_run_async_returns_completion_task_for_explicit_animation() {
    let app = start_app();
    let builder = NullAnimationBuilder::default();
    let animation = Animation::new();
    animation.set_duration(TimeSpan::ZERO);
    let border = Border::new();

    let task = AnimationRunner::try_build_and_run_async(Some(control(&border)), Some(&animation), Some(&builder));

    let task = task.expect("the animation was started");
    assert_eq!(builder.build_count(), 0);
    // `await task`: the completion of the run is posted to the dispatcher (and ticks of the
    // global clock, which the headless platform of the reference makes, drive the animators).
    run_jobs();
    app.clock.pulse(TimeSpan::ZERO);
    run_jobs();
    assert!(matches!(task.result(), Some(Ok(()))));
}
