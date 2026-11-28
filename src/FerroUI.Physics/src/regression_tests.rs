//! Regression coverage for world lifecycle, collision events, query freshness,
//! and frame-rate independent UI motion.

use crate::prelude::*;

fn one_step_world() -> PhysicsWorld {
    let mut world = PhysicsWorld::new(Vec2::ZERO);
    world.config.substeps = 1;
    world
}

#[test]
fn default_bvh_is_empty_and_usable() {
    let mut tree = DynamicBvh::default();
    let bounds = Aabb2::from_center_half_extents(Vec2::ZERO, Vec2::splat(1.0));
    tree.query_aabb(bounds, |_| panic!("empty tree has no leaves"));
    tree.query_ray(Ray2::new(Vec2::ZERO, Vec2::X), 10.0, |_, _| {
        panic!("empty tree has no leaves")
    });
    let proxy = tree.insert(7, bounds);
    let mut hits = Vec::new();
    tree.query_aabb(bounds, |id| hits.push(id));
    assert_eq!(hits, [7]);
    tree.remove(proxy);
    tree.query_aabb(bounds, |_| panic!("removed leaf is gone"));
}

#[test]
fn clear_never_resurrects_body_or_joint_handles() {
    let mut world = one_step_world();
    let old = world.add_body(BodyBuilder::dynamic(Shape2D::circle(1.0)));
    let old_joint = world.add_joint(Joint::target(old, Vec2::ZERO, Vec2::ZERO, 10.0));
    world.step(0.01);
    world.clear();
    let new = world.add_body(BodyBuilder::dynamic(Shape2D::circle(1.0)));
    let new_joint = world.add_joint(Joint::target(new, Vec2::ZERO, Vec2::ZERO, 10.0));
    assert_eq!(old.index(), new.index());
    assert_ne!(old.generation(), new.generation());
    assert_ne!(old_joint, new_joint);
    assert!(world.body(old).is_none());
    assert!(world.joint(old_joint).is_none());
    assert!(!world.remove_body(old));
    assert!(!world.remove_joint(old_joint));
    assert_eq!(world.last_stats().total_bodies, 0);
}

#[test]
fn removing_body_reclaims_attached_joints() {
    let mut world = one_step_world();
    let body = world.add_body(BodyBuilder::dynamic(Shape2D::circle(1.0)));
    let joint = world.add_joint(Joint::target(body, Vec2::ZERO, Vec2::ZERO, 10.0));
    assert!(world.remove_body(body));
    assert!(world.joint(joint).is_none());
    assert_eq!(world.all_joints().count(), 0);
}

#[test]
fn stale_joint_endpoint_cannot_move_reused_body_slot() {
    let mut world = one_step_world();
    let stale = world.add_body(BodyBuilder::dynamic(Shape2D::circle(0.2)));
    world.remove_body(stale);
    let replacement = world.add_body(BodyBuilder::dynamic(Shape2D::circle(0.2)));
    world.add_joint(Joint::target(
        stale,
        Vec2::ZERO,
        Vec2::new(20.0, 0.0),
        1000.0,
    ));
    world.step(0.1);
    assert_eq!(world.body(replacement).unwrap().position, Vec2::ZERO);
}

#[test]
fn queries_observe_static_teleports_immediately() {
    let mut world = one_step_world();
    let body = world.add_body(BodyBuilder::static_body(Shape2D::rect(2.0, 2.0)));
    world
        .body_mut(body)
        .unwrap()
        .set_position(Vec2::new(20.0, 0.0));
    let hit = world
        .raycast(
            Ray2::new(Vec2::new(15.0, 0.0), Vec2::X),
            10.0,
            CollisionFilter::ALL,
        )
        .unwrap();
    assert_eq!(hit.body, body);
    assert!((hit.toi - 4.0).abs() < 1e-5);
    assert_eq!(
        world.point_query(Vec2::new(20.0, 0.0), CollisionFilter::ALL),
        [body]
    );
    assert!(world
        .point_query(Vec2::ZERO, CollisionFilter::ALL)
        .is_empty());
    world.step(0.01);
    assert_eq!(world.pick_body(Vec2::new(20.0, 0.0), false), Some(body));
}

#[test]
fn queries_observe_last_integrated_substep() {
    let mut world = one_step_world();
    let body = world.add_body(
        BodyBuilder::dynamic(Shape2D::circle(0.2))
            .linear_velocity((100.0, 0.0))
            .linear_damping(0.0)
            .can_sleep(false),
    );
    world.step(1.0);
    let p = world.body(body).unwrap().position;
    assert_eq!(p, Vec2::new(100.0, 0.0));
    assert_eq!(world.pick_body(p, true), Some(body));
    assert!(world.body(body).unwrap().aabb.contains_point(p));
}

#[test]
fn restoring_sleeping_snapshot_refreshes_queries() {
    let mut world = one_step_world();
    let body = world.add_body(BodyBuilder::dynamic(Shape2D::circle(0.5)));
    world.body_mut(body).unwrap().put_to_sleep();
    let snapshot = WorldSnapshot::capture(&world, 0);
    world
        .body_mut(body)
        .unwrap()
        .set_position(Vec2::new(100.0, 0.0));
    world.step(0.1);
    snapshot.restore(&mut world);
    assert!(world.body(body).unwrap().is_sleeping);
    assert_eq!(world.pick_body(Vec2::ZERO, true), Some(body));
    assert_eq!(world.pick_body(Vec2::new(100.0, 0.0), true), None);
    assert!(world.collision_events().is_empty());
}

#[test]
fn snapshot_restores_fractured_topology_joint_handles_and_pending_forces() {
    let mut world = one_step_world();
    let anchor = world.add_body(BodyBuilder::static_body(Shape2D::circle(0.1)));
    let body = world.add_body(BodyBuilder::dynamic(Shape2D::rect(2.0, 2.0)).position((3.0, 0.0)));
    let joint = world.add_joint(Joint::distance(anchor, body, Vec2::ZERO, Vec2::ZERO, 3.0));
    world
        .body_mut(body)
        .unwrap()
        .apply_force(Vec2::new(0.0, 10.0));
    let snapshot = WorldSnapshot::capture(&world, 10);
    let shards = fracture_body(&mut world, body, Vec2::new(3.0, 0.0), 2.0);
    assert!(!shards.is_empty());
    assert!(world.body(body).is_none());
    world.gravity = Vec2::new(100.0, 0.0);
    snapshot.restore(&mut world);
    assert_eq!(world.body_count(), 2);
    assert!(world.joint(joint).is_some());
    assert_eq!(world.body(body).unwrap().force, Vec2::new(0.0, 10.0));
    assert_eq!(world.gravity, Vec2::ZERO);
    assert!(shards.iter().all(|handle| world.body(*handle).is_none()));
}

#[test]
fn snapshot_replay_preserves_warm_start_state_and_results() {
    let mut world = one_step_world();
    world.gravity = Vec2::new(0.0, 9.81);
    world.add_body(BodyBuilder::static_body(Shape2D::rect(20.0, 1.0)).position((0.0, 5.0)));
    let body = world.add_body(
        BodyBuilder::dynamic(Shape2D::rect(1.0, 1.0))
            .position((0.0, 4.0))
            .can_sleep(false),
    );
    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }
    let snapshot = WorldSnapshot::capture(&world, 60);
    for _ in 0..30 {
        world.step(1.0 / 60.0);
    }
    let expected = world.body(body).unwrap().clone();
    snapshot.restore(&mut world);
    for _ in 0..30 {
        world.step(1.0 / 60.0);
    }
    let actual = world.body(body).unwrap();
    assert_eq!(actual.position, expected.position);
    assert_eq!(actual.linear_velocity, expected.linear_velocity);
    assert_eq!(actual.angle, expected.angle);
    assert_eq!(actual.angular_velocity, expected.angular_velocity);
}

#[test]
fn history_honors_requested_capacity_and_truncates_future() {
    let mut world = one_step_world();
    let body = world.add_body(BodyBuilder::dynamic(Shape2D::circle(0.5)));
    let mut history = WorldHistory::new(2);
    for i in 0..4 {
        world
            .body_mut(body)
            .unwrap()
            .set_position(Vec2::new(i as f32, 0.0));
        history.record(&world);
    }
    assert_eq!(history.len(), 2);
    assert!(history.step_back(&mut world));
    assert_eq!(world.body(body).unwrap().position.x, 2.0);
    world
        .body_mut(body)
        .unwrap()
        .set_position(Vec2::new(9.0, 0.0));
    history.record(&world);
    assert!(!history.step_forward(&mut world));
    assert_eq!(history.len(), 2);
}

#[test]
fn capsule_and_offset_segment_inertia_include_translation_terms() {
    let capsule = Shape2D::capsule(2.0, 1.0).compute_mass_properties(1.0);
    let rectangle_inertia = 8.0 * 20.0 / 12.0;
    let caps_inertia = std::f32::consts::PI * (0.5 + 4.0) + 16.0 / 3.0;
    assert!((capsule.inertia - rectangle_inertia - caps_inertia).abs() < 1e-4);
    let segment = Shape2D::Segment {
        a: Vec2::new(2.0, 0.0),
        b: Vec2::new(4.0, 0.0),
    };
    let mass = segment.compute_mass_properties(1.0);
    assert!((mass.inertia - mass.mass * (4.0 / 12.0 + 9.0)).abs() < 1e-6);
    assert!(ConvexPolygon::from_points(&[Vec2::ZERO, Vec2::X, Vec2::new(f32::NAN, 1.0)]).is_none());
}

#[test]
fn sleeping_contacts_persist_without_phantom_ended_events() {
    let mut world = one_step_world();
    world.add_body(BodyBuilder::static_body(Shape2D::circle(1.0)).position((1.8, 0.0)));
    let body = world.add_body(BodyBuilder::dynamic(Shape2D::circle(1.0)));
    world.body_mut(body).unwrap().put_to_sleep();
    world.step(0.01);
    assert!(world
        .collision_events()
        .iter()
        .any(|ev| matches!(ev, CollisionEvent::Started { .. })));
    world.step(0.01);
    assert!(world
        .collision_events()
        .iter()
        .any(|ev| matches!(ev, CollisionEvent::Persisted { .. })));
    assert!(!world
        .collision_events()
        .iter()
        .any(|ev| matches!(ev, CollisionEvent::Ended { .. })));
    assert!(world.body(body).unwrap().is_sleeping);
}

#[test]
fn moving_kinematic_body_wakes_sleeping_contact() {
    let mut world = one_step_world();
    world.add_body(
        BodyBuilder::new(BodyType::Kinematic, Shape2D::circle(1.0))
            .position((-1.8, 0.0))
            .linear_velocity((0.1, 0.0)),
    );
    let sleeper = world.add_body(BodyBuilder::dynamic(Shape2D::circle(1.0)));
    world.body_mut(sleeper).unwrap().put_to_sleep();
    world.step(0.01);
    assert!(!world.body(sleeper).unwrap().is_sleeping);
    assert!(world.body(sleeper).unwrap().linear_velocity.x > 0.0);
}

#[test]
fn kinematic_sensors_emit_events_and_preserve_removed_sensor_identity() {
    let mut world = one_step_world();
    let sensor =
        world.add_body(BodyBuilder::new(BodyType::Kinematic, Shape2D::circle(1.0)).sensor(true));
    let other = world.add_body(BodyBuilder::static_body(Shape2D::circle(1.0)));
    world.step(0.01);
    assert!(world.trigger_events().iter().any(|ev| matches!(ev,
        TriggerEvent::Entered { sensor: s, other: o } if *s == sensor && *o == other)));
    world.remove_body(sensor);
    world.step(0.01);
    assert!(world.trigger_events().iter().any(|ev| matches!(ev,
        TriggerEvent::Exited { sensor: s, other: o } if *s == sensor && *o == other)));
}

#[test]
fn transient_sensor_crossings_survive_later_substeps() {
    let mut world = PhysicsWorld::new(Vec2::ZERO);
    world.config.substeps = 4;
    let sensor = world.add_body(BodyBuilder::static_body(Shape2D::circle(0.2)).sensor(true));
    let body = world.add_body(
        BodyBuilder::dynamic(Shape2D::circle(0.1))
            .position((-0.75, 0.0))
            .linear_velocity((3.0, 0.0))
            .linear_damping(0.0),
    );
    world.step(1.0);
    assert!(world.trigger_events().iter().any(|ev| matches!(ev,
        TriggerEvent::Entered { sensor: s, other } if *s == sensor && *other == body)));
    assert!(world.trigger_events().iter().any(|ev| matches!(ev,
        TriggerEvent::Exited { sensor: s, other } if *s == sensor && *other == body)));
}

#[test]
fn collision_impact_speed_is_measured_before_solver_impulses() {
    let mut world = one_step_world();
    world.add_body(
        BodyBuilder::dynamic(Shape2D::circle(1.0))
            .position((-0.9, 0.0))
            .linear_velocity((2.0, 0.0))
            .linear_damping(0.0)
            .restitution(0.0),
    );
    world.add_body(
        BodyBuilder::dynamic(Shape2D::circle(1.0))
            .position((0.9, 0.0))
            .linear_velocity((-2.0, 0.0))
            .linear_damping(0.0)
            .restitution(0.0),
    );
    world.step(0.01);
    let impact = world
        .collision_events()
        .iter()
        .find_map(|ev| match ev {
            CollisionEvent::Started { impact_speed, .. } => Some(*impact_speed),
            _ => None,
        })
        .unwrap();
    assert!((impact - 4.0).abs() < 1e-5, "closing speed: {impact}");
}

#[test]
fn collision_impact_speed_includes_rotation_at_contact_point() {
    let mut world = one_step_world();
    let spinning = world.add_body(
        BodyBuilder::dynamic(Shape2D::rect(2.0, 2.0))
            .angular_damping(0.0)
            .restitution(0.0),
    );
    world.body_mut(spinning).unwrap().angular_velocity = -2.0;
    world.add_body(BodyBuilder::static_body(Shape2D::circle(0.25)).position((1.2, 0.5)));
    world.step(0.01);
    let speed = world
        .collision_events()
        .iter()
        .find_map(|event| match event {
            CollisionEvent::Started { impact_speed, .. } => Some(*impact_speed),
            _ => None,
        })
        .unwrap();
    assert!(
        speed > 0.9,
        "a rotating edge can impact without center-of-mass velocity: {speed}"
    );
}

#[test]
fn repeated_position_iterations_do_not_overcorrect_penetration() {
    let mut world = one_step_world();
    world.config.position_iterations = 32;
    let a = world.add_body(BodyBuilder::dynamic(Shape2D::circle(1.0)).position((-0.5, 0.0)));
    let b = world.add_body(BodyBuilder::dynamic(Shape2D::circle(1.0)).position((0.5, 0.0)));
    world.step(0.01);
    let distance = world
        .body(a)
        .unwrap()
        .position
        .distance(world.body(b).unwrap().position);
    assert!(
        (1.98..=2.0).contains(&distance),
        "overcorrected distance: {distance}"
    );
}

#[test]
fn motor_torque_budget_is_independent_of_iteration_count() {
    let run = |iterations| {
        let mut world = one_step_world();
        world.config.velocity_iterations = iterations;
        let anchor = world.add_body(BodyBuilder::static_body(Shape2D::circle(0.1)));
        let wheel = world.add_body(BodyBuilder::dynamic(Shape2D::circle(1.0)).angular_damping(0.0));
        world.add_joint(
            Joint::revolute(anchor, wheel, Vec2::ZERO, Vec2::ZERO).with_motor(100.0, 1.0),
        );
        world.step(0.1);
        let body = world.body(wheel).unwrap();
        assert!((body.angular_velocity * body.inertia - 0.1).abs() < 1e-5);
        body.angular_velocity
    };
    assert!((run(1) - run(16)).abs() < 1e-6);
}

#[test]
fn pointer_target_force_budget_is_independent_of_iteration_count() {
    let run = |iterations| {
        let mut world = one_step_world();
        world.config.velocity_iterations = iterations;
        let body = world.add_body(BodyBuilder::dynamic(Shape2D::circle(1.0)).linear_damping(0.0));
        world.add_joint(Joint::target(body, Vec2::ZERO, Vec2::new(100.0, 0.0), 2.0));
        world.step(0.1);
        let body = world.body(body).unwrap();
        assert!((body.linear_velocity.x * body.mass - 0.2).abs() < 1e-5);
        body.linear_velocity.x
    };
    assert!((run(1) - run(16)).abs() < 1e-6);
}

#[test]
fn compliant_spring_strength_does_not_grow_with_solver_iterations() {
    let run = |iterations| {
        let mut world = one_step_world();
        world.config.velocity_iterations = iterations;
        let anchor = world.add_body(BodyBuilder::static_body(Shape2D::circle(0.1)));
        let body = world.add_body(BodyBuilder::dynamic(Shape2D::circle(0.5)).position((3.0, 0.0)));
        world.add_joint(Joint::spring(
            anchor,
            body,
            Vec2::ZERO,
            Vec2::ZERO,
            2.0,
            2.0,
            0.8,
        ));
        world.step(0.01);
        world.body(body).unwrap().linear_velocity.x
    };
    assert!((run(1) - run(16)).abs() < 1e-5);
}

#[test]
fn parallel_boundary_rays_and_exact_distance_hits_work() {
    let bounds = Aabb2::new(Vec2::ZERO, Vec2::splat(1.0));
    let ray = Ray2::new(Vec2::new(-1.0, 1.0), Vec2::X);
    assert_eq!(
        bounds.ray_intersect(ray.origin, ray.inv_direction(), 1.0),
        Some(1.0)
    );
    let mut world = one_step_world();
    let body = world.add_body(BodyBuilder::static_body(Shape2D::circle(1.0)).position((2.0, 0.0)));
    assert_eq!(
        world
            .raycast(Ray2::new(Vec2::ZERO, Vec2::X), 1.0, CollisionFilter::ALL)
            .unwrap()
            .body,
        body
    );
}

#[test]
fn fixed_timestep_rejects_invalid_time_without_poisoning_state() {
    let mut clock = FixedTimestep::new(0.01);
    assert_eq!(clock.advance(f32::NAN, |_| panic!("no step")), 0.0);
    assert_eq!(clock.advance(f32::INFINITY, |_| panic!("no step")), 0.0);
    let mut count = 0;
    clock.advance(0.02, |_| count += 1);
    assert_eq!(count, 2);
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        clock.dt = dt;
        assert_eq!(clock.advance(0.1, |_| panic!("invalid timestep")), 0.0);
    }
    clock.dt = f32::MIN_POSITIVE;
    let mut catch_up_steps = 0;
    let alpha = clock.advance(0.1, |_| catch_up_steps += 1);
    assert_eq!(catch_up_steps, 1024);
    assert!((0.0..1.0).contains(&alpha));
}

#[test]
fn undriven_spring_still_obeys_viscous_damping() {
    let mut spring = Spring1D::new(0.0, SpringConfig::new(0.0, 2.0, 1.0));
    spring.velocity = 10.0;
    spring.step(1.0);
    assert!((spring.velocity - 10.0 * (-2.0_f32).exp()).abs() < 1e-5);
    assert!((spring.value - 5.0 * (1.0 - (-2.0_f32).exp())).abs() < 1e-5);
}

#[test]
fn inertia_integrates_consistently_at_different_frame_rates() {
    let mut single = InertiaScroller::new(-10000.0, 10000.0, 500.0);
    single.jump_to(0.0);
    single.end_drag(1000.0);
    let mut divided = single.clone();
    single.step(0.1);
    for _ in 0..10 {
        divided.step(0.01);
    }
    assert!((single.position - divided.position).abs() < 1e-3);
    assert!((single.velocity - divided.velocity).abs() < 1e-3);
}

#[test]
fn explicit_scroller_targets_end_drag_and_clamp_to_bounds() {
    let mut scroller = InertiaScroller::new(0.0, 100.0, 100.0);
    scroller.begin_drag();
    scroller.drag_by(20.0);
    scroller.animate_to(200.0);
    for _ in 0..120 {
        scroller.step(1.0 / 60.0);
    }
    assert!((scroller.position - 100.0).abs() < 0.01);
    scroller.jump_to(-20.0);
    assert_eq!(scroller.position, 0.0);
    scroller.animate_to(f32::NAN);
    scroller.step(f32::NAN);
    assert_eq!(scroller.position, 0.0);
}

#[test]
fn coincident_particles_separate_without_nonfinite_state() {
    let mut system = ParticleSystem::new(ParticleConfig {
        gravity: Vec2::ZERO,
        substeps: 1,
        ..ParticleConfig::default()
    });
    system.push(Vec2::new(2.0, 2.0), Vec2::ZERO, 0.0);
    system.push(Vec2::new(2.0, 2.0), Vec2::ZERO, 0.0);
    system.step_cpu(0.01);
    assert!(system.px[1] > system.px[0]);
    assert!(system.px.iter().all(|value| value.is_finite()));
    let before = system.to_gpu_particles();
    system.step_cpu(f32::NAN);
    assert_eq!(system.to_gpu_particles(), before);
}

#[test]
fn particle_export_reuses_destination_and_matches_allocating_helper() {
    let mut system = ParticleSystem::new(ParticleConfig::default());
    system.push(Vec2::new(2.0, 3.0), Vec2::new(4.0, 5.0), 0.25);
    system.push(Vec2::new(7.0, 8.0), Vec2::new(9.0, 10.0), 0.75);
    let mut destination = Vec::with_capacity(32);
    let allocation = destination.as_ptr();
    system.write_gpu_particles(&mut destination);
    assert_eq!(destination.as_ptr(), allocation);
    assert_eq!(destination.capacity(), 32);
    assert_eq!(destination, system.to_gpu_particles());
    system.clear();
    system.write_gpu_particles(&mut destination);
    assert!(destination.is_empty());
    assert_eq!(destination.as_ptr(), allocation);
}

#[test]
fn increasing_particle_radius_rebuilds_undersized_hash_cells() {
    let mut system = ParticleSystem::new(ParticleConfig {
        particle_radius: 0.1,
        gravity: Vec2::ZERO,
        substeps: 1,
        ..ParticleConfig::default()
    });
    system.push(Vec2::new(0.219, 2.0), Vec2::ZERO, 0.0);
    system.push(Vec2::new(0.449, 2.0), Vec2::ZERO, 0.0);
    system.config.particle_radius = 0.12;
    system.step_cpu(0.01);
    assert!(
        system.density[0] > 0.0,
        "overlapping particles in cells two apart must interact"
    );
}

#[test]
fn conservative_ccd_finds_thin_wall_during_long_translation() {
    let mut world = one_step_world();
    world.add_body(
        BodyBuilder::static_body(Shape2D::rect(0.01, 10.0))
            .position((5.073, 0.0))
            .restitution(0.5),
    );
    let bullet = world.add_body(
        BodyBuilder::dynamic(Shape2D::circle(0.05))
            .linear_velocity((60000.0, 0.0))
            .linear_damping(0.0)
            .bullet(true)
            .restitution(0.5),
    );
    let stats = world.step(1.0 / 60.0);
    assert_eq!(stats.ccd_events, 1);
    let body = world.body(bullet).unwrap();
    assert!(
        body.position.x < 5.073,
        "bullet crossed a thin wall: {}",
        body.position.x
    );
    assert!(
        body.linear_velocity.x < 0.0,
        "bullet must bounce away from the wall"
    );
}

#[test]
fn conservative_sweep_handles_short_casts_and_separated_parallel_paths() {
    use crate::narrowphase::swept_toi;
    let circle = Shape2D::circle(1.0);
    let hit = swept_toi(
        &circle,
        Transform2D::IDENTITY,
        Transform2D::new(Vec2::new(0.1, 0.0), 0.0),
        &circle,
        Transform2D::new(Vec2::new(2.05, 0.0), 0.0),
    )
    .unwrap();
    assert!((hit - 0.5).abs() < 1e-3);
    assert!(swept_toi(
        &circle,
        Transform2D::new(Vec2::new(0.0, 3.0), 0.0),
        Transform2D::new(Vec2::new(1000.0, 3.0), 0.0),
        &circle,
        Transform2D::new(Vec2::new(5.0, 0.0), 0.0)
    )
    .is_none());
}

#[cfg(feature = "gpu")]
#[test]
fn gpu_dense_bucket_visits_every_neighbor() {
    let Some(mut solver) = GpuParticleSolver::try_new(64) else {
        eprintln!("dense-bucket GPU regression skipped: no compatible adapter");
        return;
    };
    let config = ParticleConfig {
        gravity: Vec2::ZERO,
        damping: 1.0,
        particle_radius: 0.1,
        stiffness: 0.0,
        viscosity: 0.0,
        substeps: 1,
        ..ParticleConfig::default()
    };
    let particles = vec![
        GpuParticle {
            pos: [2.0, 2.0],
            ..GpuParticle::default()
        };
        64
    ];
    solver.upload_particles(&particles);
    let mut out = Vec::new();
    solver.step_and_readback(&config, &[], 0.01, &mut out);
    assert_eq!(out.len(), 64);
    for particle in out {
        assert!(
            (particle.density - 63.0).abs() < 1e-4,
            "all 63 coincident neighbors contribute"
        );
    }
}

#[cfg(feature = "gpu")]
#[test]
fn gpu_hash_collisions_do_not_double_count_neighbor_cells() {
    let Some(mut solver) = GpuParticleSolver::try_new(2) else {
        eprintln!("hash-collision GPU regression skipped: no compatible adapter");
        return;
    };
    // Cell (1,1) hashes to the same bucket as (-1,-1), both neighbors of (0,0).
    let config = ParticleConfig {
        bounds: Aabb2::new(Vec2::splat(-10.0), Vec2::splat(10.0)),
        gravity: Vec2::ZERO,
        damping: 1.0,
        particle_radius: 0.1,
        stiffness: 0.0,
        viscosity: 0.0,
        substeps: 1,
        ..ParticleConfig::default()
    };
    let particles = [
        GpuParticle {
            pos: [0.205, 0.205],
            ..GpuParticle::default()
        },
        GpuParticle {
            pos: [0.215, 0.215],
            ..GpuParticle::default()
        },
    ];
    solver.upload_particles(&particles);
    let mut out = Vec::new();
    solver.step_and_readback(&config, &[], 0.01, &mut out);
    let expected = 1.0 - (0.01_f32 * 0.01 * 2.0).sqrt() / 0.2;
    assert_eq!(out.len(), 2);
    for particle in out {
        assert!(
            (particle.density - expected).abs() < 1e-4,
            "neighbor counted more than once"
        );
    }
}

#[cfg(feature = "ferroui")]
#[test]
fn visual_bindings_restore_prior_transform_and_ignore_later_user_override() {
    use ferroui_base::media::{ITransform, MatrixTransform};
    use ferroui_base::{Matrix, Rect, Visual};
    use std::rc::Rc;

    let visual = Visual::new();
    let original = MatrixTransform::with_matrix(Matrix::new(1.0, 0.0, 0.0, 1.0, 7.0, 9.0));
    let original: Rc<dyn ITransform> = (&original).into();
    visual.set_render_transform(Some(original.clone()));
    {
        let mut host = VisualPhysicsHost::default();
        let bounds = Some(Rect::new(0.0, 0.0, 100.0, 40.0));
        let body = host.attach_visual(&visual, bounds, VisualBodyOptions::default());
        assert_eq!(
            host.attach_visual(&visual, bounds, VisualBodyOptions::default()),
            body
        );
        assert_eq!(host.bound_count(), 1);
    }
    assert!(Rc::ptr_eq(&visual.render_transform().unwrap(), &original));

    let mut driver = VisualSpringDriver::new();
    let first = driver.attach(&visual, SpringConfig::bouncy());
    assert_eq!(driver.attach(&visual, SpringConfig::bouncy()), first);
    assert!(driver.detach(first));
    assert!(!driver.detach(first));
    assert!(Rc::ptr_eq(&visual.render_transform().unwrap(), &original));
    let second = driver.attach(&visual, SpringConfig::bouncy());
    assert_ne!(second, first);
    visual.set_render_transform(Some(original.clone()));
    drop(driver);
    assert!(Rc::ptr_eq(&visual.render_transform().unwrap(), &original));
}

#[cfg(feature = "ferroui")]
#[test]
fn drag_velocity_uses_input_time_without_render_ticks() {
    use ferroui_base::Visual;
    let visual = Visual::new();
    let mut drag = DraggableVisual::attach(&visual, SpringConfig::bouncy(), vec![Vec2::ZERO]);
    drag.begin_drag(Vec2::ZERO);
    drag.drag_to(Vec2::new(10.0, 0.0), 0.01);
    drag.drag_to(Vec2::new(20.0, 0.0), 0.01);
    drag.end_drag();
    assert!(drag.card.position.velocity().x > 100.0);
}
