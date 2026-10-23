//! Tests of this port for `ElementCompositionPreview`, in a window rendered by the test
//! compositor.

use super::ElementCompositionPreview;
use crate::compositor_hit_testing_tests::CompositorTestServices;
use crate::{Border, Canvas, Control, ScrollViewer};
use ferroui_base::media::Brushes;
use ferroui_base::numerics::Vector3;
use ferroui_base::rendering::composition::animations::ICompositionAnimationBase;
use ferroui_base::rendering::composition::server::ServerCompositionVisual;
use ferroui_base::rendering::composition::{CompositionGetValueStatus, CompositionVisual, Compositor};
use ferroui_base::{Ref, Size, Vector, Visual};
use std::rc::Rc;
use std::time::{Duration, Instant};

fn scene() -> (CompositorTestServices, Ref<Canvas>) {
    let services = CompositorTestServices::new(Size::new(400.0, 300.0));
    let canvas = Canvas::new();
    services.set_content(&canvas.clone().upcast::<Control>());
    services.run_jobs();
    (services, canvas)
}

fn border() -> Ref<Border> {
    let border = Border::new();
    border.set_width(50.0);
    border.set_height(40.0);
    border.set_background(Some(Brushes::blue()));
    Canvas::set_left(&border, 10.0);
    Canvas::set_top(&border, 20.0);
    border
}

fn element_visual(visual: &Visual) -> Rc<CompositionVisual> {
    ElementCompositionPreview::get_element_visual(visual).expect("the element has a composition visual")
}

fn compositor_of(visual: &Visual) -> Rc<Compositor> {
    element_visual(visual).compositor().clone()
}

fn server(visual: &Rc<CompositionVisual>) -> Rc<ServerCompositionVisual> {
    visual.compositor().server().get::<ServerCompositionVisual>(visual.server()).expect("the server visual exists")
}

/// An animation of `Opacity` from 0 to 1 over `duration`.
fn fade(compositor: &Rc<Compositor>, from: f32, to: f32, duration: Duration) -> Rc<dyn ICompositionAnimationBase> {
    let animation = compositor.create_scalar_key_frame_animation();
    animation.insert_key_frame(0.0, from);
    animation.insert_key_frame(1.0, to);
    animation.set_duration(duration);
    animation.set_target(Some("Opacity".to_owned()));
    animation
}

fn run_until(services: &CompositorTestServices, done: impl Fn() -> bool) {
    let started = Instant::now();
    while !done() && started.elapsed() < Duration::from_secs(2) {
        services.run_jobs();
    }
}

#[test]
fn the_show_animation_runs_when_the_element_is_added() {
    let (services, canvas) = scene();
    let probe = border();
    canvas.children().add(probe.clone());
    services.run_jobs();
    let compositor = compositor_of(&probe);
    canvas.children().remove(&probe);
    services.run_jobs();

    let element = border();
    // An animation that holds the opacity at a quarter shows that it started.
    let hold = compositor.create_expression_animation_with("0.25");
    hold.set_target(Some("Opacity".to_owned()));
    ElementCompositionPreview::set_implicit_show_animation(&element, Some(hold));
    canvas.children().add(element.clone());
    services.run_jobs();
    services.run_jobs();
    let visual = element_visual(&element);
    assert_eq!(server(&visual).opacity(), 0.25);
}

#[test]
fn the_hide_animation_keeps_the_element_visible_until_it_completes() {
    let (services, canvas) = scene();
    let element = border();
    canvas.children().add(element.clone());
    services.run_jobs();
    let compositor = compositor_of(&element);
    ElementCompositionPreview::set_implicit_hide_animation(
        &element,
        Some(fade(&compositor, 1.0, 0.0, Duration::from_millis(100))),
    );

    element.set_is_visible(false);
    services.run_jobs();
    let visual = element_visual(&element);
    assert!(visual.visible(), "the element is shown while it hides");
    assert!(element.is_shown_by_composition());

    run_until(&services, || !element.is_shown_by_composition());
    services.run_jobs();
    assert!(!element.is_shown_by_composition());
    assert!(!visual.visible());
}

#[test]
fn the_hide_animation_of_a_removed_element_plays_on_its_image() {
    let (services, canvas) = scene();
    let element = border();
    canvas.children().add(element.clone());
    services.run_jobs();
    let compositor = compositor_of(&element);
    let canvas_visual = element_visual(&canvas);
    ElementCompositionPreview::set_implicit_hide_animation(
        &element,
        Some(fade(&compositor, 1.0, 0.0, Duration::from_millis(100))),
    );

    canvas.children().remove(&element);
    services.run_jobs();
    // The image of the element stays in the canvas while the animation runs.
    assert_eq!(canvas_visual.children().count(), 1);
    let image = canvas_visual.children().get(0);
    assert_eq!(image.size(), Vector::new(50.0, 40.0));

    run_until(&services, || canvas_visual.children().count() == 0);
    assert_eq!(canvas_visual.children().count(), 0);
}

#[test]
fn the_manipulation_property_set_follows_the_offset_of_the_scroll_viewer() {
    let (services, canvas) = scene();
    let scroll_viewer = ScrollViewer::new();
    canvas.children().add(scroll_viewer.clone());
    services.run_jobs();
    // The window has no theme: the extent and viewport a template would give.
    scroll_viewer.set_extent(Size::new(100.0, 1000.0));
    scroll_viewer.set_viewport(Size::new(100.0, 100.0));

    let set = ElementCompositionPreview::get_scroll_viewer_manipulation_property_set(&scroll_viewer)
        .expect("a compositor renders the window");
    assert_eq!(set.try_get_vector3("Translation"), (CompositionGetValueStatus::Succeeded, Vector3::new(0.0, 0.0, 0.0)));
    scroll_viewer.set_offset(Vector::new(0.0, 120.0));
    services.run_jobs();
    assert_eq!(set.try_get_vector3("Translation"), (CompositionGetValueStatus::Succeeded, Vector3::new(0.0, -120.0, 0.0)));
    assert_eq!(set.try_get_vector3("Scale"), (CompositionGetValueStatus::Succeeded, Vector3::new(1.0, 1.0, 1.0)));

    // The same set for the same scroll viewer.
    let again = ElementCompositionPreview::get_scroll_viewer_manipulation_property_set(&scroll_viewer).unwrap();
    assert!(Rc::ptr_eq(&set, &again));
}

#[test]
fn the_pointer_position_property_set_has_a_position() {
    let (services, canvas) = scene();
    let element = border();
    canvas.children().add(element.clone());
    services.run_jobs();
    let set = ElementCompositionPreview::get_pointer_position_property_set(&element).expect("a compositor renders the window");
    assert_eq!(set.try_get_vector3("Position"), (CompositionGetValueStatus::Succeeded, Vector3::new(0.0, 0.0, 0.0)));
    assert!(set.is_live());
}

#[test]
fn translation_is_always_enabled() {
    let (services, canvas) = scene();
    let element = border();
    canvas.children().add(element.clone());
    services.run_jobs();
    ElementCompositionPreview::set_is_translation_enabled(&element, true);
    element.set_translation(ferroui_base::Vector3D::new(5.0, 6.0, 0.0));
    services.run_jobs();
    assert_eq!(element_visual(&element).translation(), ferroui_base::Vector3D::new(5.0, 6.0, 0.0));
}
