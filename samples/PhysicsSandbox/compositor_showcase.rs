//! **Compositor Engine & Physics-Driven GPU Shaders Showcase**
//!
//! Demonstrates FerroUI's public composition simulation contract
//! (`ferroui_base::rendering::composition::ICompositionSimulation`):
//! 1. **Zero-Layout-Pass Compositor Motion**: Real interactive FerroUI UI cards driven
//!    directly on `Compositor` (`MediaContext` animation frames, ZERO `DispatcherTimer`!).
//! 2. **Multi-Mode Extensibility**: Switch the exact same UI visuals live between:
//!    - `SimulationMode::Physics` (`CompositorPhysicsSimulation` rigid bodies + collisions)
//!    - `SimulationMode::SpringDynamics` (`CompositorPhysicsSimulation` elastic UI springs)
//!    - `SimulationMode::Harmonic` (`HarmonicWaveSimulation` from `ferroui-base` — zero physics!)
//! 3. **Physics-Driven WGSL/SkSL GPU Shaders** (compiled to Apple Metal MSL via `ferroui-quartz`!):
//!    - `PhysicsKineticGlassEffect`: live velocity, tilt, and kinetic energy drive iridescent
//!      specular sweeps, directional micro-refraction, and kinetic perimeter glow.
//!    - `PhysicsImpactRippleEffect`: real physics collisions, pointer grabs, and shockwaves
//!      trigger radial refractive wavefront ripples & chromatic rings right on each UI card!

use ferroui_base::input::InputElement;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::effects::{IImmutableEffect, ImmutableCustomEffect};
use ferroui_base::media::Color;
use ferroui_base::rendering::composition::{
    CompositionSimulationHandle, Compositor, HarmonicWaveSimulation, SimulationEffectUpdate,
    SimulationMode, SimulationTarget,
};
use ferroui_base::{Point, Ref, Vector, Visual};
use ferroui_controls::{Border, Dock};
use ferroui_physics::compositor::{
    CompositorPhysicsController, PhysicsCompositorBehavior, PhysicsKineticGlassEffect,
    PhysicsShaderMode,
};
use ferroui_physics::Vec2;
use ferroui_view::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShowcaseMode {
    PhysicsRigidBody,
    SpringElastic,
    HarmonicWave,
}

impl ShowcaseMode {
    const ALL: [Self; 3] = [Self::PhysicsRigidBody, Self::SpringElastic, Self::HarmonicWave];

    fn label(self) -> &'static str {
        match self {
            Self::PhysicsRigidBody => "⚡ Physics Rigid Bodies",
            Self::SpringElastic => "🧲 Elastic Springs",
            Self::HarmonicWave => "🌊 Harmonic Wave (Non-Physics)",
        }
    }

    fn contract_mode(self) -> SimulationMode {
        match self {
            Self::PhysicsRigidBody => SimulationMode::Physics,
            Self::SpringElastic => SimulationMode::SpringDynamics,
            Self::HarmonicWave => SimulationMode::Harmonic,
        }
    }
}

struct CardSpec {
    title: &'static str,
    subtitle: &'static str,
    badge: &'static str,
    metric: &'static str,
    accent_hex: &'static str,
    accent_color: Color,
    rest_x: f64,
    rest_y: f64,
}

const CARD_W: f64 = 248.0;
const CARD_H: f64 = 154.0;

const CARDS: [CardSpec; 6] = [
    CardSpec {
        title: "Metal WGSL Shader",
        subtitle: "Kinetic glass & impact wavefronts",
        badge: "WGSL → MSL",
        metric: "60/120 Hz · Zero Layout",
        accent_hex: "#38BDF8",
        accent_color: Color::from_argb(255, 56, 189, 248),
        rest_x: 42.0,
        rest_y: 34.0,
    },
    CardSpec {
        title: "Rigid-Body Dynamics",
        subtitle: "Collision contacts fire GPU ripples",
        badge: "PHYSICS",
        metric: "Substepped SIMD Solver",
        accent_hex: "#A855F7",
        accent_color: Color::from_argb(255, 168, 85, 247),
        rest_x: 320.0,
        rest_y: 34.0,
    },
    CardSpec {
        title: "Elastic UI Springs",
        subtitle: "Velocity sheen + squash & stretch",
        badge: "SPRING 2D",
        metric: "Analytical Closed-Form",
        accent_hex: "#34D399",
        accent_color: Color::from_argb(255, 52, 211, 153),
        rest_x: 598.0,
        rest_y: 34.0,
    },
    CardSpec {
        title: "Compositor Clock",
        subtitle: "Direct CompositionVisual batching",
        badge: "COMPOSITOR",
        metric: "Auto-Idle Equilibrium",
        accent_hex: "#F59E0B",
        accent_color: Color::from_argb(255, 245, 158, 11),
        rest_x: 42.0,
        rest_y: 220.0,
    },
    CardSpec {
        title: "Harmonic Field Mode",
        subtitle: "Pure ferroui-base non-physics sim",
        badge: "MULTI-MODE",
        metric: "Zero Physics Dependency",
        accent_hex: "#F472B6",
        accent_color: Color::from_argb(255, 244, 114, 182),
        rest_x: 320.0,
        rest_y: 220.0,
    },
    CardSpec {
        title: "Interactive Hit-Test",
        subtitle: "Drag, fling, or tap Pulse below",
        badge: "LIVE TREE",
        metric: "Real FerroUI Controls",
        accent_hex: "#22D3EE",
        accent_color: Color::from_argb(255, 34, 211, 238),
        rest_x: 598.0,
        rest_y: 220.0,
    },
];

/// Builds the live **Compositor Engine & Physics-Driven GPU Shaders** showcase tab.
pub fn compositor_shader_showcase() -> Element {
    let active_mode = signal(ShowcaseMode::SpringElastic);
    let shader_mode = signal(PhysicsShaderMode::ReactiveAuto);
    let gravity_enabled = signal(true);
    let sim_paused = signal(false);
    let telemetry_line = signal(String::from(
        "Compositor: binding visuals to ICompositionSimulation (WGSL → Metal GPU shaders ready)",
    ));

    // Shared handles owned for the lifetime of this tab.
    let active_handle: Rc<RefCell<Option<CompositionSimulationHandle>>> = Rc::new(RefCell::new(None));
    let physics_ctrl: Rc<RefCell<Option<CompositorPhysicsController>>> = Rc::new(RefCell::new(None));
    let card_targets: Rc<RefCell<Vec<SimulationTarget>>> = Rc::new(RefCell::new(Vec::new()));
    let compositor_ref: Rc<RefCell<Option<Rc<Compositor>>>> = Rc::new(RefCell::new(None));

    // Helper that registers whichever simulation mode is currently selected directly on FerroUI's Compositor.
    let install_mode: Rc<dyn Fn(ShowcaseMode)> = {
        let active_handle = active_handle.clone();
        let physics_ctrl = physics_ctrl.clone();
        let card_targets = card_targets.clone();
        let compositor_ref = compositor_ref.clone();
        Rc::new(move |mode: ShowcaseMode| {
            // 1. Detach previous simulation handle cleanly (resets visuals to identity).
            *active_handle.borrow_mut() = None;
            *physics_ctrl.borrow_mut() = None;

            let Some(comp) = compositor_ref
                .borrow()
                .clone()
                .or_else(Compositor::try_get_default_compositor)
            else {
                return;
            };
            *compositor_ref.borrow_mut() = Some(comp.clone());

            let targets = card_targets.borrow().clone();
            if targets.is_empty() {
                return;
            }

            let cur_shader = shader_mode.get_untracked();

            match mode {
                ShowcaseMode::PhysicsRigidBody | ShowcaseMode::SpringElastic => {
                    let behavior = match mode {
                        ShowcaseMode::PhysicsRigidBody => PhysicsCompositorBehavior::RigidBody,
                        _ => PhysicsCompositorBehavior::ElasticSprings,
                    };
                    let (handle, ctrl) = CompositorPhysicsController::attach_to_compositor(
                        &comp,
                        "studio-compositor-physics",
                        920.0,
                        460.0,
                        behavior,
                    );
                    ctrl.set_shader_mode(cur_shader);
                    ctrl.set_gravity(if gravity_enabled.get_untracked() {
                        Vec2::new(0.0, 12.5)
                    } else {
                        Vec2::ZERO
                    });

                    for (idx, target) in targets.iter().enumerate() {
                        let spec = &CARDS[idx];
                        let rest_center = Vec2::new(
                            (spec.rest_x + CARD_W * 0.5) as f32,
                            (spec.rest_y + CARD_H * 0.5) as f32,
                        );
                        ctrl.bind_visual(
                            target.clone(),
                            rest_center,
                            Vec2::new(CARD_W as f32, CARD_H as f32),
                            spec.accent_color,
                        );
                    }

                    // Give an immediate lively entrance impulse so shaders & motion shine immediately!
                    ctrl.shockwave(Vec2::new(440.0, 200.0), 260.0);
                    if sim_paused.get_untracked() {
                        handle.pause();
                    }
                    *physics_ctrl.borrow_mut() = Some(ctrl);
                    *active_handle.borrow_mut() = Some(handle);
                }
                ShowcaseMode::HarmonicWave => {
                    // Pure ferroui-base non-physics simulation mode driving BOTH CompositionVisual
                    // motion AND live WGSL shader uniforms via the shared public contract!
                    let mut wave = HarmonicWaveSimulation::new("studio-harmonic-wave", targets)
                        .with_amplitudes(18.0, 26.0)
                        .with_tilt_and_pulse(0.075, 0.04)
                        .with_frequency(0.55, 0.92);

                    let sh_sig = shader_mode;
                    wave = wave.with_effect_driver(move |idx, phase, vel_norm, energy| {
                        if sh_sig.get_untracked() == PhysicsShaderMode::Disabled {
                            return SimulationEffectUpdate::Clear;
                        }
                        let accent = CARDS.get(idx).map(|c| c.accent_color).unwrap_or(Color::from_argb(255, 56, 189, 248));
                        let eff: Arc<dyn IImmutableEffect> = Arc::new(ImmutableCustomEffect::new(
                            PhysicsKineticGlassEffect {
                                velocity: Vector::new((vel_norm * 420.0) as f64, ((energy - 0.5) * 380.0) as f64),
                                energy: (0.28 + energy * 0.62) as f64,
                                tilt: (vel_norm * 0.25) as f64,
                                phase: phase as f64,
                                accent,
                            },
                        ));
                        SimulationEffectUpdate::Set(eff)
                    });

                    let handle = comp.register_simulation(wave);
                    if sim_paused.get_untracked() {
                        handle.pause();
                    }
                    *active_handle.borrow_mut() = Some(handle);
                }
            }

            if telemetry_line.is_alive() {
                telemetry_line.set(format!(
                    "Contract Mode: {} · Active Compositor Sims: {} · Shaders: {:?}",
                    mode.contract_mode(),
                    comp.simulation_count(),
                    cur_shader,
                ));
            }
        })
    };

    // Keep live telemetry refreshed gently from the compositor's after_commit hook (no DispatcherTimer!).
    let commit_sub: Rc<RefCell<Option<ferroui_base::reactive::Subscription>>> = Rc::new(RefCell::new(None));
    {
        let pctrl = physics_ctrl.clone();
        let comp_ref = compositor_ref.clone();
        let last_update = Rc::new(Cell::new(Instant::now()));
        if let Some(comp) = Compositor::try_get_default_compositor() {
            *comp_ref.borrow_mut() = Some(comp.clone());
            let sub = comp.after_commit(move || {
                if !telemetry_line.is_alive() {
                    return;
                }
                if last_update.get().elapsed().as_millis() < 120 {
                    return;
                }
                last_update.set(Instant::now());
                let mode = active_mode.get_untracked();
                if let Some(ctrl) = pctrl.borrow().as_ref() {
                    let t = ctrl.telemetry();
                    telemetry_line.set_if_changed(format!(
                        "Mode: {}  ·  Awake: {}/{}  ·  Peak Speed: {:.0} px/s  ·  GPU Ripples: {}  ·  Collisions: {}  ·  Steps: {}",
                        mode.contract_mode(),
                        t.awake_bodies,
                        t.bound_visuals,
                        t.peak_speed_px,
                        t.active_ripples,
                        t.total_collisions,
                        t.step_count,
                    ));
                } else {
                    telemetry_line.set_if_changed(format!(
                        "Mode: {} (Zero-Physics ferroui-base Contract)  ·  Shaders: {:?}  ·  Direct Compositor Clock",
                        mode.contract_mode(),
                        shader_mode.get_untracked(),
                    ));
                }
            });
            *commit_sub.borrow_mut() = Some(sub);
        }
    }

    // Cleanup simulation handle & commit subscription when leaving tab.
    {
        let ah = active_handle.clone();
        let cs = commit_sub.clone();
        on_cleanup(move || {
            *ah.borrow_mut() = None;
            *cs.borrow_mut() = None;
        });
    }

    // -------------------------------------------------------------------------
    // 1. Top Mode Switcher & Shader Pipeline Toolbar
    // -------------------------------------------------------------------------
    let mut mode_row = stack_panel().orientation(Orientation::Horizontal).spacing(5.0);
    for mode in ShowcaseMode::ALL {
        let install = install_mode.clone();
        mode_row = mode_row.child(
            button()
                .padding((10.0, 5.0, 10.0, 5.0))
                .corner_radius(6.0)
                .background(move || if active_mode.get() == mode { "#0284C7" } else { "#182235" })
                .foreground("#F8FAFC")
                .font_size(11.5)
                .content(mode.label())
                .on_click(move |_| {
                    active_mode.set(mode);
                    install(mode);
                }),
        );
    }

    let shader_buttons = [
        ("✨ Auto Shaders", PhysicsShaderMode::ReactiveAuto),
        ("💎 Kinetic Glass", PhysicsShaderMode::KineticGlassOnly),
        ("🌊 Impact Ripple", PhysicsShaderMode::ImpactRippleOnly),
        ("⬜ Shaders Off", PhysicsShaderMode::Disabled),
    ];
    let mut shader_row = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
    for (lbl, sm) in shader_buttons {
        let pctrl = physics_ctrl.clone();
        let ah = active_handle.clone();
        shader_row = shader_row.child(
            button()
                .padding((8.0, 4.0, 8.0, 4.0))
                .corner_radius(5.0)
                .background(move || if shader_mode.get() == sm { "#7C3AED" } else { "#1E293B" })
                .foreground("#F8FAFC")
                .font_size(11.0)
                .content(lbl)
                .on_click(move |_| {
                    shader_mode.set(sm);
                    if let Some(ctrl) = pctrl.borrow().as_ref() {
                        ctrl.set_shader_mode(sm);
                    }
                    if let Some(h) = ah.borrow().as_ref() {
                        h.wake();
                    }
                }),
        );
    }

    let pctrl_blast = physics_ctrl.clone();
    let pctrl_grav = physics_ctrl.clone();
    let pctrl_home = physics_ctrl.clone();
    let ah_pause = active_handle.clone();

    let action_row = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(5.0)
        .child(
            button()
                .padding((10.0, 5.0, 10.0, 5.0))
                .corner_radius(6.0)
                .background("#D97706")
                .foreground("#FFFFFF")
                .font_size(11.5)
                .content("💥 Shockwave")
                .on_click(move |_| {
                    if let Some(ctrl) = pctrl_blast.borrow().as_ref() {
                        ctrl.shockwave(Vec2::new(450.0, 210.0), 560.0);
                    }
                }),
        )
        .child(
            button()
                .padding((9.0, 5.0, 9.0, 5.0))
                .corner_radius(6.0)
                .background(move || if gravity_enabled.get() { "#059669" } else { "#334155" })
                .foreground("#FFFFFF")
                .font_size(11.5)
                .content(move || if gravity_enabled.get() { "🌍 Gravity: ON" } else { "🪐 Zero-G" })
                .on_click(move |_| {
                    let next = !gravity_enabled.get_untracked();
                    gravity_enabled.set(next);
                    if let Some(ctrl) = pctrl_grav.borrow().as_ref() {
                        ctrl.set_gravity(if next { Vec2::new(0.0, 12.5) } else { Vec2::ZERO });
                    }
                }),
        )
        .child(
            button()
                .padding((9.0, 5.0, 9.0, 5.0))
                .corner_radius(6.0)
                .background("#1E293B")
                .foreground("#E2E8F0")
                .font_size(11.5)
                .content("🧲 Snap Home")
                .on_click(move |_| {
                    if let Some(ctrl) = pctrl_home.borrow().as_ref() {
                        ctrl.snap_home();
                    }
                }),
        )
        .child(
            button()
                .padding((9.0, 5.0, 9.0, 5.0))
                .corner_radius(6.0)
                .background(move || if sim_paused.get() { "#059669" } else { "#2563EB" })
                .foreground("#FFFFFF")
                .font_size(11.5)
                .content(move || if sim_paused.get() { "▶ Resume" } else { "⏸ Pause" })
                .on_click(move |_| {
                    let next = !sim_paused.get_untracked();
                    sim_paused.set(next);
                    if let Some(h) = ah_pause.borrow().as_ref() {
                        h.set_paused(next);
                    }
                }),
        );

    let top_bar = border()
        .dock_panel_dock(Dock::Top)
        .background("#0F172A")
        .border_brush("#24314A")
        .border_thickness((0.0, 0.0, 0.0, 1.0))
        .padding((14.0, 9.0, 14.0, 9.0))
        .child(
            stack_panel()
                .spacing(7.0)
                .child(dock_panel().child(action_row.dock_panel_dock(Dock::Right)).child(mode_row))
                .child(
                    dock_panel()
                        .child(
                            text_block()
                                .dock_panel_dock(Dock::Right)
                                .text(move || telemetry_line.get())
                                .font_size(11.0)
                                .foreground("#38BDF8")
                                .vertical_alignment(VerticalAlignment::Center),
                        )
                        .child(shader_row),
                ),
        );

    // -------------------------------------------------------------------------
    // 2. Interactive Compositor Arena with 6 Live Shader-Driven UI Cards
    // -------------------------------------------------------------------------
    let mut stage = canvas().clip_to_bounds(true);

    for (idx, spec) in CARDS.iter().enumerate() {
        let card_ref = NodeRef::<Border>::new();
        let targets_mount = card_targets.clone();
        let install_mount = install_mode.clone();

        let pctrl_down = physics_ctrl.clone();
        let pctrl_move = physics_ctrl.clone();
        let pctrl_up = physics_ctrl.clone();
        let pctrl_pulse = physics_ctrl.clone();

        let dragging = Rc::new(Cell::new(false));
        let last_pt = Rc::new(Cell::new(Point::new(0.0, 0.0)));
        let last_t = Rc::new(Cell::new(Instant::now()));

        let drag_d = dragging.clone();
        let drag_m = dragging.clone();
        let drag_u = dragging.clone();
        let lp_d = last_pt.clone();
        let lp_m = last_pt.clone();
        let lt_d = last_t.clone();
        let lt_m = last_t.clone();

        let card_el = border()
            .canvas_left(spec.rest_x)
            .canvas_top(spec.rest_y)
            .width(CARD_W)
            .height(CARD_H)
            .background("#141E33")
            .border_brush(spec.accent_hex)
            .border_thickness(1.5)
            .corner_radius(12.0)
            .padding(14.0)
            .node_ref(card_ref)
            .with(move |b: &Ref<Border>| {
                let vis: Ref<Visual> = b.clone().upcast();
                let mut list = targets_mount.borrow_mut();
                list.push(SimulationTarget::visual(vis));
                if list.len() == CARDS.len() {
                    drop(list);
                    install_mount(active_mode.get_untracked());
                }
            })
            .on_pointer_pressed(move |e| {
                if e.handled() {
                    return;
                }
                e.set_handled(true);
                if let Some(b) = card_ref.get_untracked() {
                    let input: Ref<InputElement> = b.clone().upcast();
                    e.pointer().capture(Some(&input));
                    let local = e.get_position(Some(&b));
                    let norm = Point::new((local.x / CARD_W).clamp(0.0, 1.0), (local.y / CARD_H).clamp(0.0, 1.0));
                    if let Some(ctrl) = pctrl_down.borrow().as_ref() {
                        ctrl.begin_drag(idx, norm);
                    }
                }
                drag_d.set(true);
                lp_d.set(e.get_position(None));
                lt_d.set(Instant::now());
            })
            .on_pointer_moved(move |e| {
                if !drag_m.get() {
                    return;
                }
                e.set_handled(true);
                let cur = e.get_position(None);
                let prev = lp_m.get();
                let dt = lt_m.get().elapsed().as_secs_f32().clamp(0.004, 0.05);
                lp_m.set(cur);
                lt_m.set(Instant::now());
                let delta = Vec2::new((cur.x - prev.x) as f32, (cur.y - prev.y) as f32);
                if let Some(ctrl) = pctrl_move.borrow().as_ref() {
                    ctrl.drag_by(idx, delta, dt);
                }
            })
            .on_pointer_released(move |e| {
                if drag_u.replace(false) {
                    e.set_handled(true);
                    e.pointer().capture(None);
                    if let Some(ctrl) = pctrl_up.borrow().as_ref() {
                        ctrl.end_drag(idx);
                    }
                }
            })
            .child(
                dock_panel()
                    .child(
                        dock_panel()
                            .dock_panel_dock(Dock::Top)
                            .child(
                                border()
                                    .dock_panel_dock(Dock::Right)
                                    .background("#0B1221")
                                    .border_brush(spec.accent_hex)
                                    .border_thickness(1.0)
                                    .corner_radius(4.0)
                                    .padding((6.0, 2.0, 6.0, 2.0))
                                    .child(text_block().text(spec.badge).font_size(9.5).foreground(spec.accent_hex)),
                            )
                            .child(
                                text_block()
                                    .text(spec.title)
                                    .font_size(14.0)
                                    .foreground("#F8FAFC")
                                    .vertical_alignment(VerticalAlignment::Center),
                            ),
                    )
                    .child(
                        dock_panel()
                            .dock_panel_dock(Dock::Bottom)
                            .margin((0.0, 8.0, 0.0, 0.0))
                            .child(
                                button()
                                    .dock_panel_dock(Dock::Right)
                                    .padding((9.0, 4.0, 9.0, 4.0))
                                    .corner_radius(5.0)
                                    .background(spec.accent_hex)
                                    .foreground("#08101E")
                                    .font_size(10.5)
                                    .content("⚡ Pulse")
                                    .on_click(move |_| {
                                        if let Some(ctrl) = pctrl_pulse.borrow().as_ref() {
                                            let angle = idx as f32 * 1.1;
                                            ctrl.trigger_impact(
                                                idx,
                                                Point::new(0.82, 0.80),
                                                1.0,
                                                Vec2::new(angle.cos() * 340.0, -320.0),
                                            );
                                        }
                                    }),
                            )
                            .child(
                                text_block()
                                    .text(spec.metric)
                                    .font_size(10.5)
                                    .foreground("#94A3B8")
                                    .vertical_alignment(VerticalAlignment::Center),
                            ),
                    )
                    .child(
                        text_block()
                            .margin((0.0, 10.0, 0.0, 0.0))
                            .text(spec.subtitle)
                            .font_size(11.5)
                            .foreground("#CBD5E1"),
                    ),
            );

        stage = stage.child(card_el);
    }

    // Keep rigid-body arena bounds matched to stage size automatically.
    let stage_container = border()
        .background("#090E1A")
        .clip_to_bounds(true)
        .on_size_changed({
            let pctrl = physics_ctrl.clone();
            move |e| {
                let sz = e.new_size();
                if sz.width > 120.0 && sz.height > 120.0 {
                    if let Some(ctrl) = pctrl.borrow().as_ref() {
                        ctrl.set_arena_size(sz.width as f32, sz.height as f32);
                    }
                }
            }
        })
        .child(stage);

    let footer_hint = border()
        .dock_panel_dock(Dock::Bottom)
        .background("#0B1120")
        .border_brush("#1E293B")
        .border_thickness((0.0, 1.0, 0.0, 0.0))
        .padding((14.0, 6.0, 14.0, 6.0))
        .child(
            text_block()
                .text("Drag & fling any card to drive GPU Kinetic Glass velocity sheen · Smash cards or click ⚡ Pulse / 💥 Shockwave for WGSL Impact Ripples · Switch to 🌊 Harmonic Wave to test a non-physics simulation mode on the same ferroui-base ICompositionSimulation contract!")
                .font_size(11.0)
                .foreground("#94A3B8")
                .horizontal_alignment(HorizontalAlignment::Left),
        );

    dock_panel()
        .background("#090D16")
        .clip_to_bounds(true)
        .child(top_bar)
        .child(footer_hint)
        .child(stage_container)
        .into()
}
