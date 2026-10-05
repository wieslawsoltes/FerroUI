//! Port of `CompositionBrushTests.cs`, and tests of this port for the
//! composition brushes (marked as such).

use super::*;
use crate::collections::FerroList;
use crate::media::{Colors, GradientSpreadMethod, GradientStop, IBrush, IGradientStop, MediaContext};
use crate::rendering::composition::drawing::brush_get_server_resource;
use crate::rendering::composition::server::ServerCompositionGradientStop;
use crate::rendering::composition::test_compositor::TestCompositor;
use crate::rendering::composition::transport::BatchResource;
use crate::rendering::composition::{Compositor, ICompositionObjectAnimations};
use crate::rendering::testing::ManualRenderLoop;
use crate::threading::Dispatcher;
use crate::Ref;
use std::rc::Rc;
use std::time::Duration;

#[test]
fn replacing_the_gradient_stop_list_after_a_commit_should_reach_the_server() {
    let services = TestCompositor::new();
    let compositor = &services.compositor;

    let brush = compositor.create_linear_gradient_brush();
    brush.gradient_stops().add(compositor.create_gradient_stop_with(0.0, Colors::RED));
    services.run_jobs();

    let server = services.server::<ServerCompositionLinearGradientBrush>(brush.server());
    assert_eq!(server.gradient_stops().len(), 1);

    brush.set_gradient_stops(FerroList::from_items([
        compositor.create_gradient_stop_with(0.0, Colors::RED) as Rc<dyn IGradientStop>,
        compositor.create_gradient_stop_with(1.0, Colors::BLUE),
    ]));
    services.run_jobs();

    assert_eq!(2, server.gradient_stops().len());
}

#[test]
fn changing_spread_method_after_a_commit_should_reach_the_server() {
    let services = TestCompositor::new();
    let compositor = &services.compositor;

    let brush = compositor.create_linear_gradient_brush();
    brush.gradient_stops().add(compositor.create_gradient_stop_with(0.0, Colors::RED));
    services.run_jobs();

    let server = services.server::<ServerCompositionLinearGradientBrush>(brush.server());
    assert_eq!(GradientSpreadMethod::Pad, server.spread_method());

    brush.set_spread_method(GradientSpreadMethod::Repeat);
    services.run_jobs();

    assert_eq!(GradientSpreadMethod::Repeat, server.spread_method());
}

#[test]
fn mutable_gradient_stops_should_be_snapshotted_for_the_server() {
    let services = TestCompositor::new();
    let compositor = &services.compositor;

    let mutable_stop: Ref<GradientStop> = GradientStop::with_color_and_offset(Colors::RED, 0.0);
    let mutable_stop: Rc<dyn IGradientStop> = mutable_stop.into();
    let brush = compositor.create_linear_gradient_brush();
    brush.gradient_stops().add(mutable_stop.clone());
    services.run_jobs();

    // The render thread reads the server list at replay time, so a mutable
    // UI-thread stop must not cross the batch by reference.
    let server = services.server::<ServerCompositionLinearGradientBrush>(brush.server());
    let stops = server.gradient_stops();
    assert_eq!(stops.len(), 1);
    let server_stop = &stops[0];
    assert!(!std::ptr::addr_eq(Rc::as_ptr(&mutable_stop), Rc::as_ptr(server_stop)));
    assert_eq!(Colors::RED, server_stop.color());
    assert_eq!(0.0, server_stop.offset());
}

#[test]
fn using_a_composition_brush_with_a_foreign_compositor_should_throw() {
    let services = TestCompositor::new();

    let brush = services.compositor.create_solid_color_brush_with(Colors::RED);

    let foreign = Compositor::with_scheduler(
        ManualRenderLoop::new(),
        None,
        true,
        &MediaContext::instance().scheduler(),
        Dispatcher::ui_thread(),
        None,
        None,
    );

    // A composition brush's server object belongs to its own compositor's
    // render loop; handing it to another compositor's stream would let two
    // render threads race over one resource.
    let as_brush = brush.as_brush();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        brush_get_server_resource(Some(&as_brush), Some(&foreign))
    }));
    assert!(result.is_err());

    // A transient context without a compositor keeps the client brush and
    // draws its static values, so no affinity applies there.
    match brush_get_server_resource(Some(&as_brush), None) {
        Some(BatchResource::Value(value)) => assert!(std::ptr::addr_eq(Rc::as_ptr(&value), Rc::as_ptr(&as_brush))),
        _ => panic!("the brush itself is expected"),
    }
}

// --- tests of this port ---------------------------------------------------------

/// Not from upstream: the properties of a composition brush reach its
/// server object, which is the brush drawn on its own compositor.
#[test]
fn composition_brush_properties_reach_the_server() {
    let services = TestCompositor::new();
    let compositor = &services.compositor;

    let solid = compositor.create_solid_color_brush_with(Colors::GREEN);
    solid.set_opacity(0.5);
    let radial = compositor.create_radial_gradient_brush();
    services.run_jobs();

    let server = services.server::<ServerCompositionSolidColorBrush>(solid.server());
    let server_brush: &dyn IBrush = &*server;
    assert_eq!(server_brush.as_solid_color_brush().map(|b| b.color()), Some(Colors::GREEN));
    assert_eq!(server_brush.opacity(), 0.5);
    assert_eq!(solid.as_brush().as_solid_color_brush().map(|b| b.color()), Some(Colors::GREEN));
    match brush_get_server_resource(Some(&solid.as_brush()), Some(compositor)) {
        Some(BatchResource::Server(id)) => assert_eq!(id, solid.server()),
        _ => panic!("the server object of the brush is expected"),
    }

    // The defaults of the schema.
    let radial_server = services.server::<ServerCompositionRadialGradientBrush>(radial.server());
    assert_eq!(radial_server.props().radius_x(), crate::RelativeScalar::MIDDLE);
    assert_eq!(radial.radius(), 0.5);
    assert_eq!(radial_server.brush_props().opacity(), 1.0);
}

/// Not from upstream: a composition gradient stop stays live on the server
/// and animates individually; the brush observes it.
#[test]
fn composition_gradient_stops_stay_live_and_animate() {
    let services = TestCompositor::new();
    let compositor = &services.compositor;

    let stop = compositor.create_gradient_stop_with(1.5, Colors::RED);
    // The offset is clamped.
    assert_eq!(stop.offset(), 1.0);
    let brush = compositor.create_conic_gradient_brush();
    brush.gradient_stops().add(stop.clone());
    services.run_jobs();

    let server_stop = services.server::<ServerCompositionGradientStop>(stop.server());
    let server = services.server::<ServerCompositionConicGradientBrush>(brush.server());
    let stops = server.gradient_stops();
    assert!(std::ptr::addr_eq(Rc::as_ptr(&stops[0]), Rc::as_ptr(&server_stop)));

    stop.set_color(Colors::BLUE);
    services.run_jobs();
    assert_eq!(server.gradient_stops()[0].color(), Colors::BLUE);

    // An animation of the offset runs on the server clock.
    let animation = compositor.create_double_key_frame_animation();
    animation.insert_key_frame(1.0, 0.25);
    animation.set_duration(Duration::from_millis(50));
    stop.start_animation("Offset", &*animation);
    services.run_jobs();
    let started = compositor.server().server_now();
    while compositor.server().server_now() < started + Duration::from_millis(150) {
        services.render_loop.tick();
    }
    assert_eq!(server_stop.offset(), 0.25);
    assert!(services.animated(stop.server()).server_object().animations().is_some());

    // The brush animates its own properties too.
    assert!(brush.get_composition_property("Opacity").is_some());
    assert!(brush.get_composition_property("Angle").is_some());
    assert!(brush.get_composition_property("Nope").is_none());
}
