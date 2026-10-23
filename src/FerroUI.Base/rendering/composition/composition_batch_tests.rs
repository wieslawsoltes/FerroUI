//! Tests of this port for the batches of the compositor
//! (`CompositionScopedBatch`, `CompositionCommitBatch`).

use super::animations::{AnimationIterationBehavior, ScalarKeyFrameAnimation};
use super::server::ServerCompositionVisual;
use super::test_compositor::TestCompositor;
use super::{CompositionBatchTypes, CompositionSolidColorVisual, ICompositionObjectAnimations};
use crate::animation::easings::LinearEasing;
use crate::threading::Dispatcher;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// A compositor with one visual whose server side is active, so its
/// animations run on the server clock.
///
/// The fields drop in order: the visual before the compositor of the test,
/// which ends the scopes the compositor runs in.
struct Fixture {
    server: Rc<ServerCompositionVisual>,
    visual: CompositionSolidColorVisual,
    test: TestCompositor,
}

impl Fixture {
    fn new() -> Fixture {
        let test = TestCompositor::new();
        let visual = test.compositor.create_solid_color_visual();
        test.run_jobs();
        let server = test.server::<ServerCompositionVisual>(visual.server());
        server.activate();
        Fixture { server, visual, test }
    }

    /// An opacity animation to `value` over `duration`.
    fn opacity_animation(&self, value: f32, duration: Duration) -> Rc<ScalarKeyFrameAnimation> {
        let animation = self.test.compositor.create_scalar_key_frame_animation();
        animation.insert_key_frame_with_easing(1.0, value, Rc::new(LinearEasing::new()));
        animation.set_duration(duration);
        animation
    }

    /// Renders frames for `duration` of the server clock, then runs the jobs
    /// of the dispatcher (where `Completed` is raised).
    fn run_for(&self, duration: Duration) {
        let server = self.test.compositor.server();
        let started = server.server_now();
        while server.server_now() < started + duration {
            self.test.render_loop.tick();
        }
        Dispatcher::ui_thread().run_jobs(None);
    }
}

/// A counter of `Completed`.
fn counter() -> (Rc<Cell<u32>>, impl Fn(&super::CompositionBatchCompletedEventArgs) + 'static) {
    let count = Rc::new(Cell::new(0));
    let handler = {
        let count = count.clone();
        move |_: &super::CompositionBatchCompletedEventArgs| count.set(count.get() + 1)
    };
    (count, handler)
}

#[test]
fn scoped_batch_completes_when_its_key_frame_animation_ends() {
    let fixture = Fixture::new();
    let compositor = &fixture.test.compositor;
    let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    assert!(batch.is_active());
    assert!(!batch.is_ended());

    let animation = fixture.opacity_animation(0.5, Duration::from_millis(50));
    fixture.visual.start_animation("Opacity", &*animation);
    batch.end();
    assert!(!batch.is_active());
    assert!(batch.is_ended());
    // As in WinUI, the handler may be subscribed after `End`.
    let (completed, handler) = counter();
    let _subscription = batch.completed(handler);

    fixture.test.run_jobs();
    assert_eq!(completed.get(), 0, "the animation is running");

    fixture.run_for(Duration::from_millis(150));
    assert_eq!(fixture.server.opacity(), 0.5);
    assert_eq!(completed.get(), 1);

    // Completed is raised once.
    fixture.run_for(Duration::from_millis(20));
    assert_eq!(completed.get(), 1);
}

#[test]
fn scoped_batch_ended_without_animations_completes_asynchronously() {
    let fixture = Fixture::new();
    let batch = fixture.test.compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    batch.end();
    let (completed, handler) = counter();
    let _subscription = batch.completed(handler);
    assert_eq!(completed.get(), 0);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(completed.get(), 1);

    // Ending again does nothing.
    batch.end();
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(completed.get(), 1);
}

#[test]
fn dropped_scoped_batch_still_raises_completed() {
    let fixture = Fixture::new();
    let batch = fixture.test.compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    let (completed, handler) = counter();
    let _subscription = batch.completed(handler);
    batch.end();
    drop(batch);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(completed.get(), 1);
}

#[test]
fn suspended_scoped_batch_excludes_animations() {
    let fixture = Fixture::new();
    let compositor = &fixture.test.compositor;
    let batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);

    batch.suspend();
    assert!(!batch.is_active());
    assert!(!batch.is_ended());
    // Started while suspended: not part of the batch.
    let long = fixture.opacity_animation(0.25, Duration::from_secs(30));
    fixture.visual.start_animation("RotationAngle", &*long);
    batch.resume();
    assert!(batch.is_active());

    let short = fixture.opacity_animation(0.5, Duration::from_millis(50));
    fixture.visual.start_animation("Opacity", &*short);
    batch.end();
    let (completed, handler) = counter();
    let _subscription = batch.completed(handler);

    fixture.test.run_jobs();
    assert_eq!(completed.get(), 0);
    fixture.run_for(Duration::from_millis(150));
    assert_eq!(completed.get(), 1, "the excluded animation is still running");

    fixture.visual.stop_animation("RotationAngle");
    fixture.test.run_jobs();
}

#[test]
fn suspend_after_end_panics() {
    let fixture = Fixture::new();
    let batch = fixture.test.compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    batch.end();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| batch.suspend()));
    assert!(result.is_err());
    Dispatcher::ui_thread().run_jobs(None);
}

#[test]
fn scoped_batch_completes_when_its_animation_is_stopped() {
    let fixture = Fixture::new();
    let batch = fixture.test.compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    let animation = fixture.opacity_animation(0.5, Duration::from_secs(30));
    fixture.visual.start_animation("Opacity", &*animation);
    batch.end();
    let (completed, handler) = counter();
    let _subscription = batch.completed(handler);

    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(20));
    assert_eq!(completed.get(), 0);

    fixture.visual.stop_animation("Opacity");
    fixture.test.run_jobs();
    assert_eq!(completed.get(), 1);
}

#[test]
fn scoped_batch_completes_when_its_animation_is_replaced() {
    let fixture = Fixture::new();
    let batch = fixture.test.compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    let animation = fixture.opacity_animation(0.5, Duration::from_secs(30));
    fixture.visual.start_animation("Opacity", &*animation);
    batch.end();
    let (completed, handler) = counter();
    let _subscription = batch.completed(handler);
    fixture.test.run_jobs();
    assert_eq!(completed.get(), 0);

    // Replaced on the server by an animation outside the batch.
    let other = fixture.opacity_animation(0.25, Duration::from_secs(30));
    fixture.visual.start_animation("Opacity", &*other);
    fixture.test.run_jobs();
    assert_eq!(completed.get(), 1);
    fixture.visual.stop_animation("Opacity");
    fixture.test.run_jobs();
}

#[test]
fn scoped_batch_completes_when_its_pending_animation_is_overwritten() {
    let fixture = Fixture::new();
    let batch = fixture.test.compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    let animation = fixture.opacity_animation(0.5, Duration::from_secs(30));
    fixture.visual.start_animation("Opacity", &*animation);
    // Setting the value drops the animation before it reaches the server.
    fixture.visual.set_opacity(0.75);
    batch.end();
    let (completed, handler) = counter();
    let _subscription = batch.completed(handler);
    fixture.test.run_jobs();
    assert_eq!(completed.get(), 1);
    assert_eq!(fixture.server.opacity(), 0.75);
}

#[test]
fn scoped_batch_completes_when_the_target_is_disposed() {
    let fixture = Fixture::new();
    let batch = fixture.test.compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    let animation = fixture.opacity_animation(0.5, Duration::from_secs(30));
    fixture.visual.start_animation("Opacity", &*animation);
    batch.end();
    let (completed, handler) = counter();
    let _subscription = batch.completed(handler);
    fixture.test.run_jobs();
    assert_eq!(completed.get(), 0);

    fixture.visual.dispose();
    fixture.test.run_jobs();
    assert_eq!(completed.get(), 1);
}

#[test]
fn infinite_animations_belong_to_infinite_batches_only() {
    let fixture = Fixture::new();
    let compositor = &fixture.test.compositor;
    let finite_batch = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    let all_batch = compositor.create_scoped_batch(CompositionBatchTypes::ALL_ANIMATIONS);
    let animation = fixture.opacity_animation(0.5, Duration::from_millis(50));
    animation.set_iteration_behavior(AnimationIterationBehavior::Forever);
    fixture.visual.start_animation("Opacity", &*animation);
    finite_batch.end();
    all_batch.end();
    let (finite_completed, finite_handler) = counter();
    let _finite = finite_batch.completed(finite_handler);
    let (all_completed, all_handler) = counter();
    let _all = all_batch.completed(all_handler);

    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(150));
    assert_eq!(finite_completed.get(), 1, "an Animation batch does not wait for an infinite animation");
    assert_eq!(all_completed.get(), 0, "the infinite animation is still running");

    fixture.visual.stop_animation("Opacity");
    fixture.test.run_jobs();
    assert_eq!(all_completed.get(), 1);
}

#[test]
fn nested_scoped_batches_both_track_an_animation() {
    let fixture = Fixture::new();
    let compositor = &fixture.test.compositor;
    let outer = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    let inner = compositor.create_scoped_batch(CompositionBatchTypes::ANIMATION);
    let animation = fixture.opacity_animation(0.5, Duration::from_millis(50));
    fixture.visual.start_animation("Opacity", &*animation);
    inner.end();
    let late = fixture.opacity_animation(0.5, Duration::from_millis(600));
    fixture.visual.start_animation("RotationAngle", &*late);
    outer.end();
    let (inner_completed, inner_handler) = counter();
    let _inner = inner.completed(inner_handler);
    let (outer_completed, outer_handler) = counter();
    let _outer = outer.completed(outer_handler);

    fixture.test.run_jobs();
    fixture.run_for(Duration::from_millis(150));
    assert_eq!(inner_completed.get(), 1);
    assert_eq!(outer_completed.get(), 0, "the later animation of the outer batch is running");
    fixture.run_for(Duration::from_millis(600));
    assert_eq!(outer_completed.get(), 1);
}

#[test]
fn commit_batch_tracks_the_animations_of_its_frame() {
    let fixture = Fixture::new();
    let compositor = &fixture.test.compositor;
    // Started before the batch is asked for: still part of the frame.
    let animation = fixture.opacity_animation(0.5, Duration::from_millis(50));
    fixture.visual.start_animation("Opacity", &*animation);
    let batch = compositor.get_commit_batch(CompositionBatchTypes::ANIMATION);
    assert!(batch.is_active());
    assert!(!batch.is_ended());
    assert!(Rc::ptr_eq(&batch, &compositor.get_commit_batch(CompositionBatchTypes::ANIMATION)));
    let (completed, handler) = counter();
    let _subscription = batch.completed(handler);

    fixture.test.run_jobs();
    assert!(batch.is_ended());
    assert!(!batch.is_active());
    assert_eq!(completed.get(), 0);

    // The next frame has a batch of its own.
    let next = compositor.get_commit_batch(CompositionBatchTypes::ANIMATION);
    assert!(!Rc::ptr_eq(&batch, &next));
    let (next_completed, next_handler) = counter();
    let _next = next.completed(next_handler);

    fixture.run_for(Duration::from_millis(150));
    assert_eq!(completed.get(), 1);
    // Nothing was started in the next frame, which is committed because
    // the batch was asked for.
    fixture.test.run_jobs();
    assert_eq!(next_completed.get(), 1);
}

#[test]
fn create_property_set_creates_a_property_set() {
    let fixture = Fixture::new();
    let set = fixture.test.compositor.create_property_set();
    set.insert_scalar("Weight", 0.5);
    assert_eq!(set.try_get_scalar("Weight").1, 0.5);
}
