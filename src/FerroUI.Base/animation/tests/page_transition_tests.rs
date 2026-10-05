use super::*;
use crate::animation::{
    CompositePageTransition, CrossFade, IPageTransition, IProgressPageTransition, IterationCount, PageSlide,
    Rotate3DTransition, SlideAxis,
};
use crate::threading::{CancellationToken, CancellationTokenSource, Dispatcher};

fn pages() -> (Ref<TestRoot>, Ref<Visual>, Ref<Visual>) {
    let root = TestRoot::new();
    root.set_bounds(Rect::new(0.0, 0.0, 200.0, 100.0));
    let from = Border::new();
    let to = Border::new();
    to.set_is_visible(false);
    root.set_child(&from);
    root.set_child(&to);
    (root, from.upcast(), to.upcast())
}

fn run_jobs() {
    Dispatcher::current_dispatcher().run_jobs(None);
}

#[test]
fn cross_fade_fades_and_hides_the_old_page_through_the_dispatcher() {
    let clock = start();
    let (_root, from, to) = pages();
    let transition = CrossFade::with_duration(seconds(1.0));

    let task = transition.start(Some(&from), Some(&to), true, CancellationToken::none());

    // The body runs up to its first await right away.
    assert!(to.is_visible());
    assert!(!task.is_completed());

    clock.pulse(seconds(0.0));
    clock.pulse(seconds(0.5));
    assert_eq!(from.opacity(), 0.5);
    assert_eq!(to.opacity(), 0.5);

    clock.pulse(seconds(1.0));
    assert_eq!(from.opacity(), 0.0);
    assert_eq!(to.opacity(), 1.0);

    // The animations have ended; the continuation is resumed by the
    // dispatcher.
    assert!(from.is_visible());
    assert!(!task.is_completed());
    run_jobs();
    assert!(task.is_completed_successfully());
    assert!(!from.is_visible());
}

#[test]
fn cancelled_cross_fade_completes_and_leaves_the_old_page_visible() {
    let clock = start();
    let (_root, from, to) = pages();
    let transition = CrossFade::with_duration(seconds(1.0));
    let source = CancellationTokenSource::new();

    let task = transition.start(Some(&from), Some(&to), true, source.token());
    clock.pulse(seconds(0.0));
    clock.pulse(seconds(0.5));

    source.cancel();
    run_jobs();

    assert!(task.is_completed_successfully());
    assert!(from.is_visible());
}

#[test]
fn transition_started_with_a_cancelled_token_completes_immediately() {
    start();
    let (_root, from, to) = pages();
    let source = CancellationTokenSource::new();
    source.cancel();

    let task = PageSlide::with_duration(seconds(1.0), SlideAxis::Horizontal).start(
        Some(&from),
        Some(&to),
        true,
        source.token(),
    );

    assert!(task.is_completed_successfully());
    assert!(!to.is_visible());
}

#[test]
fn page_slide_slides_both_pages_and_resets_them() {
    let clock = start();
    let (_root, from, to) = pages();
    let transition = PageSlide::with_duration(seconds(1.0), SlideAxis::Horizontal);

    let task = transition.start(Some(&from), Some(&to), true, CancellationToken::none());

    clock.pulse(seconds(0.0));
    clock.pulse(seconds(0.5));
    assert_eq!(from.render_transform().unwrap().value(), Matrix::create_translation(-100.0, 0.0));
    assert_eq!(to.render_transform().unwrap().value(), Matrix::create_translation(100.0, 0.0));

    clock.pulse(seconds(1.0));
    run_jobs();

    assert!(task.is_completed_successfully());
    assert!(!from.is_visible());
    assert!(to.is_visible());
    assert!(from.render_transform().is_none());
    assert!(to.render_transform().is_none());
}

#[test]
fn rotate_3d_transition_ends_with_the_new_page_on_top() {
    let clock = start();
    let (_root, from, to) = pages();
    let transition = Rotate3DTransition::with_duration(seconds(1.0), SlideAxis::Horizontal, None);

    let task = transition.start(Some(&from), Some(&to), true, CancellationToken::none());

    clock.pulse(seconds(0.0));
    clock.pulse(seconds(0.5));
    clock.pulse(seconds(1.0));
    // Each animation animates several properties: their completion and the
    // continuation of the transition both go through the dispatcher.
    run_jobs();
    run_jobs();

    assert!(task.is_completed_successfully());
    assert_eq!(to.z_index(), 2);
    assert_eq!(from.z_index(), 1);
    assert!(!from.is_visible());
    assert!(to.is_visible());
}

#[test]
fn composite_page_transition_completes_when_all_transitions_have() {
    let clock = start();
    let (_root, from, to) = pages();
    let composite = CompositePageTransition::new();
    composite.add(Rc::new(CrossFade::with_duration(seconds(1.0))));
    composite.add(Rc::new(PageSlide::with_duration(seconds(2.0), SlideAxis::Vertical)));

    let task = composite.start(Some(&from), Some(&to), true, CancellationToken::none());

    clock.pulse(seconds(0.0));
    clock.pulse(seconds(1.0));
    run_jobs();
    assert!(!task.is_completed());

    clock.pulse(seconds(2.0));
    run_jobs();
    run_jobs();
    assert!(task.is_completed_successfully());
}

#[test]
fn progress_updates_are_applied_and_reset() {
    let (_root, from, to) = pages();

    let cross_fade = CrossFade::new();
    cross_fade.update(0.25, Some(&from), Some(&to), true, 0.0, &[]);
    assert_eq!(from.opacity(), 0.75);
    assert_eq!(to.opacity(), 0.25);
    assert!(to.is_visible());
    cross_fade.reset(&from);
    assert_eq!(from.opacity(), 1.0);

    let slide = PageSlide::new();
    slide.update(0.25, Some(&from), Some(&to), true, 0.0, &[]);
    assert_eq!(from.render_transform().unwrap().value(), Matrix::create_translation(-50.0, 0.0));
    assert_eq!(to.render_transform().unwrap().value(), Matrix::create_translation(150.0, 0.0));
    slide.reset(&from);
    assert!(from.render_transform().is_none());

    let rotate = Rotate3DTransition::new();
    rotate.update(0.75, Some(&from), Some(&to), true, 0.0, &[]);
    assert_eq!(from.z_index(), 1);
    assert_eq!(to.z_index(), 2);
    rotate.reset(&to);
    assert!(to.render_transform().is_none());
    assert_eq!(to.z_index(), 0);
}

#[test]
fn transition_whose_animation_cannot_run_fails() {
    start();
    let (_root, from, _to) = pages();
    // An animation of a single property that cannot be run with `run_async`
    // is not reachable through the built-in transitions; the failure path is
    // exercised through the helper they share.
    let animation = crate::animation::Animation::new();
    animation.set_iteration_count(IterationCount::INFINITE);
    let run = animation.run_async(&from, CancellationToken::none());
    let task = crate::animation::start_page_transition(async move {
        crate::animation::when_all_animations(vec![run]).await;
    });
    assert!(task.is_faulted());
}
