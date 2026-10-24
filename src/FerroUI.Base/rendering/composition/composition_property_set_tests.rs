//! Tests of this port for the live property sets (`docs/porting/winui-composition.md`): a
//! property set made by `CompositionPropertySet::new` has a server counterpart, its entries can
//! be animated, and an expression that refers to it reads its current values.

use super::expressions::ExpressionVariant;
use super::server::ServerCompositionPropertySet;
use super::test_compositor::TestCompositor;
use super::server::ServerObjectId;
use super::{AsCompositionObject, CompositionGetValueStatus, CompositionPropertySet, ICompositionObjectAnimations};
use crate::numerics::Vector3;

fn id(set: &CompositionPropertySet) -> ServerObjectId {
    set.as_composition_object().server().expect("a live property set has a server object")
}

#[test]
fn entries_reach_the_server() {
    let fixture = TestCompositor::new();
    let set = CompositionPropertySet::new(&fixture.compositor);
    assert!(set.is_live());
    set.insert_scalar("Weight", 0.25);
    set.insert_vector3("Offset", Vector3::new(1.0, 2.0, 3.0));
    fixture.run_jobs();

    let server = fixture.server::<ServerCompositionPropertySet>(id(&set));
    assert_eq!(server.get_value("Weight"), ExpressionVariant::create(0.25f32));
    assert_eq!(server.get_value("Offset"), ExpressionVariant::create(Vector3::new(1.0, 2.0, 3.0)));
    assert_eq!(server.get_value("Missing"), ExpressionVariant::default());

    // A later value replaces the earlier one.
    set.insert_scalar("Weight", 0.75);
    fixture.run_jobs();
    assert_eq!(server.get_value("Weight"), ExpressionVariant::create(0.75f32));
    assert_eq!(set.try_get_scalar("Weight"), (CompositionGetValueStatus::Succeeded, 0.75));
}

#[test]
fn an_expression_reads_the_current_values_of_a_referenced_set() {
    let fixture = TestCompositor::new();
    let compositor = &fixture.compositor;
    let set = CompositionPropertySet::new(compositor);
    set.insert_vector3("Pan", Vector3::new(10.0, 20.0, 0.0));
    // The target is another live set: a visual animates only while it is in a composition
    // target, and a property set is always active.
    let target = CompositionPropertySet::new(compositor);
    target.insert_vector3("Offset", Vector3::new(0.0, 0.0, 0.0));

    let animation = compositor.create_expression_animation_with("Vector3(ps.Pan.X, ps.Pan.Y * 2, 0)");
    animation.set_reference_parameter("ps", set.clone());
    target.start_animation("Offset", &*animation);
    fixture.run_jobs();
    fixture.run_jobs();
    let server = fixture.server::<ServerCompositionPropertySet>(id(&target));
    assert_eq!(server.get_value("Offset").cast_or_default::<Vector3>(), Vector3::new(10.0, 40.0, 0.0));

    // Not a copy taken at the start: a later value of the set reaches the expression.
    set.insert_vector3("Pan", Vector3::new(-5.0, 1.0, 0.0));
    fixture.run_jobs();
    fixture.run_jobs();
    assert_eq!(server.get_value("Offset").cast_or_default::<Vector3>(), Vector3::new(-5.0, 2.0, 0.0));
}

#[test]
fn an_entry_can_be_animated() {
    let fixture = TestCompositor::new();
    let compositor = &fixture.compositor;
    let set = CompositionPropertySet::new(compositor);
    set.insert_scalar("Value", 1.0);
    set.insert_scalar("Factor", 3.0);
    let animation = compositor.create_expression_animation_with("this.Target.Factor * 2");
    set.start_animation("Value", &*animation);
    fixture.run_jobs();
    fixture.run_jobs();

    let server = fixture.server::<ServerCompositionPropertySet>(id(&set));
    assert_eq!(server.get_value("Value"), ExpressionVariant::create(6.0f32));

    // A value set directly ends the animation.
    set.insert_scalar("Value", 4.0);
    fixture.run_jobs();
    assert_eq!(server.get_value("Value"), ExpressionVariant::create(4.0f32));
}

#[test]
fn an_entry_must_exist_to_be_animated() {
    let fixture = TestCompositor::new();
    let compositor = &fixture.compositor;
    let set = CompositionPropertySet::new(compositor);
    let animation = compositor.create_expression_animation_with("1");
    assert!(!set.try_start_animation("Value", &*animation, None));
}

#[test]
fn the_parameters_of_an_animation_are_copied() {
    let fixture = TestCompositor::new();
    let compositor = &fixture.compositor;
    let parameters = CompositionPropertySet::new_parameters(compositor);
    assert!(!parameters.is_live());
    parameters.insert_scalar("a", 1.0);
    assert_eq!(parameters.snapshot().get_parameter("a"), ExpressionVariant::create(1.0f32));
}

#[test]
fn sub_channels_of_an_entry_started_together_run_together() {
    let fixture = TestCompositor::new();
    let compositor = &fixture.compositor;
    let set = CompositionPropertySet::new(compositor);
    set.insert_vector3("Offset", Vector3::new(1.0, 2.0, 3.0));
    let x = compositor.create_expression_animation_with("10");
    let y = compositor.create_expression_animation_with("20");
    set.start_animation("Offset.X", &*x);
    set.start_animation("Offset.Y", &*y);
    fixture.run_jobs();
    fixture.run_jobs();
    let server = fixture.server::<ServerCompositionPropertySet>(id(&set));
    assert_eq!(server.get_value("Offset").cast_or_default::<Vector3>(), Vector3::new(10.0, 20.0, 3.0));
}
