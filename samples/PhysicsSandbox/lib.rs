//! **FerroUI Physics Studio** — Professional hardware-accelerated physics showcase built on
//! `ferroui-physics` and `ferroui-view`.

pub mod live_ui_tree;
pub mod scenes;
pub mod viewport;

use crate::scenes::{SandboxEngine, SceneKind, SolverProfile, SpringPreset, ToolMode};
use crate::viewport::PhysicsViewport;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_controls::Dock;
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Reusable studio inspector section card with dark recessed backdrop and crisp hairline border.
fn studio_card(title: &'static str, body: impl IntoElement) -> Element {
    border()
        .background("#141B2D")
        .border_brush("#24314A")
        .border_thickness(1.0)
        .corner_radius(8.0)
        .padding(10.0)
        .child(
            stack_panel()
                .spacing(7.0)
                .child(text_block().text(title).font_size(11.0).foreground("#7DD3FC"))
                .child(body),
        )
        .into()
}

/// Builds the full **FerroUI Physics Studio** application layout.
pub fn physics_sandbox() -> Element {
    let engine = Rc::new(RefCell::new(SandboxEngine::new()));

    let active_scene = signal(SceneKind::Playground);
    let live_ui_mode = signal(false);
    let active_tool = signal(ToolMode::Drag);
    let active_preset = signal(SpringPreset::Bouncy);
    let active_solver = signal(SolverProfile::Game);
    let active_pcount = signal(4096usize);
    let paused = signal(false);
    let prefer_gpu = signal(engine.borrow().gpu_available());

    let gravity = signal(9.81_f64);
    let time_scale = signal(1.0_f64);
    let restitution = signal(0.35_f64);
    let wind_x = signal(3.2_f64);
    let timeline = signal(1.0_f64);

    let show_aabbs = signal(false);
    let show_contacts = signal(false);
    let show_velocities = signal(false);
    let show_joints = signal(true);
    let show_hud = signal(true);

    let telemetry_backend = signal(engine.borrow().telemetry.backend_label.clone());
    let telemetry_step_ms = signal(0.0_f32);
    let telemetry_breakdown = signal(String::from("Ready"));
    let telemetry_entities = signal(String::from("Ready"));
    let telemetry_history = signal(String::from("Frame 0/0"));

    // Reactive sync of sliders into engine
    {
        let eng = engine.clone();
        effect(move || {
            let mut sim = eng.borrow_mut();
            sim.gravity_y = gravity.get() as f32;
            sim.time_scale = time_scale.get() as f32;
            sim.restitution = restitution.get() as f32;
            sim.wind_x = wind_x.get() as f32;
        });
    }

    // -------------------------------------------------------------------------
    // 1. Top Studio Header & Transport Bar
    // -------------------------------------------------------------------------
    let mut scene_tabs = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
    for kind in SceneKind::ALL {
        let eng = engine.clone();
        scene_tabs = scene_tabs.child(
            button()
                .padding((8.0, 5.0, 8.0, 5.0))
                .corner_radius(6.0)
                .background(move || if !live_ui_mode.get() && active_scene.get() == kind { "#0284C7" } else { "#182235" })
                .foreground("#F1F5F9")
                .content(kind.title())
                .on_click(move |_| {
                    live_ui_mode.set(false);
                    eng.borrow_mut().load_scene(kind);
                    active_scene.set(kind);
                    paused.set(false);
                }),
        );
    }
    scene_tabs = scene_tabs.child(
        button()
            .padding((9.0, 5.0, 9.0, 5.0))
            .corner_radius(6.0)
            .background(move || if live_ui_mode.get() { "#059669" } else { "#14332A" })
            .foreground("#ECFDF5")
            .content("⚡ Live UI Tree")
            .on_click(move |_| {
                live_ui_mode.set(true);
            }),
    );

    let eng_back = engine.clone();
    let eng_pause = engine.clone();
    let eng_fwd = engine.clone();
    let eng_reset = engine.clone();
    let eng_burst = engine.clone();

    let transport = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(4.0)
        .child(
            button()
                .padding((8.0, 5.0, 8.0, 5.0))
                .corner_radius(6.0)
                .background("#1E293B")
                .foreground("#E2E8F0")
                .content("◀")
                .on_click(move |_| {
                    eng_back.borrow_mut().step_back_history();
                    paused.set(true);
                }),
        )
        .child(
            button()
                .padding((10.0, 5.0, 10.0, 5.0))
                .corner_radius(6.0)
                .background(move || if paused.get() { "#059669" } else { "#2563EB" })
                .foreground("#FFFFFF")
                .content(move || if paused.get() { "▶ Play" } else { "⏸ Pause" })
                .on_click(move |_| {
                    let next = !paused.get_untracked();
                    eng_pause.borrow_mut().paused = next;
                    paused.set(next);
                }),
        )
        .child(
            button()
                .padding((8.0, 5.0, 8.0, 5.0))
                .corner_radius(6.0)
                .background("#1E293B")
                .foreground("#E2E8F0")
                .content("▶|")
                .on_click(move |_| {
                    eng_fwd.borrow_mut().step_forward_one();
                }),
        )
        .child(
            button()
                .padding((9.0, 5.0, 9.0, 5.0))
                .corner_radius(6.0)
                .background("#1E293B")
                .foreground("#E2E8F0")
                .content("↺ Reset")
                .on_click(move |_| {
                    let sc = active_scene.get_untracked();
                    eng_reset.borrow_mut().load_scene(sc);
                    paused.set(false);
                }),
        )
        .child(
            button()
                .padding((9.0, 5.0, 9.0, 5.0))
                .corner_radius(6.0)
                .background("#7C3AED")
                .foreground("#FFFFFF")
                .content("✦ Burst")
                .on_click(move |_| {
                    eng_burst.borrow_mut().spawn_quick_burst();
                }),
        );

    let gpu_badge_text = if engine.borrow().gpu_available() { "METAL GPU" } else { "CPU SIMD" };
    let header = border()
        .dock_panel_dock(Dock::Top)
        .background("#0D1321")
        .border_brush("#1E293B")
        .border_thickness((0.0, 0.0, 0.0, 1.0))
        .padding((12.0, 8.0, 12.0, 8.0))
        .child(
            dock_panel()
                .child(
                    stack_panel()
                        .dock_panel_dock(Dock::Left)
                        .orientation(Orientation::Horizontal)
                        .vertical_alignment(VerticalAlignment::Center)
                        .spacing(8.0)
                        .margin((0.0, 0.0, 14.0, 0.0))
                        .child(text_block().text("⚡ FERRO PHYSICS").font_size(14.0).foreground("#F8FAFC"))
                        .child(
                            border()
                                .background("#064E3B")
                                .border_brush("#10B981")
                                .border_thickness(1.0)
                                .corner_radius(4.0)
                                .padding((6.0, 2.0, 6.0, 2.0))
                                .child(text_block().text(gpu_badge_text).font_size(10.0).foreground("#6EE7B7")),
                        ),
                )
                .child(transport.dock_panel_dock(Dock::Right))
                .child(scene_tabs),
        );

    // -------------------------------------------------------------------------
    // 2. Left Vertical Studio Tool Rail
    // -------------------------------------------------------------------------
    let mut tool_rail_stack = stack_panel().spacing(6.0).child(
        text_block()
            .text("TOOLS")
            .font_size(10.0)
            .foreground("#64748B")
            .horizontal_alignment(HorizontalAlignment::Center),
    );
    for tool in ToolMode::ALL {
        let eng = engine.clone();
        tool_rail_stack = tool_rail_stack.child(
            button()
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .padding((4.0, 8.0, 4.0, 8.0))
                .corner_radius(6.0)
                .background(move || if active_tool.get() == tool { "#0284C7" } else { "#141B2D" })
                .foreground("#F1F5F9")
                .content(tool.label())
                .on_click(move |_| {
                    eng.borrow_mut().tool = tool;
                    active_tool.set(tool);
                }),
        );
    }

    let left_rail = border()
        .dock_panel_dock(Dock::Left)
        .width(88.0)
        .background("#0D1321")
        .border_brush("#1E293B")
        .border_thickness((0.0, 0.0, 1.0, 0.0))
        .padding(8.0)
        .clip_to_bounds(true)
        .child(tool_rail_stack);

    // -------------------------------------------------------------------------
    // 3. Right Studio Inspector Dock (Scrollable Cards)
    // -------------------------------------------------------------------------
    let gpu_name = engine
        .borrow()
        .gpu_adapter_name()
        .unwrap_or("CPU SIMD Fallback")
        .to_string();

    let eng_gpu = engine.clone();
    let mut pcount_row = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
    for &(cnt, label) in &[(2048usize, "2K"), (4096, "4K"), (8192, "8K")] {
        let eng = engine.clone();
        pcount_row = pcount_row.child(
            button()
                .padding((10.0, 4.0, 10.0, 4.0))
                .corner_radius(5.0)
                .background(move || if active_pcount.get() == cnt { "#0284C7" } else { "#1E293B" })
                .foreground("#F1F5F9")
                .content(label)
                .on_click(move |_| {
                    eng.borrow_mut().set_particle_count(cnt);
                    active_pcount.set(cnt);
                }),
        );
    }

    let mut solver_row = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
    for prof in SolverProfile::ALL {
        let eng = engine.clone();
        solver_row = solver_row.child(
            button()
                .padding((8.0, 4.0, 8.0, 4.0))
                .corner_radius(5.0)
                .background(move || if active_solver.get() == prof { "#0284C7" } else { "#1E293B" })
                .foreground("#F1F5F9")
                .content(prof.label())
                .on_click(move |_| {
                    eng.borrow_mut().set_solver_profile(prof);
                    active_solver.set(prof);
                }),
        );
    }

    let compute_card = studio_card(
        "HARDWARE & SOLVER",
        stack_panel()
            .spacing(6.0)
            .child(text_block().text(format!("Adapter: {gpu_name}")).font_size(11.0).foreground("#CBD5E1"))
            .child(
                button()
                    .horizontal_alignment(HorizontalAlignment::Stretch)
                    .padding((8.0, 6.0, 8.0, 6.0))
                    .corner_radius(6.0)
                    .background(move || if prefer_gpu.get() { "#0369A1" } else { "#334155" })
                    .foreground("#FFFFFF")
                    .content(move || {
                        if prefer_gpu.get() {
                            "Backend: GPU Compute (wgpu)"
                        } else {
                            "Backend: CPU SIMD (NEON/AVX2)"
                        }
                    })
                    .on_click(move |_| {
                        let next = !prefer_gpu.get_untracked();
                        eng_gpu.borrow_mut().prefer_gpu = next;
                        prefer_gpu.set(next);
                    }),
            )
            .child(text_block().text("GPU Fluid Particles").font_size(10.5).foreground("#94A3B8"))
            .child(pcount_row)
            .child(text_block().text("Rigid Solver Profile").font_size(10.5).foreground("#94A3B8"))
            .child(solver_row),
    );

    let eng_scrub = engine.clone();
    let timeline_card = studio_card(
        "TIME-TRAVEL & SPEED",
        stack_panel()
            .spacing(5.0)
            .child(text_block().text(move || format!("History Scrub: {}", telemetry_history.get())).font_size(11.0).foreground("#CBD5E1"))
            .child(
                slider()
                    .minimum(0.0)
                    .maximum(1.0)
                    .value(timeline.two_way())
                    .on_changed(ferroui_controls::primitives::RangeBase::value_property(), move |v| {
                        if paused.get_untracked() {
                            eng_scrub.borrow_mut().scrub_history(*v as f32);
                        }
                    }),
            )
            .child(text_block().text(move || format!("Time Scale: {:.2}x", time_scale.get())).font_size(11.0).foreground("#CBD5E1"))
            .child(slider().minimum(0.05).maximum(2.0).value(time_scale.two_way())),
    );

    let env_card = studio_card(
        "WORLD ENVIRONMENT",
        stack_panel()
            .spacing(5.0)
            .child(text_block().text(move || format!("Gravity: {:.1} m/s²", gravity.get())).font_size(11.0).foreground("#CBD5E1"))
            .child(slider().minimum(-15.0).maximum(25.0).value(gravity.two_way()))
            .child(text_block().text(move || format!("Bounciness: {:.2}", restitution.get())).font_size(11.0).foreground("#CBD5E1"))
            .child(slider().minimum(0.0).maximum(0.98).value(restitution.two_way()))
            .child(text_block().text(move || format!("Wind / Gust: {:.1} m/s²", wind_x.get())).font_size(11.0).foreground("#CBD5E1"))
            .child(slider().minimum(-12.0).maximum(12.0).value(wind_x.two_way())),
    );

    let mut preset_row1 = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
    let mut preset_row2 = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
    for (idx, preset) in SpringPreset::ALL.iter().copied().enumerate() {
        let eng = engine.clone();
        let btn = button()
            .padding((8.0, 4.0, 8.0, 4.0))
            .corner_radius(5.0)
            .background(move || if active_preset.get() == preset { "#7C3AED" } else { "#1E293B" })
            .foreground("#F1F5F9")
            .content(preset.label())
            .on_click(move |_| {
                eng.borrow_mut().set_spring_preset(preset);
                active_preset.set(preset);
            });
        if idx < 3 {
            preset_row1 = preset_row1.child(btn);
        } else {
            preset_row2 = preset_row2.child(btn);
        }
    }
    let spring_card = studio_card(
        "UI SPRING PRESET",
        stack_panel().spacing(5.0).child(preset_row1).child(preset_row2),
    );

    let eng_aabb = engine.clone();
    let eng_contacts = engine.clone();
    let eng_vel = engine.clone();
    let eng_joints = engine.clone();
    let eng_hud = engine.clone();

    let debug_card = studio_card(
        "VIEWPORT OVERLAYS",
        stack_panel()
            .spacing(5.0)
            .child(
                stack_panel()
                    .orientation(Orientation::Horizontal)
                    .spacing(4.0)
                    .child(
                        button()
                            .padding((8.0, 4.0, 8.0, 4.0))
                            .corner_radius(5.0)
                            .background(move || if show_aabbs.get() { "#0284C7" } else { "#1E293B" })
                            .foreground("#F1F5F9")
                            .content("BVH AABB")
                            .on_click(move |_| {
                                let v = !show_aabbs.get_untracked();
                                eng_aabb.borrow_mut().show_aabbs = v;
                                show_aabbs.set(v);
                            }),
                    )
                    .child(
                        button()
                            .padding((8.0, 4.0, 8.0, 4.0))
                            .corner_radius(5.0)
                            .background(move || if show_contacts.get() { "#DB2777" } else { "#1E293B" })
                            .foreground("#F1F5F9")
                            .content("Contacts")
                            .on_click(move |_| {
                                let v = !show_contacts.get_untracked();
                                eng_contacts.borrow_mut().show_contacts = v;
                                show_contacts.set(v);
                            }),
                    )
                    .child(
                        button()
                            .padding((8.0, 4.0, 8.0, 4.0))
                            .corner_radius(5.0)
                            .background(move || if show_velocities.get() { "#0891B2" } else { "#1E293B" })
                            .foreground("#F1F5F9")
                            .content("Vel")
                            .on_click(move |_| {
                                let v = !show_velocities.get_untracked();
                                eng_vel.borrow_mut().show_velocities = v;
                                show_velocities.set(v);
                            }),
                    ),
            )
            .child(
                stack_panel()
                    .orientation(Orientation::Horizontal)
                    .spacing(4.0)
                    .child(
                        button()
                            .padding((8.0, 4.0, 8.0, 4.0))
                            .corner_radius(5.0)
                            .background(move || if show_joints.get() { "#CA8A04" } else { "#1E293B" })
                            .foreground("#F1F5F9")
                            .content("Joints")
                            .on_click(move |_| {
                                let v = !show_joints.get_untracked();
                                eng_joints.borrow_mut().show_joints = v;
                                show_joints.set(v);
                            }),
                    )
                    .child(
                        button()
                            .padding((8.0, 4.0, 8.0, 4.0))
                            .corner_radius(5.0)
                            .background(move || if show_hud.get() { "#059669" } else { "#1E293B" })
                            .foreground("#F1F5F9")
                            .content("HUD Graph")
                            .on_click(move |_| {
                                let v = !show_hud.get_untracked();
                                eng_hud.borrow_mut().show_hud_graph = v;
                                show_hud.set(v);
                            }),
                    ),
            ),
    );

    let inspector = border()
        .dock_panel_dock(Dock::Right)
        .width(308.0)
        .background("#0D1321")
        .border_brush("#1E293B")
        .border_thickness((1.0, 0.0, 0.0, 0.0))
        .clip_to_bounds(true)
        .child(
            scroll_viewer().child(
                stack_panel()
                    .margin(10.0)
                    .spacing(9.0)
                    .child(compute_card)
                    .child(timeline_card)
                    .child(env_card)
                    .child(spring_card)
                    .child(debug_card),
            ),
        );

    // -------------------------------------------------------------------------
    // 4. Bottom Precision Telemetry Bar
    // -------------------------------------------------------------------------
    let status_bar = border()
        .dock_panel_dock(Dock::Bottom)
        .background("#080C14")
        .border_brush("#1E293B")
        .border_thickness((0.0, 1.0, 0.0, 0.0))
        .padding((12.0, 6.0, 12.0, 6.0))
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(16.0)
                .child(text_block().text(move || telemetry_backend.get()).font_size(11.0).foreground("#38BDF8"))
                .child(text_block().text(move || format!("Step {:.2} ms", telemetry_step_ms.get())).font_size(11.0).foreground("#34D399"))
                .child(text_block().text(move || telemetry_breakdown.get()).font_size(11.0).foreground("#94A3B8"))
                .child(text_block().text(move || telemetry_entities.get()).font_size(11.0).foreground("#CBD5E1"))
                .child(text_block().text(move || active_scene.get().hint()).font_size(11.0).foreground("#64748B")),
        );

    // -------------------------------------------------------------------------
    // 5. Center Hardware-Rendered Studio Viewport
    // -------------------------------------------------------------------------
    let eng_vp = engine.clone();
    let tick_cb: Rc<dyn Fn(&SandboxEngine)> = Rc::new(move |sim: &SandboxEngine| {
        if telemetry_backend.is_alive() {
            telemetry_backend.set_if_changed(sim.telemetry.backend_label.clone());
            telemetry_step_ms.set(sim.telemetry.step_ms);
            telemetry_breakdown.set_if_changed(sim.telemetry.breakdown.clone());
            telemetry_entities.set_if_changed(sim.telemetry.entity_summary.clone());
            if sim.telemetry.history_len > 0 {
                telemetry_history.set_if_changed(format!(
                    "Frame {}/{}",
                    sim.telemetry.history_frame + 1,
                    sim.telemetry.history_len
                ));
                if !sim.paused {
                    timeline.set_if_changed(sim.history.normalized_cursor() as f64);
                }
            }
        }
    });

    let center_stage = border().clip_to_bounds(true).child(show_else(
        move || live_ui_mode.get(),
        move || live_ui_tree::live_ui_tree_showcase(),
        move || {
            let eng_vp = eng_vp.clone();
            let tick_cb = tick_cb.clone();
            El::<PhysicsViewport>::new()
                .with(move |control| {
                    control.bind_engine(eng_vp.clone(), tick_cb.clone());
                })
                .into_element()
        },
    ));

    dock_panel()
        .background("#090D16")
        .child(header)
        .child(status_bar)
        .child(left_rail)
        .child(inspector)
        .child(center_stage)
        .into()
}

#[cfg(test)]
mod tests {
    use super::scenes::{SandboxEngine, SceneKind, ToolMode};
    use ferroui_physics::Vec2;

    #[test]
    fn all_eight_studio_scenes_simulate_cleanly_with_interactions() {
        let mut engine = SandboxEngine::new();
        for kind in SceneKind::ALL {
            engine.load_scene(kind);
            for frame in 0..45 {
                if frame == 10 {
                    engine.on_pointer_pressed(Vec2::new(6.0, 4.0), false);
                } else if frame == 18 {
                    engine.on_pointer_moved(Vec2::new(8.0, 3.5));
                } else if frame == 25 {
                    engine.on_pointer_released();
                } else if frame == 30 {
                    engine.tool = ToolMode::Explosion;
                    engine.on_pointer_pressed(Vec2::new(7.5, 5.0), true);
                    engine.on_pointer_released();
                    engine.tool = ToolMode::Drag;
                }
                engine.step(1.0 / 60.0);
            }

            for b in engine.world.bodies() {
                assert!(b.position.is_finite(), "{kind:?}: non-finite body position");
                assert!(b.linear_velocity.is_finite(), "{kind:?}: non-finite body velocity");
            }
            for p in &engine.gpu_particles_view {
                assert!(p.pos[0].is_finite() && p.pos[1].is_finite(), "{kind:?}: non-finite particle");
            }
            for sb in &engine.soft_bodies {
                assert!(sb.center().is_finite(), "{kind:?}: non-finite softbody");
            }
            if let Some(cloth) = engine.cloth.as_ref() {
                for n in &cloth.nodes {
                    assert!(n.pos.is_finite(), "{kind:?}: non-finite cloth node");
                }
            }
            assert!(!engine.telemetry.backend_label.is_empty(), "{kind:?}: missing telemetry");
        }
    }

    #[test]
    fn laser_gravity_orbits_remain_stable_and_bounded() {
        let mut engine = SandboxEngine::new();
        engine.load_scene(SceneKind::LaserGravity);

        let star_center = Vec2::new(16.0 * 0.54, 10.0 * 0.5);
        let initial_radii: Vec<(ferroui_physics::BodyHandle, f32)> = engine
            .world
            .bodies()
            .filter(|b| b.is_dynamic())
            .map(|b| (b.handle, (b.position - star_center).length()))
            .collect();
        assert_eq!(initial_radii.len(), 5, "expected 5 orbital bodies");

        // Simulate 5 full seconds (300 frames) and verify every planet holds a stable orbit
        for _ in 0..300 {
            engine.step(1.0 / 60.0);
        }

        for (handle, r0) in initial_radii {
            let body = engine.world.body(handle).expect("orbital body must exist");
            let r_now = (body.position - star_center).length();
            assert!(
                (r_now - r0).abs() < 0.35,
                "planet drifted out of stable Keplerian orbit: r0={r0:.2}, r_now={r_now:.2}"
            );
            assert!(
                body.position.x > 0.5 && body.position.x < 15.5 && body.position.y > 0.5 && body.position.y < 9.5,
                "planet escaped arena bounds: {:?}",
                body.position
            );
        }
        assert!(
            !engine.laser_segments.is_empty() && engine.laser_segments.len() <= 6,
            "laser should trace clean bounded segments, got {}",
            engine.laser_segments.len()
        );
    }
}
