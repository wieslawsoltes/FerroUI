//! Scene workspace: transport, relevant controls, timeline and viewport.

use crate::icons::{icon_label, Icons};
use crate::scenes::{SandboxEngine, SceneKind, SolverProfile, SpringPreset, ToolMode};
use crate::showcase_style::{subtle_text, surface};
use crate::viewport::PhysicsViewport;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::FontWeight;
use ferroui_controls::{Border, Dock, Slider};
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

fn inspector_card(title: &'static str, body: impl IntoElement) -> El<Border> {
    surface().padding(16.0).child(
        stack_panel()
            .spacing(12.0)
            .child(
                text_block()
                    .text(title)
                    .font_size(16.0)
                    .font_weight(FontWeight::SemiBold),
            )
            .child(body),
    )
}

fn history_slider(
    engine: Rc<RefCell<SandboxEngine>>,
    paused: Signal<bool>,
    timeline: Signal<f64>,
) -> El<Slider> {
    slider()
        .minimum(0.0)
        .maximum(1.0)
        .is_enabled(move || paused.get())
        .value(move || timeline.get())
        .on_changed(
            ferroui_controls::primitives::RangeBase::value_property(),
            move |value| {
                // Telemetry writes the source first. Ignore that echo before borrowing the
                // engine, including when reactive bindings flush after a batch completes.
                // A user edit differs from the source and explicitly writes it back here.
                if *value == timeline.get_untracked() || !paused.get_untracked() {
                    return;
                }
                timeline.set_if_changed(*value);
                let mut sim = engine.borrow_mut();
                if !sim.history.is_empty() {
                    sim.scrub_history(*value as f32);
                }
            },
        )
}

pub(crate) fn scene_workspace(engine: Rc<RefCell<SandboxEngine>>) -> Element {
    let gpu_available = engine.borrow().gpu_available();
    let scene = engine.borrow().scene;
    let controls_open = signal(true);
    let active_scene = signal(engine.borrow().scene);
    let active_tool = signal(engine.borrow().tool);
    let active_preset = signal(engine.borrow().spring_preset);
    let active_solver = signal(engine.borrow().solver_profile);
    let active_pcount = signal(engine.borrow().particle_target_count);
    let paused = signal(engine.borrow().paused);
    let prefer_gpu = signal(engine.borrow().prefer_gpu);

    let gravity = signal(engine.borrow().gravity_y as f64);
    let time_scale = signal(engine.borrow().time_scale as f64);
    let restitution = signal(engine.borrow().restitution as f64);
    let wind_x = signal(engine.borrow().wind_x as f64);
    let timeline = signal(engine.borrow().history.normalized_cursor() as f64);

    let show_aabbs = signal(engine.borrow().show_aabbs);
    let show_contacts = signal(engine.borrow().show_contacts);
    let show_velocities = signal(engine.borrow().show_velocities);
    let show_joints = signal(engine.borrow().show_joints);
    let show_hud = signal(engine.borrow().show_hud_graph);

    let telemetry_backend = signal(engine.borrow().telemetry.backend_label.clone());
    let telemetry_step_ms = signal(0.0_f32);
    let telemetry_breakdown = signal(String::from("Ready"));
    let telemetry_entities = signal(String::from("Ready"));
    let telemetry_history = signal(String::from("Frame 0/0"));

    // Reactive sync of environment sliders into SandboxEngine
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

    let eng_back = engine.clone();
    let eng_pause = engine.clone();
    let eng_fwd = engine.clone();
    let eng_reset = engine.clone();
    let eng_burst = engine.clone();

    let transport = wrap_panel()
        .item_spacing(8.0)
        .line_spacing(8.0)
        .child(
            button()
                .padding((8.0, 5.0, 8.0, 5.0))
                .content(icon_label(Icons::STEP_BACK, "Back", 14.0))
                .is_visible(scene.supports_history())
                .on_click(move |_| {
                    eng_back.borrow_mut().step_back_history();
                    paused.set(true);
                }),
        )
        .child(
            button()
                .padding((10.0, 5.0, 10.0, 5.0))
                .class("accent")
                .content(dynamic(move || {
                    if paused.get() {
                        icon_label(Icons::PLAY, "Play", 14.0)
                    } else {
                        icon_label(Icons::PAUSE, "Pause", 14.0)
                    }
                }))
                .on_click(move |_| {
                    let next = !paused.get_untracked();
                    eng_pause.borrow_mut().paused = next;
                    paused.set(next);
                }),
        )
        .child(
            button()
                .padding((8.0, 5.0, 8.0, 5.0))
                .content(icon_label(Icons::STEP_FWD, "Step", 14.0))
                .is_enabled(move || paused.get())
                .on_click(move |_| {
                    let mut sim = eng_fwd.borrow_mut();
                    sim.paused = true;
                    sim.step_forward_one();
                    paused.set(true);
                }),
        )
        .child(
            button()
                .padding((10.0, 5.0, 10.0, 5.0))
                .content(icon_label(Icons::RESET, "Reset", 14.0))
                .on_click(move |_| {
                    let sc = active_scene.get_untracked();
                    eng_reset.borrow_mut().load_scene(sc);
                    paused.set(false);
                }),
        )
        .child(
            button()
                .padding((10.0, 5.0, 10.0, 5.0))
                .content(icon_label(
                    Icons::SPARKLE,
                    if scene == SceneKind::ClothSoftBody {
                        "Add jelly"
                    } else {
                        "Add bodies"
                    },
                    14.0,
                ))
                .is_visible(scene.supports_body_tools())
                .on_click(move |_| {
                    eng_burst.borrow_mut().spawn_quick_burst();
                }),
        );

    let tool_specs = [
        (ToolMode::Drag, Icons::CURSOR),
        (ToolMode::SpawnBox, Icons::BOX_SHAPE),
        (ToolMode::SpawnCircle, Icons::CIRCLE_SHAPE),
        (ToolMode::SpawnPolygon, Icons::HEX_SHAPE),
        (ToolMode::SpawnJelly, Icons::BLUR),
        (ToolMode::Shatter, Icons::SHATTER),
        (ToolMode::Explosion, Icons::LIGHTNING),
    ];
    let mut tools_row = wrap_panel()
        .item_spacing(6.0)
        .line_spacing(6.0)
        .is_visible(scene.supports_body_tools());
    for (tool, ic) in tool_specs {
        if !scene.supports_tool(tool) {
            continue;
        }
        let eng = engine.clone();
        tools_row = tools_row.child(
            toggle_button()
                .is_checked(move || Some(active_tool.get() == tool))
                .padding((10.0, 6.0, 10.0, 6.0))
                .font_size(14.0)
                .content(icon_label(ic, tool.label(), 13.0))
                .on_click(move |_| {
                    eng.borrow_mut().tool = tool;
                    active_tool.set(tool);
                }),
        );
    }

    let scene_header = border()
        .dock_panel_dock(Dock::Top)
        .padding((24.0, 20.0, 24.0, 16.0))
        .dynamic_resource(
            Border::border_brush_property(),
            "CardStrokeColorDefaultBrush",
        )
        .border_thickness((0.0, 0.0, 0.0, 1.0))
        .child(
            stack_panel()
                .spacing(12.0)
                .child(
                    text_block()
                        .text(scene.title())
                        .font_size(24.0)
                        .font_weight(FontWeight::SemiBold),
                )
                .child(subtle_text().text(scene.hint()))
                .child(
                    transport.child(
                        toggle_button()
                            .is_checked(move || Some(controls_open.get()))
                            .content("Controls")
                            .on_click(move |_| controls_open.set(!controls_open.get_untracked())),
                    ),
                )
                .child(tools_row),
        );

    // Right Inspector (clean stock theme cards)
    let adapter_name = engine
        .borrow()
        .gpu_adapter_name()
        .unwrap_or("CPU SIMD Fallback")
        .to_string();
    let eng_gpu = engine.clone();
    let mut pcount_row = wrap_panel().item_spacing(6.0).line_spacing(6.0);
    for count in [2048usize, 4096, 8192] {
        let eng = engine.clone();
        let label = format!("{}K", count / 1024);
        pcount_row = pcount_row.child(
            toggle_button()
                .is_checked(move || Some(active_pcount.get() == count))
                .padding((10.0, 6.0, 10.0, 6.0))
                .font_size(12.0)
                .content(label)
                .on_click(move |_| {
                    eng.borrow_mut().set_particle_count(count);
                    active_pcount.set(count);
                }),
        );
    }
    let mut solver_row = wrap_panel().item_spacing(6.0).line_spacing(6.0);
    for prof in SolverProfile::ALL {
        let eng = engine.clone();
        solver_row = solver_row.child(
            toggle_button()
                .is_checked(move || Some(active_solver.get() == prof))
                .padding((10.0, 6.0, 10.0, 6.0))
                .font_size(12.0)
                .content(prof.label())
                .on_click(move |_| {
                    eng.borrow_mut().set_solver_profile(prof);
                    active_solver.set(prof);
                }),
        );
    }

    let particle_card = inspector_card(
        "Particle compute",
        stack_panel()
            .spacing(10.0)
            .child(subtle_text().text(adapter_name))
            .child(
                toggle_button()
                    .is_checked(move || Some(prefer_gpu.get()))
                    .is_enabled(gpu_available)
                    .horizontal_alignment(HorizontalAlignment::Stretch)
                    .content(move || {
                        if prefer_gpu.get() {
                            "GPU compute"
                        } else {
                            "CPU simulation"
                        }
                    })
                    .on_click(move |_| {
                        let next = !prefer_gpu.get_untracked() && gpu_available;
                        eng_gpu.borrow_mut().prefer_gpu = next;
                        prefer_gpu.set(next);
                    }),
            )
            .child(text_block().text("Particle count").font_size(14.0))
            .child(pcount_row),
    )
    .is_visible(scene == SceneKind::GpuParticles);
    let solver_card = inspector_card("Solver quality", solver_row).is_visible(!matches!(
        scene,
        SceneKind::UiSprings | SceneKind::GpuParticles | SceneKind::ClothSoftBody
    ));

    let timeline_card = inspector_card(
        "Playback",
        stack_panel()
            .spacing(10.0)
            .child(
                stack_panel()
                    .spacing(8.0)
                    .is_visible(scene.supports_history())
                    .child(subtle_text().text(move || telemetry_history.get()))
                    .child(history_slider(engine.clone(), paused, timeline))
                    .child(subtle_text().text("Pause to scrub recorded body motion.")),
            )
            .child(
                text_block()
                    .text(move || format!("Speed  {:.2}×", time_scale.get()))
                    .font_size(14.0),
            )
            .child(
                slider()
                    .minimum(0.1)
                    .maximum(2.0)
                    .value(time_scale.two_way()),
            ),
    );

    let env_card = inspector_card(
        "Environment",
        stack_panel()
            .spacing(10.0)
            .child(
                stack_panel()
                    .spacing(6.0)
                    .is_visible(scene != SceneKind::LaserGravity)
                    .child(
                        text_block()
                            .text(move || format!("Gravity  {:.1} m/s²", gravity.get()))
                            .font_size(14.0),
                    )
                    .child(
                        slider()
                            .minimum(-15.0)
                            .maximum(25.0)
                            .value(gravity.two_way()),
                    ),
            )
            .child(
                text_block()
                    .text(move || {
                        format!(
                            "{} restitution  {:.2}",
                            if scene == SceneKind::GpuParticles {
                                "Particle"
                            } else {
                                "New body"
                            },
                            restitution.get()
                        )
                    })
                    .font_size(13.0)
                    .is_visible(scene.supports_body_tools() || scene == SceneKind::GpuParticles),
            )
            .child(
                slider()
                    .minimum(0.0)
                    .maximum(0.95)
                    .value(restitution.two_way())
                    .is_visible(scene.supports_body_tools() || scene == SceneKind::GpuParticles),
            )
            .child(
                stack_panel()
                    .spacing(6.0)
                    .is_visible(matches!(
                        scene,
                        SceneKind::GpuParticles | SceneKind::ClothSoftBody
                    ))
                    .child(
                        text_block()
                            .text(move || format!("Wind  {:.1} m/s²", wind_x.get()))
                            .font_size(14.0),
                    )
                    .child(
                        slider()
                            .minimum(-12.0)
                            .maximum(12.0)
                            .value(wind_x.two_way()),
                    ),
            ),
    )
    .is_visible(scene != SceneKind::UiSprings);

    let mut preset_wrap = wrap_panel().item_spacing(5.0).line_spacing(5.0);
    for preset in SpringPreset::ALL {
        let eng = engine.clone();
        preset_wrap = preset_wrap.child(
            toggle_button()
                .is_checked(move || Some(active_preset.get() == preset))
                .padding((10.0, 6.0, 10.0, 6.0))
                .font_size(12.0)
                .content(preset.label())
                .on_click(move |_| {
                    eng.borrow_mut().set_spring_preset(preset);
                    active_preset.set(preset);
                }),
        );
    }
    let spring_card =
        inspector_card("Spring response", preset_wrap).is_visible(scene == SceneKind::UiSprings);

    let make_dbg =
        |label: &'static str, sig: Signal<bool>, setter: fn(&mut SandboxEngine, bool)| {
            let eng = engine.clone();
            check_box()
                .is_checked(move || Some(sig.get()))
                .font_size(14.0)
                .content(label)
                .on_click(move |_| {
                    let next = !sig.get_untracked();
                    setter(&mut eng.borrow_mut(), next);
                    sig.set(next);
                })
        };
    let rigid_overlays = !matches!(
        scene,
        SceneKind::UiSprings | SceneKind::GpuParticles | SceneKind::ClothSoftBody
    );
    let debug_card = inspector_card(
        "Viewport",
        stack_panel()
            .spacing(6.0)
            .child(
                make_dbg("Collision bounds", show_aabbs, |s, v| s.show_aabbs = v)
                    .is_visible(rigid_overlays),
            )
            .child(
                make_dbg("Contact points", show_contacts, |s, v| s.show_contacts = v)
                    .is_visible(rigid_overlays),
            )
            .child(
                make_dbg("Velocities", show_velocities, |s, v| s.show_velocities = v)
                    .is_visible(rigid_overlays),
            )
            .child(
                make_dbg("Joints", show_joints, |s, v| s.show_joints = v)
                    .is_visible(rigid_overlays),
            )
            .child(make_dbg("Performance overlay", show_hud, |s, v| {
                s.show_hud_graph = v
            })),
    );

    let inspector = border()
        .dock_panel_dock(Dock::Right)
        .width(288.0)
        .is_visible(move || controls_open.get())
        .dynamic_resource(
            Border::border_brush_property(),
            "CardStrokeColorDefaultBrush",
        )
        .border_thickness((1.0, 0.0, 0.0, 0.0))
        .padding(12.0)
        .child(
            scroll_viewer().content(
                stack_panel()
                    .spacing(10.0)
                    .child(particle_card)
                    .child(solver_card)
                    .child(timeline_card)
                    .child(env_card)
                    .child(spring_card)
                    .child(debug_card),
            ),
        );

    let status_bar = border()
        .dock_panel_dock(Dock::Bottom)
        .dynamic_resource(
            Border::border_brush_property(),
            "CardStrokeColorDefaultBrush",
        )
        .border_thickness((0.0, 1.0, 0.0, 0.0))
        .padding((16.0, 6.0, 16.0, 6.0))
        .child(
            wrap_panel()
                .item_spacing(16.0)
                .line_spacing(4.0)
                .child(
                    text_block()
                        .text(move || telemetry_backend.get())
                        .font_size(12.0)
                        .font_weight(FontWeight::SemiBold),
                )
                .child(
                    text_block()
                        .text(move || format!("Step {:.2} ms", telemetry_step_ms.get()))
                        .font_size(12.0)
                        .opacity(0.85),
                )
                .child(
                    text_block()
                        .text(move || telemetry_breakdown.get())
                        .font_size(12.0)
                        .opacity(0.72),
                )
                .child(
                    text_block()
                        .text(move || telemetry_entities.get())
                        .font_size(12.0)
                        .opacity(0.8),
                ),
        );

    let eng_vp = engine.clone();
    let tick_cb: Rc<dyn Fn(&SandboxEngine)> = Rc::new(move |sim: &SandboxEngine| {
        if telemetry_backend.is_alive() {
            telemetry_backend.set_if_changed(sim.telemetry.backend_label.clone());
            telemetry_step_ms.set_if_changed(sim.telemetry.step_ms);
            telemetry_breakdown.set_if_changed(sim.telemetry.breakdown.clone());
            telemetry_entities.set_if_changed(sim.telemetry.entity_summary.clone());
            if sim.telemetry.history_len > 0 {
                telemetry_history.set_if_changed(format!(
                    "Frame {}/{}",
                    sim.telemetry.history_frame + 1,
                    sim.telemetry.history_len
                ));
                timeline.set_if_changed(sim.history.normalized_cursor() as f64);
            }
        }
    });

    let viewport_frame = border().clip_to_bounds(true).child(
        El::<PhysicsViewport>::new()
            .with(move |control| {
                control.bind_engine(eng_vp.clone(), tick_cb.clone());
            })
            .into_element(),
    );

    dock_panel()
        .clip_to_bounds(true)
        .child(scene_header)
        .child(status_bar)
        .child(inspector)
        .child(viewport_frame)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_controls::primitives::RangeBase;

    #[test]
    fn paused_timeline_sync_suppresses_feedback_but_user_changes_scrub() {
        let engine = Rc::new(RefCell::new(SandboxEngine::new_cpu()));
        for _ in 0..3 {
            engine.borrow_mut().step(1.0 / 60.0);
        }
        let owner = Owner::new_root();
        let (timeline, control) = owner.with(|| {
            let timeline = signal(1.0);
            let paused = signal(true);
            let control = history_slider(engine.clone(), paused, timeline)
                .into_element()
                .mount();
            (timeline, control)
        });

        engine.borrow_mut().step_back_history();
        {
            // This is the same immutable borrow held by the viewport telemetry callback.
            // Updating the paused slider must not re-enter scrub_history with a mutable borrow.
            let sim = engine.borrow();
            let cursor = sim.history.cursor();
            let normalized = sim.history.normalized_cursor() as f64;
            batch(|| timeline.set_if_changed(normalized));
            assert_eq!(control.get_value(RangeBase::value_property()), normalized);
            assert_eq!(sim.history.cursor(), cursor);
        }

        control.set_value(RangeBase::value_property(), 0.0);
        assert_eq!(timeline.get_untracked(), 0.0);
        assert_eq!(engine.borrow().history.cursor(), 0);
        assert!(engine.borrow().paused);
        owner.dispose();
    }
}
