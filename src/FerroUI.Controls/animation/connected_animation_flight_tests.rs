//! Tests of this port for the composition engine of connected animations (the port of WinUI's
//! `CConnectedAnimation`, `connected_animation_flight.rs`), in a window rendered by the test
//! compositor.

use super::{
    BasicConnectedAnimationConfiguration, ConnectedAnimationService, DirectConnectedAnimationConfiguration,
    GravityConnectedAnimationConfiguration,
};
use crate::compositor_hit_testing_tests::CompositorTestServices;
use crate::{Border, Canvas, Control};
use ferroui_base::media::Brushes;
use ferroui_base::rendering::composition::CompositionVisual;
use ferroui_base::{Ref, Size, Vector, Vector3D, Visual};
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

struct Scene {
    services: CompositorTestServices,
    canvas: Ref<Canvas>,
    source: Ref<Border>,
    destination: Ref<Border>,
}

fn border(width: f64, height: f64, left: f64, top: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    border.set_background(Some(Brushes::red()));
    Canvas::set_left(&border, left);
    Canvas::set_top(&border, top);
    border
}

/// A 400 x 300 window with a source at (20, 30) of 100 x 50 and a destination at (200, 100) of
/// 150 x 75 on a canvas.
fn scene() -> Scene {
    let services = CompositorTestServices::new(Size::new(400.0, 300.0));
    let canvas = Canvas::new();
    let source = border(100.0, 50.0, 20.0, 30.0);
    let destination = border(150.0, 75.0, 200.0, 100.0);
    canvas.children().add(source.clone());
    canvas.children().add(destination.clone());
    services.set_content(&canvas.clone().upcast::<Control>());
    services.run_jobs();
    Scene { services, canvas, source, destination }
}

fn layer(scene: &Scene) -> Option<Rc<CompositionVisual>> {
    scene.services.top_level().composition_layer_visual()
}

fn element_visual(visual: &Visual) -> Rc<CompositionVisual> {
    (*visual.composition_visual().expect("the element has a composition visual")).clone()
}

/// Runs frames until `done` or two seconds of wall time.
fn run_until(scene: &Scene, done: impl Fn() -> bool) {
    let started = Instant::now();
    while !done() && started.elapsed() < Duration::from_secs(2) {
        scene.services.run_jobs();
    }
    scene.services.run_jobs();
}

#[test]
fn prepare_puts_the_image_of_the_source_in_the_layer_and_hides_the_source() {
    let scene = scene();
    let service = ConnectedAnimationService::get_for_top_level(scene.services.top_level());
    let animation = service.prepare_to_animate("item", &scene.source);
    scene.services.run_jobs();

    let layer = layer(&scene).expect("the window has a connected animation layer");
    assert_eq!(layer.children().count(), 1);
    let host = layer.children().get(0);
    assert_eq!(host.offset(), Vector3D::new(20.0, 30.0, 0.0));
    assert_eq!(host.size(), Vector::new(100.0, 50.0));
    // The image is shown in place of the source, which is hidden.
    assert_eq!(host.children().count(), 1);
    assert!(scene.source.is_hidden_by_composition());
    assert!(!element_visual(&scene.source).visible());

    // Cancel shows the source again and removes the image, without a completed event.
    let completed = Rc::new(Cell::new(false));
    let flag = completed.clone();
    let _subscription = animation.completed(move |_| flag.set(true));
    animation.cancel();
    scene.services.run_jobs();
    assert_eq!(layer.children().count(), 0);
    assert!(!scene.source.is_hidden_by_composition());
    assert!(element_visual(&scene.source).visible());
    assert!(!completed.get());
    assert!(!animation.try_start(&scene.destination));
}

#[test]
fn a_started_animation_flies_to_the_destination_and_completes() {
    let scene = scene();
    let service = ConnectedAnimationService::get_for_top_level(scene.services.top_level());
    let animation = service.prepare_to_animate("item", &scene.source);
    animation.set_configuration(Some(BasicConnectedAnimationConfiguration::new()));
    let completed = Rc::new(Cell::new(None));
    let flag = completed.clone();
    let _subscription = animation.completed(move |e| flag.set(Some(e.cancelled())));

    assert!(animation.try_start(&scene.destination));
    assert!(animation.is_consumed());
    // The destination is hidden from its first frame; the flight shows it.
    assert!(scene.destination.is_hidden_by_composition());
    scene.services.run_jobs();

    // The host now shows both images and scales from the source to the destination.
    let layer = layer(&scene).expect("the window has a connected animation layer");
    let host = layer.children().get(0);
    assert_eq!(host.children().count(), 2);

    run_until(&scene, || completed.get().is_some());
    assert_eq!(completed.get(), Some(false));
    assert_eq!(layer.children().count(), 0);
    assert!(!scene.source.is_hidden_by_composition());
    assert!(!scene.destination.is_hidden_by_composition());
    assert!(service.get_animation("item").is_none());
}

#[test]
fn the_gravity_configuration_adds_a_shadow_under_the_images() {
    let scene = scene();
    let service = ConnectedAnimationService::get_for_top_level(scene.services.top_level());
    let animation = service.prepare_to_animate("item", &scene.source);
    animation.set_configuration(Some(GravityConnectedAnimationConfiguration::new()));
    assert!(animation.try_start(&scene.destination));
    scene.services.run_jobs();
    let host = layer(&scene).unwrap().children().get(0);
    // The shadow sprite, the source image and the destination image.
    assert_eq!(host.children().count(), 3);

    let no_shadow = service.prepare_to_animate("other", &scene.destination);
    no_shadow.set_configuration(Some(GravityConnectedAnimationConfiguration::new().with_is_shadow_enabled(false)));
    assert!(no_shadow.try_start(&scene.source));
}

#[test]
fn the_destination_cannot_be_the_source() {
    let scene = scene();
    let service = ConnectedAnimationService::get_for_top_level(scene.services.top_level());
    let animation = service.prepare_to_animate("item", &scene.source);
    assert!(!animation.try_start(&scene.source));
    scene.services.run_jobs();
    assert!(!scene.source.is_hidden_by_composition());
}

#[test]
fn a_destination_that_leaves_the_tree_cancels_the_flight() {
    let scene = scene();
    let service = ConnectedAnimationService::get_for_top_level(scene.services.top_level());
    let animation = service.prepare_to_animate("item", &scene.source);
    animation.set_configuration(Some(DirectConnectedAnimationConfiguration::new()));
    let completed = Rc::new(Cell::new(None));
    let flag = completed.clone();
    let _subscription = animation.completed(move |e| flag.set(Some(e.cancelled())));
    assert!(animation.try_start(&scene.destination));
    scene.canvas.children().remove(&scene.destination);
    scene.services.run_jobs();
    assert_eq!(completed.get(), Some(true));
    assert_eq!(layer(&scene).map_or(0, |layer| layer.children().count()), 0);
    assert!(!scene.source.is_hidden_by_composition());
}

#[test]
fn coordinated_elements_enter_with_the_animation() {
    let scene = scene();
    let coordinated = border(150.0, 20.0, 200.0, 180.0);
    scene.canvas.children().add(coordinated.clone());
    scene.services.run_jobs();
    let service = ConnectedAnimationService::get_for_top_level(scene.services.top_level());
    let animation = service.prepare_to_animate("item", &scene.source);
    animation.set_configuration(Some(BasicConnectedAnimationConfiguration::new()));
    let completed = Rc::new(Cell::new(false));
    let flag = completed.clone();
    let _subscription = animation.completed(move |_| flag.set(true));
    assert!(animation.try_start_with_coordinated_elements(&scene.destination, &[coordinated.clone().upcast()]));
    assert!(coordinated.is_hidden_by_composition());
    scene.services.run_jobs();
    scene.services.run_jobs();
    // The host of the animation and the host of the coordinated element.
    assert_eq!(layer(&scene).unwrap().children().count(), 2);
    run_until(&scene, || completed.get() && !coordinated.is_hidden_by_composition());
    assert!(!coordinated.is_hidden_by_composition());
}

#[test]
fn the_scale_animation_can_be_disabled() {
    let scene = scene();
    let service = ConnectedAnimationService::get_for_top_level(scene.services.top_level());
    let animation = service.prepare_to_animate("item", &scene.source);
    assert!(animation.is_scale_animation_enabled());
    animation.set_is_scale_animation_enabled(false);
    assert!(!animation.is_scale_animation_enabled());
    assert!(animation.try_start(&scene.destination));
    scene.services.run_jobs();
    let host = layer(&scene).unwrap().children().get(0);
    // The host takes the larger of the two sizes.
    assert_eq!(host.size(), Vector::new(150.0, 75.0));
}
