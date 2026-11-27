//! Comprehensive unit, integration, SIMD, UI physics, WGSL validation, and GPU compute tests.

use crate::prelude::*;
use crate::simd::{integrate_particles_scalar, integrate_particles_soa};

#[test]
fn free_fall_matches_analytic_kinematics() {
    let mut world = PhysicsWorld::new(Vec2::new(0.0, 10.0));
    let body = world.add_body(
        BodyBuilder::dynamic(Shape2D::circle(0.5))
            .position((0.0, 0.0))
            .linear_damping(0.0)
            .can_sleep(false),
    );

    let dt = 1.0 / 60.0;
    for _ in 0..60 {
        world.step(dt);
    }

    let b = world.body(body).unwrap();
    // Analytic: y = 0.5 * g * t^2 = 5.0, v = g * t = 10.0
    assert!((b.linear_velocity.y - 10.0).abs() < 0.05, "vy = {}", b.linear_velocity.y);
    assert!((b.position.y - 5.0).abs() < 0.12, "y = {}", b.position.y);
}

#[test]
fn elastic_head_on_collision_conserves_momentum_and_energy() {
    let mut world = PhysicsWorld::new(Vec2::ZERO);
    let mat = PhysicsMaterial::new(1.0, 1.0, 0.0);
    let a = world.add_body(
        BodyBuilder::dynamic(Shape2D::circle(0.5))
            .position((-2.0, 0.0))
            .linear_velocity((4.0, 0.0))
            .material(mat)
            .linear_damping(0.0)
            .can_sleep(false),
    );
    let b = world.add_body(
        BodyBuilder::dynamic(Shape2D::circle(0.5))
            .position((2.0, 0.0))
            .linear_velocity((0.0, 0.0))
            .material(mat)
            .linear_damping(0.0)
            .can_sleep(false),
    );

    let initial_p = world.body(a).unwrap().mass * 4.0;
    let initial_ke = world.body(a).unwrap().kinetic_energy();

    for _ in 0..90 {
        world.step(1.0 / 60.0);
    }

    let ba = world.body(a).unwrap();
    let bb = world.body(b).unwrap();
    let final_p = ba.mass * ba.linear_velocity.x + bb.mass * bb.linear_velocity.x;
    let final_ke = ba.kinetic_energy() + bb.kinetic_energy();

    // Equal masses: ball A stops and transfers velocity to ball B
    assert!(ba.linear_velocity.x.abs() < 0.15, "va = {:?}", ba.linear_velocity);
    assert!((bb.linear_velocity.x - 4.0).abs() < 0.15, "vb = {:?}", bb.linear_velocity);
    assert!((final_p - initial_p).abs() < 0.05 * initial_p);
    assert!((final_ke - initial_ke).abs() < 0.08 * initial_ke);
}

#[test]
fn stacked_box_tower_remains_stable_and_sleeps() {
    let mut world = PhysicsWorld::new(Vec2::new(0.0, 9.81)).with_config(SolverConfig::high_precision());
    world.add_body(
        BodyBuilder::static_body(Shape2D::rect(20.0, 1.0))
            .position((0.0, 5.5))
            .friction(0.7)
            .restitution(0.0),
    );

    let mut boxes = Vec::new();
    for i in 0..8 {
        let y = 4.5 - i as f32 * 1.0;
        let h = world.add_body(
            BodyBuilder::dynamic(Shape2D::rect(1.0, 1.0))
                .position((0.0, y))
                .friction(0.7)
                .restitution(0.0),
        );
        boxes.push(h);
    }

    for _ in 0..240 {
        world.step(1.0 / 60.0);
    }

    for (i, &h) in boxes.iter().enumerate() {
        let b = world.body(h).unwrap();
        assert!(b.position.x.abs() < 0.25, "box {i} drifted horizontally: x={}", b.position.x);
        assert!(b.position.y < 5.1, "box {i} fell through floor: y={}", b.position.y);
        assert!(b.linear_velocity.length() < 0.2, "box {i} still moving: v={:?}", b.linear_velocity);
    }
    let stats = world.last_stats();
    assert!(stats.sleeping_bodies > 0, "expected settled stack to enter sleep: {stats:?}");
}

#[test]
fn ccd_prevents_fast_bullet_tunneling_through_thin_wall() {
    let mut world = PhysicsWorld::new(Vec2::ZERO);
    // Thin vertical wall at x = 5.0 (thickness 0.08)
    world.add_body(
        BodyBuilder::static_body(Shape2D::rect(0.08, 10.0))
            .position((5.0, 0.0))
            .restitution(0.8),
    );
    // Supersonic bullet moving at 300 m/s (5 meters per 1/60s frame, 60x thicker than wall!)
    let bullet = world.add_body(
        BodyBuilder::dynamic(Shape2D::circle(0.15))
            .position((0.0, 0.0))
            .linear_velocity((300.0, 0.0))
            .linear_damping(0.0)
            .bullet(true)
            .restitution(0.8),
    );

    for _ in 0..5 {
        world.step(1.0 / 60.0);
    }

    let b = world.body(bullet).unwrap();
    assert!(b.position.x < 5.0, "bullet tunneled through thin wall! x={}", b.position.x);
    assert!(b.linear_velocity.x < 0.0, "bullet should have bounced back! vx={}", b.linear_velocity.x);
}

#[test]
fn revolute_motor_and_distance_joint_behave_accurately() {
    let mut world = PhysicsWorld::new(Vec2::new(0.0, 9.81));
    let anchor = world.add_body(BodyBuilder::static_body(Shape2D::circle(0.1)).position((0.0, 0.0)));
    let bob = world.add_body(
        BodyBuilder::dynamic(Shape2D::circle(0.3))
            .position((2.0, 0.0))
            .can_sleep(false),
    );
    world.add_joint(Joint::distance(anchor, bob, Vec2::ZERO, Vec2::ZERO, 2.0));

    for _ in 0..120 {
        world.step(1.0 / 60.0);
    }

    let b = world.body(bob).unwrap();
    let dist = b.position.length();
    assert!((dist - 2.0).abs() < 0.08, "pendulum rod drifted: dist={dist}");
    assert!(b.position.y > 0.0, "pendulum should swing downward under gravity");

    // Motorized spinner
    let hub = world.add_body(BodyBuilder::static_body(Shape2D::circle(0.1)).position((8.0, 0.0)));
    let blade = world.add_body(BodyBuilder::dynamic(Shape2D::rect(2.0, 0.2)).position((8.0, 0.0)).can_sleep(false));
    world.add_joint(Joint::revolute(hub, blade, Vec2::ZERO, Vec2::ZERO).with_motor(5.0, 50.0));
    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }
    let w = world.body(blade).unwrap().angular_velocity;
    assert!((w - 5.0).abs() < 0.4, "motor target speed not reached: w={w}");
}

#[test]
fn raycast_and_point_queries_hit_shapes_accurately() {
    let mut world = PhysicsWorld::new(Vec2::ZERO);
    let box_h = world.add_body(BodyBuilder::static_body(Shape2D::rect(2.0, 2.0)).position((5.0, 0.0)));
    let poly_h = world.add_body(BodyBuilder::static_body(Shape2D::regular_polygon(6, 1.0)).position((10.0, 0.0)));
    world.step(1.0 / 60.0);

    let hit = world
        .raycast(Ray2::new(Vec2::new(0.0, 0.0), Vec2::X), 20.0, CollisionFilter::ALL)
        .expect("ray should hit box");
    assert_eq!(hit.body, box_h);
    assert!((hit.toi - 4.0).abs() < 1e-3, "toi = {}", hit.toi);
    assert!((hit.normal.x + 1.0).abs() < 1e-3, "normal = {:?}", hit.normal);

    let picked = world.pick_body(Vec2::new(10.0, 0.0), false);
    assert_eq!(picked, Some(poly_h));
}

#[test]
fn simd_particle_integration_matches_scalar_reference() {
    let n = 1027; // deliberately non-multiple of 4 and 8 to test tail handling
    let mut px_simd: Vec<f32> = (0..n).map(|i| i as f32 * 0.1).collect();
    let mut py_simd: Vec<f32> = (0..n).map(|i| (i as f32 * 0.37).sin()).collect();
    let mut vx_simd: Vec<f32> = (0..n).map(|i| (i as f32 * 0.13).cos()).collect();
    let mut vy_simd: Vec<f32> = (0..n).map(|i| -(i as f32) * 0.02).collect();

    let mut px_ref = px_simd.clone();
    let mut py_ref = py_simd.clone();
    let mut vx_ref = vx_simd.clone();
    let mut vy_ref = vy_simd.clone();

    integrate_particles_soa(&mut px_simd, &mut py_simd, &mut vx_simd, &mut vy_simd, 0.5, 9.81, 0.995, 1.0 / 60.0);
    integrate_particles_scalar(&mut px_ref, &mut py_ref, &mut vx_ref, &mut vy_ref, 0.5, 9.81, 0.995, 1.0 / 60.0);

    for i in 0..n {
        assert!((px_simd[i] - px_ref[i]).abs() < 1e-5, "px mismatch at {i}");
        assert!((py_simd[i] - py_ref[i]).abs() < 1e-5, "py mismatch at {i}");
        assert!((vx_simd[i] - vx_ref[i]).abs() < 1e-5, "vx mismatch at {i}");
        assert!((vy_simd[i] - vy_ref[i]).abs() < 1e-5, "vy mismatch at {i}");
    }
}

#[test]
fn ui_springs_and_inertia_scroller_behave_smoothly_and_stably() {
    // Critically damped spring must NEVER overshoot target
    let mut crit = Spring1D::new(0.0, SpringConfig::critically_damped(0.3));
    crit.set_target(100.0);
    for _ in 0..120 {
        let v = crit.step(1.0 / 60.0);
        assert!(v <= 100.001, "critically damped spring overshot: {v}");
    }
    assert!(crit.is_settled());

    // Bouncy spring MUST overshoot target and then settle
    let mut bouncy = Spring1D::new(0.0, SpringConfig::bouncy());
    bouncy.set_target(100.0);
    let mut max_val = 0.0_f32;
    for _ in 0..180 {
        max_val = max_val.max(bouncy.step(1.0 / 60.0));
    }
    assert!(max_val > 105.0, "bouncy spring should overshoot, max={max_val}");
    assert!(bouncy.is_settled());

    // Unconditional stability under huge hitch dt = 5.0 seconds
    let mut hitch = Spring1D::new(0.0, SpringConfig::wobbly());
    hitch.set_target(50.0);
    let v = hitch.step(5.0);
    assert!(v.is_finite() && (v - 50.0).abs() < 1.0, "spring unstable at huge dt: {v}");

    // Inertia scroller rubber-band and snap-back
    let mut scroller = InertiaScroller::new(0.0, 500.0, 200.0);
    scroller.begin_drag();
    scroller.drag_by(-100.0);
    assert!(scroller.position < 0.0 && scroller.position > -100.0, "rubber band failed: {}", scroller.position);
    scroller.end_drag(0.0);
    for _ in 0..120 {
        scroller.step(1.0 / 60.0);
    }
    assert!(scroller.position.abs() < 0.5, "did not bounce back to 0: {}", scroller.position);
}

#[test]
fn cpu_particle_system_simulates_and_stays_in_bounds() {
    let mut ps = ParticleSystem::new(ParticleConfig::default());
    ps.spawn_grid(Aabb2::new(Vec2::new(1.0, 1.0), Vec2::new(7.0, 5.0)), 512);
    for _ in 0..30 {
        ps.step_cpu(1.0 / 60.0);
    }
    let bounds = ps.config.bounds;
    for i in 0..ps.len() {
        assert!(ps.px[i].is_finite() && ps.py[i].is_finite());
        assert!(ps.px[i] >= bounds.min.x && ps.px[i] <= bounds.max.x);
        assert!(ps.py[i] >= bounds.min.y && ps.py[i] <= bounds.max.y);
    }
}

#[cfg(feature = "gpu")]
#[test]
fn wgsl_compute_shaders_validate_with_naga() {
    use crate::gpu::shaders::{PARTICLE_PHYSICS_WGSL, RIGID_INTEGRATE_WGSL};

    for (name, src, expected_entries) in [
        ("PARTICLE_PHYSICS_WGSL", PARTICLE_PHYSICS_WGSL, &["clear_grid", "integrate_and_hash", "resolve_collisions"][..]),
        ("RIGID_INTEGRATE_WGSL", RIGID_INTEGRATE_WGSL, &["integrate_bodies"][..]),
    ] {
        let module = naga::front::wgsl::parse_str(src)
            .unwrap_or_else(|e| panic!("{name} parse failed:\n{}", e.emit_to_string(src)));
        let mut validator =
            naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::empty());
        validator
            .validate(&module)
            .unwrap_or_else(|e| panic!("{name} validation failed:\n{}", e.emit_to_string(src)));
        for &ep in expected_entries {
            assert!(
                module.entry_points.iter().any(|e| e.name == ep && e.stage == naga::ShaderStage::Compute),
                "{name} missing compute entry point `{ep}`"
            );
        }
    }
}

#[cfg(feature = "gpu")]
#[test]
fn gpu_particle_solver_executes_on_hardware_device() {
    let Some(mut solver) = GpuParticleSolver::try_new(1024) else {
        // Headless CI machine without a GPU adapter: skip execution test cleanly.
        return;
    };

    let config = ParticleConfig::default();
    let mut ps = ParticleSystem::new(config);
    ps.spawn_grid(Aabb2::new(Vec2::new(2.0, 1.0), Vec2::new(8.0, 4.0)), 1024);
    let initial = ps.to_gpu_particles();
    let initial_mean_y: f32 = initial.iter().map(|p| p.pos[1]).sum::<f32>() / initial.len() as f32;

    solver.upload_particles(&initial);
    let mut out = Vec::new();
    for _ in 0..20 {
        solver.step_and_readback(&config, &[], 1.0 / 60.0, &mut out);
    }

    assert_eq!(out.len(), 1024);
    let final_mean_y: f32 = out.iter().map(|p| p.pos[1]).sum::<f32>() / out.len() as f32;
    assert!(
        final_mean_y > initial_mean_y + 0.1,
        "GPU particles should fall under gravity: initial_y={initial_mean_y}, final_y={final_mean_y}"
    );
    for p in &out {
        assert!(p.pos[0].is_finite() && p.pos[1].is_finite());
        assert!(p.pos[0] >= config.bounds.min.x && p.pos[0] <= config.bounds.max.x);
        assert!(p.pos[1] >= config.bounds.min.y && p.pos[1] <= config.bounds.max.y);
    }
}

#[cfg(feature = "gpu")]
#[test]
fn gpu_rigid_batch_integrator_computes_positions_and_aabbs() {
    let Some(mut integrator) = GpuRigidBatchIntegrator::try_new() else {
        return;
    };
    let mut bodies = vec![
        GpuRigidBodyRecord {
            pos: [1.0, 2.0],
            vel: [3.0, 0.0],
            force: [0.0, 0.0],
            angle: 0.0,
            ang_vel: 1.0,
            inv_mass: 1.0,
            inv_inertia: 1.0,
            damping: 0.0,
            gravity_scale: 1.0,
        };
        128
    ];
    let radii = vec![0.5_f32; 128];
    let aabbs = integrator.integrate_batch(&mut bodies, &radii, [0.0, 10.0], 0.1);
    assert_eq!(aabbs.len(), 128);
    assert!((bodies[0].vel[1] - 1.0).abs() < 1e-3);
    assert!((bodies[0].pos[0] - 1.3).abs() < 1e-3);
    assert!((bodies[0].pos[1] - 2.1).abs() < 1e-3);
    assert!((aabbs[0].min_xy[0] - (bodies[0].pos[0] - 0.5)).abs() < 1e-3);
    assert!((aabbs[0].max_xy[1] - (bodies[0].pos[1] + 0.5)).abs() < 1e-3);
}

#[test]
fn buoyancy_force_field_floats_light_body_and_triggers_events() {
    let mut world = PhysicsWorld::new(Vec2::new(0.0, 9.81));
    world.add_force_field(ForceField::Buoyancy {
        region: Aabb2::new(Vec2::new(-10.0, 0.0), Vec2::new(10.0, 10.0)),
        fluid_density: 2.0,
        linear_drag: 2.0,
        angular_drag: 2.0,
    });
    let sensor = world.add_body(
        BodyBuilder::static_body(Shape2D::rect(20.0, 10.0))
            .position((0.0, 5.0))
            .sensor(true),
    );
    let cork = world.add_body(
        BodyBuilder::dynamic(Shape2D::circle(0.5))
            .position((0.0, 3.0))
            .density(0.4)
            .can_sleep(false),
    );

    let mut saw_trigger = false;
    for _ in 0..180 {
        world.step(1.0 / 60.0);
        if world
            .trigger_events()
            .iter()
            .any(|e| matches!(e, TriggerEvent::Entered { sensor: s, other } if *s == sensor && *other == cork))
        {
            saw_trigger = true;
        }
    }
    assert!(saw_trigger, "expected sensor Entered trigger event");
    let final_y = world.body(cork).unwrap().position.y;
    assert!(final_y < 1.5, "light cork should float up toward water surface y=0, got y={final_y}");
}

#[test]
fn breakable_weld_snaps_under_excessive_impulse_and_prismatic_respects_limits() {
    let mut world = PhysicsWorld::new(Vec2::ZERO);
    let anchor = world.add_body(BodyBuilder::static_body(Shape2D::rect(1.0, 1.0)).position((0.0, 0.0)));
    let arm = world.add_body(BodyBuilder::dynamic(Shape2D::rect(1.0, 1.0)).position((1.0, 0.0)).can_sleep(false));
    let weld = world.add_joint(Joint::weld(anchor, arm, Vec2::new(0.5, 0.0), Vec2::new(-0.5, 0.0), 0.0).with_break_force(10.0));

    // Apply strong blast impulse to snap the weld
    world.body_mut(arm).unwrap().apply_impulse(Vec2::new(0.0, 200.0));
    world.step(1.0 / 60.0);
    assert!(world.joint(weld).unwrap().broken, "weld should snap under high impulse");

    // Prismatic slider limited to [-1.0, 1.0] along X
    let rail = world.add_body(BodyBuilder::static_body(Shape2D::circle(0.1)).position((10.0, 0.0)));
    let slider = world.add_body(BodyBuilder::dynamic(Shape2D::rect(0.5, 0.5)).position((10.0, 0.0)).can_sleep(false));
    world.add_joint(
        Joint::prismatic(rail, slider, Vec2::ZERO, Vec2::ZERO, Vec2::X)
            .with_limits(-1.0, 1.0)
            .with_motor(20.0, 100.0),
    );
    for _ in 0..60 {
        world.step(1.0 / 60.0);
    }
    let pos = world.body(slider).unwrap().position;
    assert!(pos.x <= 11.15 && pos.x >= 10.8, "slider should stop near upper limit x=11.0, got {}", pos.x);
    assert!(pos.y.abs() < 0.05, "slider should remain on rail y=0, got {}", pos.y);
}

#[test]
fn draggable_card_magnetic_snap_and_fixed_timestep_work_cleanly() {
    let snap = MagneticSnap::new(vec![Vec2::new(0.0, 0.0), Vec2::new(200.0, 0.0), Vec2::new(400.0, 0.0)]);
    // At x=90 with strong rightward throw velocity, velocity projection picks anchor 1 (x=200) or 2 (x=400)
    let (idx, target) = snap.choose_anchor(Vec2::new(90.0, 0.0), Vec2::new(1000.0, 0.0)).unwrap();
    assert!(idx >= 1, "expected rightward velocity to project past anchor 0, got idx={idx}");

    let mut card = DraggableCard::new(Vec2::ZERO, SpringConfig::bouncy());
    card.begin_drag(Vec2::new(20.0, 0.0));
    card.drag_to(Vec2::new(90.0, 0.0), 1.0 / 60.0);
    card.end_drag(Vec2::new(1000.0, 0.0), target);

    let mut fixed = FixedTimestep::new(1.0 / 60.0);
    let mut steps = 0;
    for _ in 0..120 {
        let alpha = fixed.advance(1.0 / 60.0, |dt| {
            card.step(dt);
            steps += 1;
        });
        assert!((0.0..=1.0).contains(&alpha));
    }
    assert_eq!(steps, 120);
    assert!(card.position.value().distance(target) < 1.0);
    assert!(card.tilt_radians.value.abs() < 0.02);
}

#[test]
fn xpbd_cloth_tears_and_softbody_preserves_area() {
    let bounds = Aabb2::new(Vec2::new(0.0, 0.0), Vec2::new(16.0, 10.0));
    let mut cloth = ClothMesh::grid(Vec2::new(2.0, 1.0), 8, 6, 0.4, 0.0002, Some(2.2), bounds);
    let before = cloth.active_constraint_count();
    for i in 0..30 {
        cloth.step(1.0 / 60.0, Vec2::new(0.0, 9.81), Vec2::new(4.0, 0.0), i as f32 / 60.0, &[]);
    }
    let torn = cloth.tear_at(Vec2::new(3.4, 2.2), 0.65);
    assert!(torn > 0 && cloth.active_constraint_count() < before);

    let mut jelly = SoftBody2D::circle_blob(Vec2::new(8.0, 3.0), 0.9, 14, 1.12, 0.0004, bounds, 0xFF34D399);
    let rest_area = jelly.rest_area;
    for _ in 0..120 {
        jelly.step(1.0 / 60.0, Vec2::new(0.0, 9.81), &[]);
    }
    let cur_area = jelly.current_area();
    assert!(
        (cur_area / rest_area - 1.0).abs() < 0.35,
        "softbody collapsed or exploded: rest={rest_area}, cur={cur_area}"
    );
    assert!(jelly.center().y > 7.5, "jelly should settle near floor");
}

#[test]
fn character_controller_slides_grounds_and_pushes_boxes() {
    let mut world = PhysicsWorld::new(Vec2::new(0.0, 9.81));
    world.add_body(BodyBuilder::static_body(Shape2D::rect(20.0, 1.0)).position((8.0, 9.5)));
    let crate_h = world.add_body(
        BodyBuilder::dynamic(Shape2D::rect(0.8, 0.8))
            .position((6.0, 8.6))
            .can_sleep(false),
    );
    world.step(1.0 / 60.0);

    let mut ctrl = CharacterController2D::new(Shape2D::capsule(0.35, 0.28), Vec2::new(3.0, 7.5));
    let mut ever_grounded = false;
    for _ in 0..90 {
        ctrl.velocity.x = 5.0;
        ctrl.velocity.y += 9.81 * (1.0 / 60.0);
        let res = ctrl.move_and_slide(&mut world, 1.0 / 60.0, None);
        if res.is_grounded {
            ever_grounded = true;
        }
        world.step(1.0 / 60.0);
    }
    assert!(ever_grounded, "character should land on floor");
    assert!(world.body(crate_h).unwrap().position.x > 6.1, "character should push dynamic crate");
}

#[test]
fn fracture_and_world_history_rollback_work_accurately() {
    let mut world = PhysicsWorld::new(Vec2::new(0.0, 9.81));
    world.add_body(BodyBuilder::static_body(Shape2D::rect(20.0, 1.0)).position((8.0, 9.5)));
    let box_h = world.add_body(
        BodyBuilder::dynamic(Shape2D::rect(1.4, 1.4))
            .position((8.0, 3.0))
            .linear_velocity((2.0, 0.0))
            .can_sleep(false),
    );

    let mut history = WorldHistory::new(120);
    history.record(&world);
    let pos0 = world.body(box_h).unwrap().position;

    for _ in 0..30 {
        world.step(1.0 / 60.0);
        history.record(&world);
    }
    let pos30 = world.body(box_h).unwrap().position;
    assert!(pos30.y > pos0.y + 0.5);

    // Scrub back to frame 0 and verify exact restoration!
    assert!(history.scrub(&mut world, 0.0));
    assert!(world.body(box_h).unwrap().position.distance(pos0) < 1e-4);

    // Fracture box into shards
    let shards = fracture_body(&mut world, box_h, pos0 + Vec2::new(0.1, 0.0), 3.0);
    assert!(shards.len() >= 4, "expected >=4 shards, got {}", shards.len());
    assert!(world.body(box_h).is_none(), "original body should be removed after fracture");

    // Verify closest_point and SpringChain
    let hit = world.closest_point(Vec2::new(8.0, 8.2), 3.0, CollisionFilter::ALL);
    assert!(hit.is_some());

    let mut chain = SpringChain::new(6, Vec2::ZERO, SpringConfig::snappy());
    chain.set_leader_target(Vec2::new(10.0, 5.0));
    for _ in 0..60 {
        chain.step(1.0 / 60.0);
    }
    assert!(chain.nodes.last().unwrap().value().distance(Vec2::new(10.0, 5.0)) < 1.0);
}

#[cfg(feature = "ferroui")]
#[test]
fn ferroui_visual_physics_host_and_spring_driver_update_render_transforms() {
    use ferroui_base::{Rect, Visual};

    let vis = Visual::new();
    let mut host = VisualPhysicsHost::new(64.0);
    host.world.gravity = Vec2::new(0.0, 9.81);
    host.set_container_bounds(Rect::new(0.0, 0.0, 640.0, 480.0));
    let bh = host.attach_visual(&vis, Some(Rect::new(100.0, 40.0, 120.0, 48.0)), VisualBodyOptions::dynamic_card());

    for _ in 0..30 {
        host.step_and_sync(1.0 / 60.0);
    }
    let mat = vis.render_transform().expect("MatrixTransform attached").value();
    assert!(mat.m32 > 10.0, "expected visual to fall downward under gravity via render_transform, m32={}", mat.m32);

    // Enable home springs and verify it pulls back toward rest (0, 0)
    host.world.gravity = Vec2::ZERO;
    host.enable_home_springs(4.5, 0.85);
    for _ in 0..120 {
        host.step_and_sync(1.0 / 60.0);
    }
    let mat_home = vis.render_transform().unwrap().value();
    assert!(mat_home.m32.abs() < 4.0, "home spring should pull visual back to layout origin, m32={}", mat_home.m32);
    assert!(host.world.body(bh).is_some());

    // VisualSpringDriver error-shake & pop test
    let vis2 = Visual::new();
    let mut driver = VisualSpringDriver::new();
    let sid = driver.attach(&vis2, SpringConfig::bouncy());
    driver.impulse_shake(sid, 400.0);
    let active = driver.step(1.0 / 60.0);
    assert_eq!(active, 1);
    let shake_mat = vis2.render_transform().unwrap().value();
    assert!(shake_mat.m31.abs() > 1.0, "expected horizontal shake offset, m31={}", shake_mat.m31);
}
