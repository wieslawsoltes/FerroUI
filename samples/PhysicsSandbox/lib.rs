//! `PhysicsSandbox` — Interactive FerroUI showcase for `ferroui-physics`.
//!
//! Demonstrates rigid-body games, hardware-accelerated `wgpu` GPU compute particles (with live
//! GPU vs CPU SIMD toggle), analytical UI springs & rubber-band inertia scrolling, articulated
//! ragdolls with breakable welds, suspension vehicles, and multi-bounce raycasting.

pub mod scenes;
pub mod viewport;

use crate::scenes::{SandboxEngine, SceneKind, SpringPreset, ToolMode};
use crate::viewport::PhysicsViewport;
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_controls::Dock;
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Builds the full interactive `PhysicsSandbox` application element tree.
pub fn physics_sandbox() -> Element {
    let engine = Rc::new(RefCell::new(SandboxEngine::new()));

    let active_scene = signal(SceneKind::Playground);
    let active_tool = signal(ToolMode::Drag);
    let active_preset = signal(SpringPreset::Bouncy);
    let paused = signal(false);
    let prefer_gpu = signal(engine.borrow().gpu_available());
    let gravity = signal(9.81_f64);
    let time_scale = signal(1.0_f64);
    let restitution = signal(0.35_f64);
    let show_aabbs = signal(false);
    let show_contacts = signal(false);
    let show_velocities = signal(false);

    let telemetry_backend = signal(engine.borrow().telemetry.backend_label.clone());
    let telemetry_step_ms = signal(0.0_f32);
    let telemetry_entities = signal(String::from("Ready"));

    // Synchronize slider signals into the simulation engine
    {
        let eng = engine.clone();
        effect(move || {
            let g = gravity.get() as f32;
            let ts = time_scale.get() as f32;
            let r = restitution.get() as f32;
            let mut sim = eng.borrow_mut();
            sim.gravity_y = g;
            sim.time_scale = ts;
            sim.restitution = r;
        });
    }

    // Scene switcher top bar
    let mut scene_bar = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(6.0);
    for kind in SceneKind::ALL {
        let eng = engine.clone();
        scene_bar = scene_bar.child(
            button()
                .content(move || {
                    if active_scene.get() == kind {
                        format!("● {}", kind.title())
                    } else {
                        kind.title().to_string()
                    }
                })
                .on_click(move |_| {
                    eng.borrow_mut().load_scene(kind);
                    active_scene.set(kind);
                }),
        );
    }

    // Spawn/interaction tool bar
    let mut tool_bar = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
    for tool in ToolMode::ALL {
        let eng = engine.clone();
        tool_bar = tool_bar.child(
            button()
                .content(move || {
                    if active_tool.get() == tool {
                        format!("[{}]", tool.label())
                    } else {
                        tool.label().to_string()
                    }
                })
                .on_click(move |_| {
                    eng.borrow_mut().tool = tool;
                    active_tool.set(tool);
                }),
        );
    }

    // UI Spring preset bar
    let mut preset_bar = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
    for preset in SpringPreset::ALL {
        let eng = engine.clone();
        preset_bar = preset_bar.child(
            button()
                .content(move || {
                    if active_preset.get() == preset {
                        format!("[{}]", preset.label())
                    } else {
                        preset.label().to_string()
                    }
                })
                .on_click(move |_| {
                    eng.borrow_mut().set_spring_preset(preset);
                    active_preset.set(preset);
                }),
        );
    }

    // Right inspector sidebar
    let eng_pause = engine.clone();
    let eng_reset = engine.clone();
    let eng_burst = engine.clone();
    let eng_gpu = engine.clone();
    let eng_aabb = engine.clone();
    let eng_contacts = engine.clone();
    let eng_vel = engine.clone();
    let gpu_name = engine
        .borrow()
        .gpu_adapter_name()
        .unwrap_or("CPU SIMD Fallback")
        .to_string();

    let sidebar = border()
        .dock_panel_dock(Dock::Right)
        .width(280.0)
        .padding(12.0)
        .child(
            stack_panel()
                .spacing(10.0)
                .child(text_block().text("FerroUI Physics").font_size(18.0))
                .child(text_block().text(format!("Hardware: {gpu_name}")).font_size(11.0))
                .child(
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .spacing(6.0)
                        .child(
                            button()
                                .content(move || if paused.get() { "Resume" } else { "Pause" })
                                .on_click(move |_| {
                                    let next = !paused.get_untracked();
                                    eng_pause.borrow_mut().paused = next;
                                    paused.set(next);
                                }),
                        )
                        .child(button().content("Reset").on_click(move |_| {
                            let sc = active_scene.get_untracked();
                            eng_reset.borrow_mut().load_scene(sc);
                        }))
                        .child(button().content("Burst!").on_click(move |_| {
                            eng_burst.borrow_mut().spawn_quick_burst();
                        })),
                )
                .child(
                    button()
                        .horizontal_alignment(HorizontalAlignment::Stretch)
                        .content(move || {
                            if prefer_gpu.get() {
                                "Compute: GPU (wgpu) → Switch CPU SIMD"
                            } else {
                                "Compute: CPU SIMD → Switch GPU (wgpu)"
                            }
                        })
                        .on_click(move |_| {
                            let next = !prefer_gpu.get_untracked();
                            eng_gpu.borrow_mut().prefer_gpu = next;
                            prefer_gpu.set(next);
                        }),
                )
                .child(text_block().text("Pointer Tool"))
                .child(tool_bar)
                .child(text_block().text("UI Spring Preset"))
                .child(preset_bar)
                .child(text_block().text(move || format!("Gravity: {:.1} m/s²", gravity.get())))
                .child(slider().minimum(-15.0).maximum(25.0).value(gravity.two_way()))
                .child(text_block().text(move || format!("Time Scale: {:.2}x", time_scale.get())))
                .child(slider().minimum(0.1).maximum(2.0).value(time_scale.two_way()))
                .child(text_block().text(move || format!("Bounciness: {:.2}", restitution.get())))
                .child(slider().minimum(0.0).maximum(0.98).value(restitution.two_way()))
                .child(text_block().text("Debug Overlays"))
                .child(
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .spacing(4.0)
                        .child(
                            button()
                                .content(move || if show_aabbs.get() { "✓ AABB" } else { "AABB" })
                                .on_click(move |_| {
                                    let v = !show_aabbs.get_untracked();
                                    eng_aabb.borrow_mut().show_aabbs = v;
                                    show_aabbs.set(v);
                                }),
                        )
                        .child(
                            button()
                                .content(move || if show_contacts.get() { "✓ Contacts" } else { "Contacts" })
                                .on_click(move |_| {
                                    let v = !show_contacts.get_untracked();
                                    eng_contacts.borrow_mut().show_contacts = v;
                                    show_contacts.set(v);
                                }),
                        )
                        .child(
                            button()
                                .content(move || if show_velocities.get() { "✓ Vel" } else { "Vel" })
                                .on_click(move |_| {
                                    let v = !show_velocities.get_untracked();
                                    eng_vel.borrow_mut().show_velocities = v;
                                    show_velocities.set(v);
                                }),
                        ),
                ),
        );

    // Top bar
    let top_bar = border()
        .dock_panel_dock(Dock::Top)
        .padding((12.0, 8.0, 12.0, 8.0))
        .child(scene_bar);

    // Bottom live telemetry HUD bar
    let bottom_bar = border()
        .dock_panel_dock(Dock::Bottom)
        .padding((12.0, 6.0, 12.0, 6.0))
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(16.0)
                .child(text_block().text(move || format!("Backend: {}", telemetry_backend.get())))
                .child(text_block().text(move || format!("Step: {:.2} ms", telemetry_step_ms.get())))
                .child(text_block().text(move || telemetry_entities.get()))
                .child(text_block().text(move || active_scene.get().hint())),
        );

    // Main hardware-accelerated physics viewport
    let eng_vp = engine.clone();
    let tick_cb: Rc<dyn Fn(&SandboxEngine)> = Rc::new(move |sim: &SandboxEngine| {
        if telemetry_backend.is_alive() {
            telemetry_backend.set_if_changed(sim.telemetry.backend_label.clone());
            telemetry_step_ms.set(sim.telemetry.step_ms);
            telemetry_entities.set_if_changed(sim.telemetry.entity_summary.clone());
        }
    });

    let vp = El::<PhysicsViewport>::new()
        .with(move |control| {
            control.bind_engine(eng_vp.clone(), tick_cb.clone());
        });

    dock_panel()
        .child(top_bar)
        .child(bottom_bar)
        .child(sidebar)
        .child(vp)
        .into()
}

#[cfg(test)]
mod tests {
    use super::scenes::{SandboxEngine, SceneKind, ToolMode};
    use ferroui_physics::Vec2;

    #[test]
    fn all_six_scenes_simulate_cleanly_with_interactions() {
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

            // Assert every simulated body/particle stays finite
            for b in engine.world.bodies() {
                assert!(b.position.is_finite(), "{kind:?}: non-finite body position");
                assert!(b.linear_velocity.is_finite(), "{kind:?}: non-finite body velocity");
            }
            for p in &engine.gpu_particles_view {
                assert!(p.pos[0].is_finite() && p.pos[1].is_finite(), "{kind:?}: non-finite particle");
            }
            assert!(!engine.telemetry.backend_label.is_empty(), "{kind:?}: missing telemetry");
        }
    }
}
