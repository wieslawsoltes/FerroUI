//! Port of `CompositionAnimationTests.cs`, and tests of this port for the
//! animation classes (marked as such).
//!
//! The reference tests read and write the server object of a visual
//! directly (`target.Server`). Here server objects are created by the batch
//! that follows their UI-thread counterpart, so the tests commit first and
//! then look the server object up by id.

use super::*;
use crate::animation::easings::LinearEasing;
use crate::animation::PlaybackDirection;
use crate::media::Colors;
use crate::rendering::composition::expressions::{ExpressionVariant, VariantType};
use crate::rendering::composition::generated::ServerCompositionVisualProps;
use crate::rendering::composition::server::{IAnimatedServerObject, ServerCompositionVisual};
use crate::rendering::composition::test_compositor::TestCompositor;
use crate::rendering::composition::{
    AsCompositionObject, CompositionGetValueStatus, CompositionVisual, ICompositionObjectAnimations,
};
use crate::Vector3D;
use std::rc::Rc;
use std::time::Duration;

/// The compositor of the tests, with the lookups of server visuals.
struct Fixture(TestCompositor);

impl std::ops::Deref for Fixture {
    type Target = TestCompositor;

    fn deref(&self) -> &TestCompositor {
        &self.0
    }
}

impl Fixture {
    fn new() -> Fixture {
        Fixture(TestCompositor::new())
    }

    /// The server visual of `visual`, after a commit.
    fn server(&self, visual: &CompositionVisual) -> Rc<ServerCompositionVisual> {
        self.0.server::<ServerCompositionVisual>(visual.server())
    }

    fn animated(&self, visual: &CompositionVisual) -> Rc<dyn IAnimatedServerObject> {
        self.0.animated(visual.server())
    }
}

struct AnimationData {
    name: &'static str,
    frames: Vec<(f32, f32)>,
    checks: Vec<(f32, f32)>,
    starting_value: f32,
}

fn generate() -> Vec<AnimationData> {
    vec![
        AnimationData {
            name: "3 frames starting from 0",
            frames: vec![(0.0, 10.0), (0.5, 30.0), (1.0, 20.0)],
            checks: vec![(0.25, 20.0), (0.5, 30.0), (0.75, 25.0), (1.0, 20.0)],
            starting_value: 0.0,
        },
        AnimationData {
            name: "1 final frame",
            frames: vec![(1.0, 10.0)],
            checks: vec![(0.0, 0.0), (0.5, 5.0), (1.0, 10.0)],
            starting_value: 0.0,
        },
    ]
}

#[test]
fn generic_check() {
    let fixture = Fixture::new();
    let compositor = &fixture.compositor;
    for data in generate() {
        let target = compositor.create_solid_color_visual();
        fixture.run_jobs();
        let ani = ScalarKeyFrameAnimation::new(compositor);
        for (key, value) in &data.frames {
            ani.insert_key_frame_with_easing(*key, *value, Rc::new(LinearEasing::new()));
        }
        ani.set_duration(Duration::from_secs(1));
        let instance = ani.create_instance(target.server(), None);
        instance.resolve(compositor.server());
        instance.initialize(
            Duration::ZERO,
            data.starting_value.into(),
            ServerCompositionVisualProps::id_of_rotation_angle_property(),
        );
        let mut current_value = ExpressionVariant::create(data.starting_value);
        for (time, value) in &data.checks {
            current_value = instance.evaluate(Duration::from_secs_f32(*time), current_value);
            assert_eq!(current_value, ExpressionVariant::Double(*value as f64), "{} at {time}", data.name);
        }
    }
}

#[test]
fn get_composition_property_returns_registered_properties() {
    let fixture = Fixture::new();
    for prop_name in ["Color", "Offset"] {
        let target = fixture.compositor.create_solid_color_visual();
        fixture.run_jobs();

        let property = fixture.animated(&target).get_composition_property(prop_name);

        let property = property.expect("the property is registered");
        assert_eq!(prop_name, property.name());
        assert!(property.get_variant().is_some());
    }
}

#[test]
fn expression_animation_operations_works_correctly() {
    let fixture = Fixture::new();
    let compositor = &fixture.compositor;
    let target = compositor.create_solid_color_visual();
    fixture.run_jobs();
    let server = fixture.server(&target);
    server.visual_props().set_offset(&*server, Vector3D::new(100.0, 200.0, 0.0));

    let ani = compositor.create_expression_animation_with("this.Target.Offset.X * 0.5 + 10");
    let instance = ani.create_instance(target.server(), None);
    instance.resolve(compositor.server());
    instance.initialize(
        Duration::ZERO,
        ExpressionVariant::create(0f32),
        ServerCompositionVisualProps::id_of_rotation_angle_property(),
    );

    let result = instance.evaluate(Duration::ZERO, ExpressionVariant::create(0f32));

    assert_eq!(VariantType::Double, result.variant_type());
    assert_eq!(ExpressionVariant::Double(60.0), result);
}

#[test]
fn expression_animation_tracks_reference_parameter() {
    let fixture = Fixture::new();
    let compositor = &fixture.compositor;
    let target = compositor.create_solid_color_visual();
    let obj = compositor.create_solid_color_visual();
    fixture.run_jobs();
    let obj_server = fixture.server(&obj);
    obj_server.visual_props().set_offset(&*obj_server, Vector3D::new(100.0, 200.0, 0.0));

    let ani = compositor.create_expression_animation_with("obj.Offset.X * 0.5 + 10");
    ani.set_reference_parameter("obj", (*obj).clone());
    let instance = ani.create_instance(target.server(), None);

    let target_server = fixture.server(&target);
    target_server.activate();

    // Invoke OnSetAnimatedValue manually to create ServerObjectAnimationInstance.
    let animations = target_server.server_object().get_or_create_animations();
    animations.on_set_animated_value(
        ServerCompositionVisualProps::id_of_rotation_angle_property(),
        ExpressionVariant::create(0f32),
        Duration::ZERO,
        instance.clone(),
    );

    let initial_result = instance.evaluate(Duration::ZERO, ExpressionVariant::create(0f32));
    assert_eq!(ExpressionVariant::Double(60.0), initial_result);

    obj_server.visual_props().set_offset(&*obj_server, Vector3D::new(200.0, 300.0, 0.0));
    let updated_result = instance.evaluate(Duration::ZERO, ExpressionVariant::create(0f32));
    assert_eq!(ExpressionVariant::Double(110.0), updated_result);
}

#[test]
fn expression_animation_tracks_target() {
    let fixture = Fixture::new();
    let compositor = &fixture.compositor;
    let target = compositor.create_solid_color_visual();
    fixture.run_jobs();
    let target_server = fixture.server(&target);

    target_server.visual_props().set_offset(&*target_server, Vector3D::new(100.0, 200.0, 0.0));

    let ani = compositor.create_expression_animation_with("this.Target.Offset.X * 0.5 + 10");
    let instance = ani.create_instance(target.server(), None);

    target_server.activate();

    // Invoke OnSetAnimatedValue manually to create ServerObjectAnimationInstance.
    let animations = target_server.server_object().get_or_create_animations();
    animations.on_set_animated_value(
        ServerCompositionVisualProps::id_of_rotation_angle_property(),
        ExpressionVariant::create(0f32),
        Duration::ZERO,
        instance.clone(),
    );

    let initial_result = instance.evaluate(Duration::ZERO, ExpressionVariant::create(0f32));
    assert_eq!(ExpressionVariant::Double(60.0), initial_result);

    target_server.visual_props().set_offset(&*target_server, Vector3D::new(200.0, 300.0, 0.0));
    let updated_result = instance.evaluate(Duration::ZERO, ExpressionVariant::create(0f32));
    assert_eq!(ExpressionVariant::Double(110.0), updated_result);
}

// --- tests of this port ---------------------------------------------------------

/// Not from upstream: a key frame animation started on a visual runs on
/// the server clock and leaves the clock when it is finished.
#[test]
fn started_key_frame_animation_runs_on_the_server_clock() {
    let fixture = Fixture::new();
    let compositor = &fixture.compositor;
    let visual = compositor.create_solid_color_visual();
    fixture.run_jobs();
    let server = fixture.server(&visual);
    server.activate();

    let ani = compositor.create_scalar_key_frame_animation();
    ani.insert_key_frame_with_easing(1.0, 0.5, Rc::new(LinearEasing::new()));
    ani.set_duration(Duration::from_millis(100));
    visual.start_animation("Opacity", &*ani);
    assert_eq!(visual.object().pending_animations().count(), 1);
    fixture.run_jobs();
    assert_eq!(visual.object().pending_animations().count(), 0);
    assert!(compositor.server().animations().need_next_tick());

    // The animation is evaluated against the server clock: render frames
    // until it is over.
    let started = compositor.server().server_now();
    while compositor.server().server_now() < started + Duration::from_millis(250) {
        fixture.render_loop.tick();
    }
    assert_eq!(server.opacity(), 0.5);
    assert!(!compositor.server().animations().need_next_tick());
}

/// Not from upstream: the timing of key frame animations (delay, iteration
/// count, direction).
#[test]
fn key_frame_animation_timing() {
    let fixture = Fixture::new();
    let compositor = &fixture.compositor;
    let target = compositor.create_solid_color_visual();
    fixture.run_jobs();
    let property = ServerCompositionVisualProps::id_of_rotation_angle_property();
    let evaluate = |ani: &Rc<ScalarKeyFrameAnimation>, times: &[f32]| -> Vec<f64> {
        let instance = ani.create_instance(target.server(), None);
        instance.resolve(compositor.server());
        instance.initialize(Duration::ZERO, ExpressionVariant::create(0f32), property);
        times
            .iter()
            .map(|t| match instance.evaluate(Duration::from_secs_f32(*t), ExpressionVariant::create(-1f32)) {
                ExpressionVariant::Double(v) => (v * 1000.0).round() / 1000.0,
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    };

    let ani = compositor.create_scalar_key_frame_animation();
    ani.insert_key_frame_with_easing(0.0, 0.0, Rc::new(LinearEasing::new()));
    ani.insert_key_frame_with_easing(1.0, 10.0, Rc::new(LinearEasing::new()));
    ani.set_duration(Duration::from_secs(1));
    ani.set_delay_time(Duration::from_secs(1));
    // Before the delay has elapsed the current value is kept.
    assert_eq!(evaluate(&ani, &[0.5, 1.5, 2.5]), [-1.0, 5.0, 10.0]);
    ani.set_delay_behavior(AnimationDelayBehavior::SetInitialValueBeforeDelay);
    assert_eq!(evaluate(&ani, &[0.5]), [0.0]);

    ani.set_delay_time(Duration::ZERO);
    ani.set_iteration_count(3);
    ani.set_direction(PlaybackDirection::Alternate);
    assert_eq!(evaluate(&ani, &[0.25, 1.25, 2.25, 3.5]), [2.5, 7.5, 2.5, 10.0]);
    ani.set_direction(PlaybackDirection::Reverse);
    assert_eq!(evaluate(&ani, &[0.25, 1.25]), [7.5, 7.5]);
    ani.set_iteration_behavior(AnimationIterationBehavior::Forever);
    ani.set_direction(PlaybackDirection::AlternateReverse);
    assert_eq!(evaluate(&ani, &[0.25, 1.25, 10.25]), [7.5, 2.5, 7.5]);
}

/// Not from upstream: an implicit animation collection starts its
/// animations when the property changes, with `this.FinalValue` bound to
/// the new value, and animation groups start every animation of the group.
#[test]
fn implicit_animations_and_groups() {
    let fixture = Fixture::new();
    let compositor = &fixture.compositor;
    let visual = compositor.create_solid_color_visual();
    fixture.run_jobs();

    let offset = compositor.create_vector3d_key_frame_animation();
    offset.set_target(Some("Offset".to_owned()));
    offset.insert_expression_key_frame(1.0, "this.FinalValue", None);
    offset.set_duration(Duration::from_millis(100));
    let opacity = compositor.create_scalar_key_frame_animation();
    opacity.set_target(Some("Opacity".to_owned()));
    opacity.insert_key_frame(1.0, 0.25);
    let group = compositor.create_animation_group();
    group.add(offset.clone());
    group.add(opacity.clone());

    let implicit = compositor.create_implicit_animation_collection();
    implicit.set("Offset", group.clone());
    assert_eq!(implicit.count(), 1);
    assert!(implicit.has_key("Offset"));
    visual.set_implicit_animations(Some(implicit.clone()));

    visual.set_offset(Vector3D::new(10.0, 20.0, 0.0));
    // The UI-thread value is the new value; both animations of the group
    // are pending.
    assert_eq!(visual.offset(), Vector3D::new(10.0, 20.0, 0.0));
    assert_eq!(visual.object().pending_animations().count(), 2);

    // Stopping the group drops the animations on the server.
    fixture.run_jobs();
    let server = fixture.server(&visual);
    assert!(server.server_object().animations().is_some());
    visual.stop_animation_group(&*group);
    fixture.run_jobs();

    implicit.remove("Offset");
    visual.set_offset(Vector3D::new(1.0, 2.0, 0.0));
    assert_eq!(visual.object().pending_animations().count(), 0);
    fixture.run_jobs();
    assert_eq!(server.offset(), Vector3D::new(1.0, 2.0, 0.0));
}

/// Not from upstream: the values and objects of a property set, and the
/// snapshot an animation takes of its parameters.
#[test]
fn property_set_values_and_snapshot() {
    let fixture = Fixture::new();
    let compositor = &fixture.compositor;
    let set = crate::rendering::composition::CompositionPropertySet::new(compositor);
    set.insert_scalar("a", 2.0);
    set.insert_color("c", Colors::RED);
    assert_eq!(set.try_get_scalar("a"), (CompositionGetValueStatus::Succeeded, 2.0));
    assert_eq!(set.try_get_color("c"), (CompositionGetValueStatus::Succeeded, Colors::RED));
    assert_eq!(set.try_get_boolean("a"), (CompositionGetValueStatus::TypeMismatch, false));
    assert_eq!(set.try_get_scalar("x"), (CompositionGetValueStatus::NotFound, 0.0));

    let visual = compositor.create_solid_color_visual();
    set.set_object("v", (*visual).clone());
    assert_eq!(set.try_get_scalar("v").0, CompositionGetValueStatus::TypeMismatch);

    let snapshot = set.snapshot();
    assert_eq!(snapshot.get_parameter("a"), ExpressionVariant::Double(2.0));
    // The object is known by id until the snapshot is resolved.
    assert!(snapshot.get_object_parameter("v").is_none());
    fixture.run_jobs();
    snapshot.resolve(compositor.server());
    assert!(snapshot.get_object_parameter("v").is_some());

    // An object without a server counterpart cannot be a parameter.
    let animation = compositor.create_expression_animation();
    set.set_object("animation", animation.clone());
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| set.snapshot()));
    assert!(result.is_err());
    assert_eq!(animation.composition_type_name(), "ExpressionAnimation");
}

/// Not from upstream: two visuals whose animations refer to each other
/// through reference parameters are released once their composition
/// objects are dropped, and a parameter whose object is gone reads as a
/// parameter without an object.
#[test]
fn mutually_referencing_animations_do_not_keep_their_objects_alive() {
    let fixture = Fixture::new();
    let compositor = &fixture.compositor;
    let a = compositor.create_solid_color_visual();
    let b = compositor.create_solid_color_visual();
    a.set_offset(Vector3D::new(1.0, 0.0, 0.0));
    b.set_offset(Vector3D::new(2.0, 0.0, 0.0));

    let ani_a = compositor.create_expression_animation_with("other.Offset.X");
    ani_a.set_reference_parameter("other", (*b).clone());
    a.start_animation("RotationAngle", &*ani_a);
    let ani_b = compositor.create_expression_animation_with("other.Offset.X");
    ani_b.set_reference_parameter("other", (*a).clone());
    b.start_animation("RotationAngle", &*ani_b);
    fixture.run_jobs();

    let server_a = Rc::downgrade(&fixture.server(&a));
    let server_b = Rc::downgrade(&fixture.server(&b));
    let instance = ani_a.create_instance(a.server(), None);
    instance.resolve(compositor.server());
    instance.initialize(
        Duration::ZERO,
        ExpressionVariant::create(0f32),
        ServerCompositionVisualProps::id_of_rotation_angle_property(),
    );
    assert_eq!(
        instance.evaluate(Duration::ZERO, ExpressionVariant::create(0f32)),
        ExpressionVariant::Double(2.0)
    );

    drop((a, b, ani_a, ani_b));
    // The release of the dropped objects rides on a later batch.
    let _other = compositor.create_solid_color_visual();
    fixture.run_jobs();

    assert!(server_a.upgrade().is_none());
    assert!(server_b.upgrade().is_none());
    assert_eq!(
        instance.evaluate(Duration::ZERO, ExpressionVariant::create(0f32)),
        ExpressionVariant::default()
    );
}
