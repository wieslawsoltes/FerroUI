//! Focused scene construction; frame stepping and interaction live in the parent module.

use super::*;

impl SandboxEngine {
    pub(super) fn build_playground_scene(&mut self) {
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.config = self.solver_profile.config();
        self.add_arena_walls();

        let palette = [
            0xFF38BDF8, 0xFF818CF8, 0xFFF472B6, 0xFF34D399, 0xFFFBBF24, 0xFFFB7185,
        ];
        let base_x = 3.8;
        let base_y = WORLD_HEIGHT - 1.05;
        let size = 0.56;
        let mut color_i = 0usize;
        for row in 0..6 {
            let cols = 6 - row;
            let start_x = base_x - (cols as f32 - 1.0) * (size * 0.52);
            for col in 0..cols {
                self.world.add_body(
                    BodyBuilder::dynamic(Shape2D::rect(size, size))
                        .position((
                            start_x + col as f32 * (size * 1.04),
                            base_y - row as f32 * (size + 0.01),
                        ))
                        .friction(0.6)
                        .restitution(self.restitution * 0.4)
                        .user_data(palette[color_i % palette.len()]),
                );
                color_i += 1;
            }
        }

        // Motorized windmill spinner
        let hub = self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.18))
                .position((10.8, 5.4))
                .user_data(0xFF94A3B8),
        );
        let blade = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::rect(3.5, 0.24))
                .position((10.8, 5.4))
                .density(2.0)
                .can_sleep(false)
                .user_data(0xFFF59E0B),
        );
        self.world
            .add_joint(Joint::revolute(hub, blade, Vec2::ZERO, Vec2::ZERO).with_motor(2.2, 85.0));

        // Hanging chain bridge
        let left_anchor = self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.15))
                .position((7.0, 2.0))
                .user_data(0xFF94A3B8),
        );
        let right_anchor = self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.15))
                .position((14.2, 2.0))
                .user_data(0xFF94A3B8),
        );
        let links = 8;
        let link_w = 0.78;
        let mut prev = left_anchor;
        let mut prev_local = Vec2::ZERO;
        for i in 0..links {
            let x = 7.45 + i as f32 * 0.85;
            let plank = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(link_w, 0.16))
                    .position((x, 2.1))
                    .density(1.2)
                    .user_data(0xFF2DD4BF),
            );
            self.world.add_joint(Joint::revolute(
                prev,
                plank,
                prev_local,
                Vec2::new(-link_w * 0.46, 0.0),
            ));
            prev = plank;
            prev_local = Vec2::new(link_w * 0.46, 0.0);
        }
        self.world
            .add_joint(Joint::revolute(prev, right_anchor, prev_local, Vec2::ZERO));

        for i in 0..7 {
            let x = 8.0 + i as f32 * 0.95;
            let shape = match i % 3 {
                0 => Shape2D::circle(0.32),
                1 => Shape2D::regular_polygon(5, 0.36),
                _ => Shape2D::regular_polygon(6, 0.35),
            };
            self.world.add_body(
                BodyBuilder::dynamic(shape)
                    .position((x, 0.9 + (i % 2) as f32 * 0.35))
                    .restitution(0.65)
                    .friction(0.35)
                    .user_data(palette[(i + 2) % palette.len()]),
            );
        }
    }

    pub(super) fn build_gpu_particles_scene(&mut self) {
        let radius = if self.particle_target_count >= 8000 {
            0.056
        } else if self.particle_target_count >= 4000 {
            0.070
        } else {
            0.088
        };
        let config = ParticleConfig {
            bounds: Aabb2::new(
                Vec2::new(0.25, 0.25),
                Vec2::new(WORLD_WIDTH - 0.25, WORLD_HEIGHT - 0.25),
            ),
            gravity: Vec2::new(0.0, self.gravity_y),
            particle_radius: radius,
            damping: 0.996,
            restitution: 0.3,
            stiffness: 0.58,
            viscosity: 0.045,
            substeps: 2,
            pointer_center: Vec2::new(WORLD_WIDTH * 0.5, WORLD_HEIGHT * 0.5),
            pointer_strength: 0.0,
            pointer_radius: 3.4,
        };
        self.particles = ParticleSystem::new(config);
        self.particles.spawn_grid(
            Aabb2::new(Vec2::new(0.55, 0.55), Vec2::new(10.0, 9.0)),
            self.particle_target_count,
        );
        self.particles.obstacles = vec![
            ParticleObstacle {
                center: [5.4, 5.1],
                radius: 0.85,
                restitution: 0.5,
            },
            ParticleObstacle {
                center: [10.4, 6.4],
                radius: 1.05,
                restitution: 0.5,
            },
            ParticleObstacle {
                center: [12.9, 3.8],
                radius: 0.72,
                restitution: 0.5,
            },
        ];
        self.particles
            .write_gpu_particles(&mut self.gpu_particles_view);
        if let Some(gpu) = self.gpu_solver.as_mut() {
            gpu.upload_particles(&self.gpu_particles_view);
            self.gpu_synced = true;
        }
    }

    pub(super) fn build_cloth_softbody_scene(&mut self) {
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.config = self.solver_profile.config();
        self.add_arena_walls();

        let bounds = Aabb2::new(
            Vec2::new(0.2, 0.2),
            Vec2::new(WORLD_WIDTH - 0.2, WORLD_HEIGHT - 0.78),
        );
        // Tearable XPBD hanging cloth curtain on the left
        self.cloth = Some(ClothMesh::grid(
            Vec2::new(1.2, 1.0),
            16,
            11,
            0.36,
            0.00018,
            Some(2.05),
            bounds,
        ));

        // Rigid bumper pegs & ramps on the right for jelly soft bodies to squish over!
        for &(x, y, r) in &[(10.2, 5.2, 0.65), (12.8, 6.4, 0.72), (8.6, 7.6, 0.55)] {
            self.world.add_body(
                BodyBuilder::static_body(Shape2D::circle(r))
                    .position((x, y))
                    .user_data(0xFF334155),
            );
        }
        // Slanted ramp under the jelly drop zone
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::rect(3.4, 0.22))
                .position((11.6, 8.2))
                .angle(-0.22)
                .user_data(0xFF475569),
        );
        // Dynamic ball swinging into cloth
        let ball = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.52))
                .position((3.6, 6.8))
                .linear_velocity((3.2, -2.0))
                .restitution(0.75)
                .user_data(0xFFF59E0B),
        );
        let _ = ball;

        // Three colorful pressure-inflated XPBD jelly blobs!
        self.soft_bodies = vec![
            SoftBody2D::circle_blob(
                Vec2::new(10.0, 2.2),
                0.92,
                18,
                1.15,
                0.00035,
                bounds,
                0xCC34D399,
            ),
            SoftBody2D::circle_blob(
                Vec2::new(12.5, 2.6),
                0.85,
                18,
                1.12,
                0.0004,
                bounds,
                0xCCF472B6,
            ),
            SoftBody2D::circle_blob(
                Vec2::new(11.3, 1.1),
                0.78,
                16,
                1.18,
                0.0003,
                bounds,
                0xCC38BDF8,
            ),
        ];
    }

    pub(super) fn build_ui_springs_scene(&mut self) {
        let anchors = vec![
            Vec2::new(2.7, 2.55),
            Vec2::new(6.3, 2.55),
            Vec2::new(9.9, 2.55),
            Vec2::new(4.5, 5.45),
            Vec2::new(8.1, 5.45),
        ];
        self.ui_snap = MagneticSnap::new(anchors.clone());
        let cfg = self.spring_preset.config();

        let defs = [
            ("PiP Player", "Magnetic Corner", 0xFF0284C7, 0),
            ("Kanban Card", "Velocity Tilt", 0xFF7C3AED, 1),
            ("Wallet Pass", "Spring Reflow", 0xFFDB2777, 2),
            ("Quick Action", "Analytical ODE", 0xFF059669, 3),
        ];
        self.ui_cards = defs
            .into_iter()
            .map(|(label, subtitle, accent_argb, slot_index)| {
                let mut card = DraggableCard::new(anchors[slot_index] + Vec2::new(0.0, -1.5), cfg);
                card.position.set_target(anchors[slot_index]);
                card.tilt_sensitivity = 0.032;
                UiDemoCard {
                    label,
                    subtitle,
                    accent_argb,
                    half_size: Vec2::new(1.48, 0.88),
                    card,
                    slot_index,
                }
            })
            .collect();

        // Expandable Radial Spring FAB Menu on the right side
        self.ui_fab_center = Vec2::new(13.6, 5.45);
        self.ui_fab_open = true;
        let fab_defs = [
            ("Cam", 0xFF38BDF8),
            ("Mic", 0xFFA78BFA),
            ("Pin", 0xFFF472B6),
            ("Tag", 0xFF34D399),
            ("Zap", 0xFFF59E0B),
        ];
        self.ui_fab_items = fab_defs
            .into_iter()
            .enumerate()
            .map(|(i, (lbl, col))| {
                let ang = -PI + (i as f32) * (PI / 4.0);
                let target = self.ui_fab_center + Vec2::new(ang.cos() * 1.45, ang.sin() * 1.45);
                let mut sp = Spring2D::new(self.ui_fab_center, cfg);
                sp.set_target(target);
                (sp, lbl, col)
            })
            .collect();
        self.recompute_spring_curve();

        self.ui_scroller = InertiaScroller::new(0.0, 12.0, 5.0);
        self.ui_scroller.detents = vec![0.0, 3.0, 6.0, 9.0, 12.0];
        self.ui_scroller.position = 3.0;
        self.ui_ribbon = SpringChain::new(
            10,
            Vec2::new(6.3, 4.0),
            SpringConfig::from_response_damping(0.15, 0.78),
        );
    }

    pub(super) fn build_ragdoll_scene(&mut self) {
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.config = self.solver_profile.config();
        self.add_arena_walls();

        let beam_x = 10.6;
        let floor_y = WORLD_HEIGHT - 0.75;
        let mut prev_left: Option<BodyHandle> = None;
        let mut prev_right: Option<BodyHandle> = None;
        for level in 0..4 {
            let y_pillar = floor_y - 0.7 - level as f32 * 1.55;
            let left = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.28, 1.35))
                    .position((beam_x - 1.15, y_pillar))
                    .density(1.4)
                    .friction(0.7)
                    .user_data(0xFF60A5FA),
            );
            let right = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.28, 1.35))
                    .position((beam_x + 1.15, y_pillar))
                    .density(1.4)
                    .friction(0.7)
                    .user_data(0xFF60A5FA),
            );
            let top = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(2.8, 0.26))
                    .position((beam_x, y_pillar - 0.78))
                    .density(1.2)
                    .friction(0.7)
                    .user_data(0xFFFBBF24),
            );
            // Glass-like shatterable core crate inside each bay!
            let crate_h = self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.95, 0.95))
                    .position((beam_x, y_pillar + 0.12))
                    .density(0.9)
                    .user_data(0xFF38BDF8),
            );
            self.shatter_candidates.push(crate_h);

            self.world.add_joint(
                Joint::weld(
                    left,
                    top,
                    Vec2::new(0.0, -0.68),
                    Vec2::new(-1.15, 0.13),
                    0.0,
                )
                .with_break_force(175.0),
            );
            self.world.add_joint(
                Joint::weld(
                    right,
                    top,
                    Vec2::new(0.0, -0.68),
                    Vec2::new(1.15, 0.13),
                    0.0,
                )
                .with_break_force(175.0),
            );
            if let (Some(pl), Some(pr)) = (prev_left, prev_right) {
                self.world.add_joint(
                    Joint::weld(pl, left, Vec2::new(0.0, -0.68), Vec2::new(0.0, 0.68), 0.0)
                        .with_break_force(155.0),
                );
                self.world.add_joint(
                    Joint::weld(pr, right, Vec2::new(0.0, -0.68), Vec2::new(0.0, 0.68), 0.0)
                        .with_break_force(155.0),
                );
            }
            prev_left = Some(left);
            prev_right = Some(right);
        }

        self.spawn_ragdoll(Vec2::new(10.6, 2.0), -10);
        self.spawn_ragdoll(Vec2::new(13.6, 6.2), -11);

        let crane = self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.22))
                .position((6.8, 0.9))
                .user_data(0xFF94A3B8),
        );
        let ball = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.75))
                .position((1.4, 2.0))
                .density(10.5)
                .restitution(0.35)
                .bullet(true)
                .user_data(0xFFEF4444),
        );
        self.world
            .add_joint(Joint::distance(crane, ball, Vec2::ZERO, Vec2::ZERO, 5.5));
    }

    pub(super) fn spawn_ragdoll(&mut self, origin: Vec2, group: i16) {
        let filter = CollisionFilter::ALL.with_group(group);
        let head = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.24))
                .position(origin + Vec2::new(0.0, -0.68))
                .filter(filter)
                .user_data(0xFFFDE68A),
        );
        let torso = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::capsule(0.32, 0.22))
                .position(origin)
                .filter(filter)
                .user_data(0xFFEC4899),
        );
        let arm_l = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::capsule(0.24, 0.1))
                .position(origin + Vec2::new(-0.42, -0.15))
                .angle(0.4)
                .filter(filter)
                .user_data(0xFFF472B6),
        );
        let arm_r = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::capsule(0.24, 0.1))
                .position(origin + Vec2::new(0.42, -0.15))
                .angle(-0.4)
                .filter(filter)
                .user_data(0xFFF472B6),
        );
        let leg_l = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::capsule(0.30, 0.11))
                .position(origin + Vec2::new(-0.18, 0.72))
                .filter(filter)
                .user_data(0xFF818CF8),
        );
        let leg_r = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::capsule(0.30, 0.11))
                .position(origin + Vec2::new(0.18, 0.72))
                .filter(filter)
                .user_data(0xFF818CF8),
        );

        self.world.add_joint(
            Joint::revolute(torso, head, Vec2::new(0.0, -0.45), Vec2::new(0.0, 0.22))
                .with_limits(-0.5, 0.5),
        );
        self.world.add_joint(
            Joint::revolute(torso, arm_l, Vec2::new(-0.22, -0.32), Vec2::new(0.0, -0.22))
                .with_limits(-1.8, 1.8),
        );
        self.world.add_joint(
            Joint::revolute(torso, arm_r, Vec2::new(0.22, -0.32), Vec2::new(0.0, -0.22))
                .with_limits(-1.8, 1.8),
        );
        self.world.add_joint(
            Joint::revolute(torso, leg_l, Vec2::new(-0.15, 0.42), Vec2::new(0.0, -0.28))
                .with_limits(-1.1, 1.1),
        );
        self.world.add_joint(
            Joint::revolute(torso, leg_r, Vec2::new(0.15, 0.42), Vec2::new(0.0, -0.28))
                .with_limits(-1.1, 1.1),
        );
    }

    pub(super) fn build_vehicle_scene(&mut self) {
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.config = self.solver_profile.config();
        self.add_arena_walls();

        // Buoyant water pool on the right side!
        self.world.add_force_field(ForceField::Buoyancy {
            region: Aabb2::new(Vec2::new(10.8, 7.2), Vec2::new(15.6, 9.6)),
            fluid_density: 2.2,
            linear_drag: 2.4,
            angular_drag: 2.0,
        });

        let terrain_mat = PhysicsMaterial::new(0.0, 0.1, 0.95);
        for (x, y, w, h, ang) in [(4.8, 9.1, 3.2, 0.28, -0.22), (8.2, 8.95, 2.8, 0.28, 0.18)] {
            self.world.add_body(
                BodyBuilder::static_body(Shape2D::rect(w, h))
                    .position((x, y))
                    .angle(ang)
                    .material(terrain_mat)
                    .user_data(0xFF334155),
            );
        }
        for i in 0..3 {
            self.world.add_body(
                BodyBuilder::static_body(Shape2D::circle(0.24))
                    .position((5.4 + i as f32 * 1.9, 9.32))
                    .material(terrain_mat)
                    .user_data(0xFF475569),
            );
        }

        let car_filter = CollisionFilter::ALL.with_group(-20);
        let chassis = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::rect(2.3, 0.56))
                .position((2.3, 7.1))
                .density(1.5)
                .filter(car_filter)
                .can_sleep(false)
                .user_data(0xFF38BDF8),
        );
        let wheel_mat = PhysicsMaterial::new(2.2, 0.2, 1.4);
        let wheel_l = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.42))
                .position((1.45, 7.75))
                .material(wheel_mat)
                .filter(car_filter)
                .can_sleep(false)
                .user_data(0xFFF59E0B),
        );
        let wheel_r = self.world.add_body(
            BodyBuilder::dynamic(Shape2D::circle(0.42))
                .position((3.15, 7.75))
                .material(wheel_mat)
                .filter(car_filter)
                .can_sleep(false)
                .user_data(0xFFF59E0B),
        );

        self.world.add_joint(
            Joint::wheel(
                chassis,
                wheel_l,
                Vec2::new(-0.85, 0.55),
                Vec2::ZERO,
                Vec2::Y,
                5.5,
                0.75,
            )
            .with_motor(11.0, 55.0),
        );
        self.world.add_joint(
            Joint::wheel(
                chassis,
                wheel_r,
                Vec2::new(0.85, 0.55),
                Vec2::ZERO,
                Vec2::Y,
                5.5,
                0.75,
            )
            .with_motor(11.0, 55.0),
        );

        // Floating wooden crates bobbing in the water pool!
        for i in 0..4 {
            self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.62, 0.62))
                    .position((11.6 + i as f32 * 0.95, 7.4 - (i % 2) as f32 * 0.6))
                    .density(0.65)
                    .can_sleep(false)
                    .user_data(0xFF34D399),
            );
        }
    }

    pub(super) fn build_character_scene(&mut self) {
        self.world.gravity = Vec2::new(0.0, self.gravity_y);
        self.world.config = self.solver_profile.config();
        self.add_arena_walls();

        // Walkable slope ramps + floating platforms
        for (x, y, w, h, ang) in [
            (4.4, 8.6, 4.2, 0.35, -0.32),
            (9.2, 7.8, 3.2, 0.35, 0.0),
            (13.2, 6.4, 3.4, 0.35, -0.25),
            (3.2, 5.2, 3.2, 0.3, 0.0),
        ] {
            self.world.add_body(
                BodyBuilder::static_body(Shape2D::rect(w, h))
                    .position((x, y))
                    .angle(ang)
                    .user_data(0xFF334155),
            );
        }

        // Moving kinematic elevator platform
        let plat_origin = Vec2::new(7.4, 5.4);
        let plat = self.world.add_body(
            BodyBuilder::kinematic(Shape2D::rect(2.4, 0.28))
                .position(plat_origin)
                .user_data(0xFF0EA5E9),
        );
        self.moving_platform = Some((plat, plat_origin));

        // Pushable dynamic physics crates on platforms
        for (x, y, col) in [
            (9.4, 7.1, 0xFFF59E0B),
            (10.1, 7.1, 0xFFFB7185),
            (9.75, 6.4, 0xFFA78BFA),
            (2.8, 4.6, 0xFF34D399),
        ] {
            self.world.add_body(
                BodyBuilder::dynamic(Shape2D::rect(0.58, 0.58))
                    .position((x, y))
                    .density(0.8)
                    .user_data(col),
            );
        }

        self.character = Some(CharacterController2D::new(
            Shape2D::capsule(0.36, 0.26),
            Vec2::new(2.0, 8.0),
        ));
    }

    pub(super) fn build_laser_gravity_scene(&mut self) {
        self.world.gravity = Vec2::ZERO;
        self.world.config = self.solver_profile.config();
        self.add_arena_walls();

        let center = Vec2::new(WORLD_WIDTH * 0.54, WORLD_HEIGHT * 0.5);
        let mu = 48.0_f32;
        // Infinite arena-wide Newtonian gravity so planets & comets NEVER escape gravity cutoff!
        self.world.add_force_field(ForceField::NewtonianGravity {
            center,
            mu,
            min_radius: 0.9,
            max_radius: 50.0,
        });

        // Luminous central star
        self.world.add_body(
            BodyBuilder::static_body(Shape2D::circle(0.62))
                .position(center)
                .restitution(0.95)
                .user_data(0xFFFBBF24),
        );

        // Five well-spaced, collision-free Keplerian planets & orbital mirrors (min radial gap 0.80m!)
        let specs: &[(f32, f32, f32, u8, u32)] = &[
            // (orbit_radius, start_angle, ecc_scale, shape_kind, color)
            (1.45, 0.3, 1.00, 0, 0xFF38BDF8), // Inner cyan planet
            (2.25, 2.0, 1.00, 1, 0xFFA78BFA), // Violet crystal hexagon
            (3.05, 3.8, 1.00, 2, 0xFF34D399), // Emerald orbital mirror relay
            (3.85, 5.2, 1.02, 0, 0xFFF472B6), // Rose outer planet (gentle ellipse)
            (4.55, 1.1, 0.99, 2, 0xFFFB923C), // Amber outer reflector
        ];

        for &(r, ang, ecc, kind, color) in specs {
            let pos = center + Vec2::new(ang.cos() * r, ang.sin() * r);
            let tangent = Vec2::new(-ang.sin(), ang.cos());
            let v_orbit = (mu / r).sqrt() * ecc;
            let shape = match kind {
                0 => Shape2D::circle(0.27),
                1 => Shape2D::regular_polygon(6, 0.30),
                _ => Shape2D::rect(0.72, 0.18),
            };
            let h = self.world.add_body(
                BodyBuilder::dynamic(shape)
                    .position(pos)
                    .angle(ang)
                    .linear_velocity(tangent * v_orbit)
                    .angular_velocity(0.9)
                    .linear_damping(0.0)
                    .angular_damping(0.0)
                    .restitution(0.95)
                    .can_sleep(false)
                    .user_data(color as u64),
            );
            let mut trail = VecDeque::with_capacity(72);
            trail.push_back(pos);
            self.orbit_trails.push((h, trail, color));
        }

        self.laser_origin = Vec2::new(0.55, WORLD_HEIGHT * 0.5);
    }
}
