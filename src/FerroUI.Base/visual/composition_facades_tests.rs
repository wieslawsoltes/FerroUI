//! Tests of the composition facades of WinUI's `UIElement` on a visual (`Rotation`,
//! `RotationAxis`, `Scale`, `CenterPoint`, `TransformMatrix`) and of their transitions.
//!
//! The renderer synchronizes a visual with its composition visual when the visual is rendered
//! again; here the tests attach the visual to a test compositor and call the synchronization
//! directly, as the renderer's update does.

use super::composition_facades::partial_start_expression_for_tests;
use crate::numerics::{Matrix4x4, Quaternion, Vector3};
use crate::rendering::composition::animations::{IAnimationInstance, ICompositionAnimation, ICompositionAnimationBase};
use crate::rendering::composition::expressions::ExpressionVariant;
use crate::rendering::composition::generated::ServerCompositionVisualProps;
use crate::rendering::composition::test_compositor::TestCompositor;
use crate::{Matrix, Rect, Ref, ScalarTransition, Vector3D, Vector3Transition, Vector3TransitionComponents, Visual};
use std::rc::Rc;
use std::time::Duration;

/// A visual with bounds, not yet attached to a compositor.
fn new_visual() -> Ref<Visual> {
    let visual = Visual::new();
    visual.set_bounds(Rect::new(10.0, 20.0, 100.0, 50.0));
    visual
}

fn implicit_keys(visual: &Visual) -> Vec<String> {
    let comp = visual.composition_visual().expect("the visual is attached");
    comp.implicit_animations().map(|animations| animations.keys()).unwrap_or_default()
}

fn pending_animations(visual: &Visual) -> usize {
    let comp = visual.composition_visual().expect("the visual is attached");
    comp.object().pending_animations().count()
}

#[test]
fn facade_defaults() {
    let visual = new_visual();
    assert_eq!(visual.rotation(), 0.0);
    assert_eq!(visual.rotation_axis(), Vector3::UNIT_Z);
    assert_eq!(visual.scale(), Vector3::ONE);
    assert_eq!(visual.center_point(), Vector3::ZERO);
    assert_eq!(visual.transform_matrix(), Matrix4x4::IDENTITY);
    assert_eq!(visual.opacity_transition(), None);
    assert_eq!(visual.rotation_transition(), None);
    assert_eq!(visual.scale_transition(), None);
    assert_eq!(visual.translation_transition(), None);

    assert_eq!(ScalarTransition::default().duration(), Duration::from_millis(300));
    assert_eq!(Vector3Transition::default().duration(), Duration::from_millis(300));
    assert_eq!(Vector3Transition::default().components(), Vector3TransitionComponents::all());
}

#[test]
fn facades_are_written_to_the_composition_visual_when_rendered() {
    let services = TestCompositor::new();
    let visual = new_visual();
    let comp = visual.attach_to_compositor(&services.compositor);

    visual.set_rotation(90.0);
    visual.set_scale(Vector3::new(2.0, 3.0, 1.0));
    visual.set_center_point(Vector3::new(5.0, 6.0, 0.0));
    visual.synchronize_composition_properties();

    assert!((comp.rotation_angle() - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    assert_eq!(comp.orientation(), Quaternion::IDENTITY);
    assert_eq!(comp.scale(), Vector3D::new(2.0, 3.0, 1.0));
    assert_eq!(comp.center_point(), Vector3D::new(5.0, 6.0, 0.0));

    // Around another axis the rotation is the orientation of the composition visual.
    visual.set_rotation_axis(Vector3::UNIT_X);
    visual.synchronize_composition_properties();
    assert_eq!(comp.rotation_angle(), 0.0);
    assert_eq!(comp.orientation(), Quaternion::create_from_axis_angle(Vector3::UNIT_X, 90f32.to_radians()));
}

#[test]
fn facades_never_set_leave_the_composition_properties_to_the_application() {
    let services = TestCompositor::new();
    let visual = new_visual();
    let comp = visual.attach_to_compositor(&services.compositor);

    comp.set_scale(Vector3D::new(4.0, 4.0, 1.0));
    comp.set_rotation_angle(1.0);
    comp.set_center_point(Vector3D::new(7.0, 8.0, 0.0));
    comp.set_translation(Vector3D::new(1.0, 2.0, 0.0));
    visual.synchronize_composition_properties();

    assert_eq!(comp.scale(), Vector3D::new(4.0, 4.0, 1.0));
    assert_eq!(comp.rotation_angle(), 1.0);
    assert_eq!(comp.center_point(), Vector3D::new(7.0, 8.0, 0.0));
    assert_eq!(comp.translation(), Vector3D::new(1.0, 2.0, 0.0));
    assert_eq!(comp.transform_matrix(), Matrix::IDENTITY);
}

#[test]
fn transform_matrix_takes_the_2d_affine_part() {
    let services = TestCompositor::new();
    let visual = new_visual();
    let comp = visual.attach_to_compositor(&services.compositor);

    let mut matrix = Matrix4x4::IDENTITY;
    matrix.m11 = 2.0;
    matrix.m12 = 0.5;
    matrix.m33 = 3.0;
    matrix.m41 = 7.0;
    matrix.m42 = 8.0;
    matrix.m43 = 9.0;
    visual.set_transform_matrix(matrix);
    assert_eq!(visual.transform_matrix(), matrix);
    visual.synchronize_composition_properties();

    assert_eq!(comp.transform_matrix(), Matrix::new(2.0, 0.5, 0.0, 1.0, 7.0, 8.0));
}

#[test]
fn transitions_install_implicit_animations_after_the_first_render() {
    let services = TestCompositor::new();
    let visual = new_visual();
    visual.set_opacity_transition(Some(ScalarTransition::new()));
    visual.set_rotation_transition(Some(ScalarTransition::with_duration(Duration::from_millis(100))));
    visual.set_scale_transition(Some(Vector3Transition::new()));
    // A transition of no component animates nothing.
    visual.set_translation_transition(Some(Vector3Transition::with(
        Duration::from_millis(100),
        Vector3TransitionComponents::empty(),
    )));
    visual.set_opacity(0.5);
    visual.attach_to_compositor(&services.compositor);

    // The first render writes the initial values without animating them.
    visual.synchronize_composition_properties();
    assert_eq!(pending_animations(&visual), 0);
    assert_eq!(implicit_keys(&visual), ["Opacity", "RotationAngle", "Orientation", "Scale"]);

    // A later change animates.
    visual.set_opacity(0.25);
    visual.synchronize_composition_properties();
    assert_eq!(pending_animations(&visual), 1);
}

#[test]
fn a_transition_set_on_a_rendered_element_animates_the_change_made_with_it() {
    let services = TestCompositor::new();
    let visual = new_visual();
    visual.attach_to_compositor(&services.compositor);
    visual.synchronize_composition_properties();
    assert!(implicit_keys(&visual).is_empty());

    visual.set_opacity_transition(Some(ScalarTransition::new()));
    visual.set_opacity(0.5);
    visual.synchronize_composition_properties();
    assert_eq!(implicit_keys(&visual), ["Opacity"]);
    assert_eq!(pending_animations(&visual), 1);
}

#[test]
fn transitions_replace_the_implicit_animations_of_the_application() {
    let services = TestCompositor::new();
    let visual = new_visual();
    let comp = visual.attach_to_compositor(&services.compositor);
    let own = services.compositor.create_implicit_animation_collection();
    let offset = services.compositor.create_vector3d_key_frame_animation();
    offset.insert_expression_key_frame(1.0, "this.FinalValue", None);
    // An implicit animation names the property it animates.
    offset.set_target(Some("Offset".to_owned()));
    own.set("Offset", offset);
    comp.set_implicit_animations(Some(own.clone()));

    // Without transitions the application's animations stay.
    visual.synchronize_composition_properties();
    assert!(comp.implicit_animations().is_some_and(|current| Rc::ptr_eq(&current, &own)));

    visual.set_scale_transition(Some(Vector3Transition::new()));
    visual.synchronize_composition_properties();
    assert_eq!(implicit_keys(&visual), ["Scale"]);

    // Removing the last transition takes the facades' animations off the visual.
    visual.set_scale_transition(None);
    assert!(comp.implicit_animations().is_none());
    visual.synchronize_composition_properties();
    assert!(comp.implicit_animations().is_none());
}

#[test]
fn a_vector3_transition_of_some_components_animates_only_those() {
    assert_eq!(
        partial_start_expression_for_tests(Vector3TransitionComponents::X),
        "Vector3(this.StartingValue.X, this.FinalValue.Y, this.FinalValue.Z)"
    );

    let services = TestCompositor::new();
    let visual = new_visual();
    visual.set_translation_transition(Some(Vector3Transition::with(
        Duration::from_millis(300),
        Vector3TransitionComponents::X,
    )));
    let comp = visual.attach_to_compositor(&services.compositor);
    visual.synchronize_composition_properties();
    services.run_jobs();

    let animations = comp.implicit_animations().expect("the transition installs implicit animations");
    let translation = animations.get("Translation");
    let animation = ICompositionAnimationBase::as_composition_animation(&*translation).expect("a composition animation");
    let final_value = Vector3D::new(100.0, 50.0, 0.0);
    let instance = ICompositionAnimation::create_instance(animation, comp.server(), Some(ExpressionVariant::create(final_value)))
        .create();
    IAnimationInstance::resolve(&*instance, services.compositor.server());
    IAnimationInstance::initialize(
        &*instance,
        Duration::ZERO,
        ExpressionVariant::create(Vector3D::default()),
        ServerCompositionVisualProps::id_of_translation_property(),
    );

    let current = ExpressionVariant::create(Vector3D::default());
    let halfway = IAnimationInstance::evaluate(&*instance, Duration::from_millis(150), current)
        .cast_or_default::<Vector3D>();
    assert!(halfway.x > 0.0 && halfway.x < 100.0, "X animates: {halfway:?}");
    assert_eq!(halfway.y, 50.0, "Y takes its final value at once");
    assert_eq!(halfway.z, 0.0);

    let end = IAnimationInstance::evaluate(&*instance, Duration::from_millis(300), current).cast_or_default::<Vector3D>();
    assert_eq!(end, final_value);
}
