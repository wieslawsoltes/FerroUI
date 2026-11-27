//! **Compositor Engine & Physics-Driven GPU Shaders Page**
//!
//! Styled in the clean `ControlCatalog` gallery layout (stock WinUI theme + vector `PathIcon`s):
//! - Runs simulations natively on FerroUI's `Compositor` (`ICompositionSimulation` contract)
//!   with **zero `DispatcherTimer`**, **zero UI layout passes**, and **zero draw-list re-recordings**
//!   (effects write directly to `CompositionVisual` via `simulation_effect_override`).
//! - Live multi-mode switcher on the exact same FerroUI visual tree:
//!   - `Elastic Springs` (`SimulationMode::SpringDynamics`) with selectable analytical spring presets
//!   - `Rigid Bodies` (`SimulationMode::Physics`) with real collision-triggered multi-ring ripples
//!   - `Harmonic Wave` (`SimulationMode::Harmonic` — pure `ferroui-base`, zero physics!)
//! - Live GPU WGSL → Metal MSL / SkSL shader switcher (`Auto`, `Kinetic Glass`, `Impact Ripple`
//!   with 3 simultaneous overlapping wave interference rings, `Energy Field`, `Off`) plus live
//!   **Intensity** & **Distortion** controls rendered natively via `ferroui-quartz` `runtime-shaders`!

use crate::icons::{icon_label, icon_view, Icons};
use ferroui_base::input::InputElement;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::effects::{IImmutableEffect, ImmutableCustomEffect};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, FontWeight, IBrush};
use ferroui_base::rendering::composition::{
    CompositionSimulationHandle, Compositor, ElementComposition, HarmonicWaveSimulation,
    SimulationEffectUpdate, SimulationMode, SimulationTarget,
};
use ferroui_base::{Point, Ref, Vector, Visual};
use ferroui_controls::{Border, Dock};
use ferroui_physics::compositor::{
    CompositorPhysicsController, PhysicsCompositorBehavior, PhysicsEnergyFieldEffect,
    PhysicsKineticGlassEffect, PhysicsShaderMode,
};
use ferroui_physics::ui::SpringConfig;
use ferroui_physics::Vec2;
use ferroui_view::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

fn solid(argb: u32) -> Option<Rc<dyn IBrush>> {
    Some(Rc::new(ImmutableSolidColorBrush::from_uint32(argb)) as Rc<dyn IBrush>)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShowcaseMode {
    PhysicsRigidBody,
    SpringElastic,
    HarmonicWave,
}

impl ShowcaseMode {
    const ALL: [Self; 3] = [Self::SpringElastic, Self::PhysicsRigidBody, Self::HarmonicWave];

    fn label(self) -> &'static str {
        match self {
            Self::PhysicsRigidBody => "Rigid Bodies",
            Self::SpringElastic => "Elastic Springs",
            Self::HarmonicWave => "Harmonic Wave (Non-Physics)",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::PhysicsRigidBody => Icons::CUBE,
            Self::SpringElastic => Icons::MAGNET,
            Self::HarmonicWave => Icons::WAVE,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ShowcaseSpring {
    Snappy,
    Bouncy,
    Gentle,
    Wobbly,
}

impl ShowcaseSpring {
    const ALL: [Self; 4] = [Self::Snappy, Self::Bouncy, Self::Gentle, Self::Wobbly];

    fn label(self) -> &'static str {
        match self {
            Self::Snappy => "Snappy",
            Self::Bouncy => "Bouncy",
            Self::Gentle => "Gentle",
            Self::Wobbly => "Wobbly",
        }
    }

    fn config(self) -> SpringConfig {
        match self {
            Self::Snappy => SpringConfig::snappy(),
            Self::Bouncy => SpringConfig::bouncy(),
            Self::Gentle => SpringConfig::gentle(),
            Self::Wobbly => SpringConfig::wobbly(),
        }
    }
}

struct CardSpec {
    title: &'static str,
    subtitle: &'static str,
    badge: &'static str,
    metric: &'static str,
    icon: &'static str,
    accent_argb: u32,
    accent_color: Color,
    rest_x: f64,
    rest_y: f64,
}

const CARD_W: f64 = 264.0;
const CARD_H: f64 = 166.0;

const CARDS: [CardSpec; 6] = [
    CardSpec {
        title: "Kinetic Glass Shader",
        subtitle: "Dual-layer iridescent sheen, Fresnel bevel & velocity dispersion",
        badge: "WGSL → MSL",
        metric: "0 Re-records",
        icon: Icons::SPARKLE,
        accent_argb: 0xFF0EA5E9,
        accent_color: Color::from_argb(255, 14, 165, 233),
        rest_x: 24.0,
        rest_y: 22.0,
    },
    CardSpec {
        title: "3-Ring Shockwaves",
        subtitle: "Click rapidly or collide to layer 3 interfering wave rings",
        badge: "3-WAVE GPU",
        metric: "3-Tap Pass",
        icon: Icons::TARGET,
        accent_argb: 0xFF8B5CF6,
        accent_color: Color::from_argb(255, 139, 92, 246),
        rest_x: 312.0,
        rest_y: 22.0,
    },
    CardSpec {
        title: "Aurora Energy Field",
        subtitle: "Dual-harmonic plasma refraction steered by live velocity",
        badge: "PLASMA",
        metric: "60/120 Hz",
        icon: Icons::BLUR,
        accent_argb: 0xFF10B981,
        accent_color: Color::from_argb(255, 16, 185, 129),
        rest_x: 600.0,
        rest_y: 22.0,
    },
    CardSpec {
        title: "Zero-Alloc Pipeline",
        subtitle: "Stack-packed Metal uniforms & direct CompositionVisual override",
        badge: "0 ALLOC",
        metric: "Auto-Idle Sleep",
        icon: Icons::LAYERS,
        accent_argb: 0xFFF59E0B,
        accent_color: Color::from_argb(255, 245, 158, 11),
        rest_x: 24.0,
        rest_y: 212.0,
    },
    CardSpec {
        title: "Multi-Mode Contract",
        subtitle: "Harmonic wave runs via ferroui-base alone with zero physics",
        badge: "EXTENSIBLE",
        metric: "Public Trait",
        icon: Icons::WAVE,
        accent_argb: 0xFFEC4899,
        accent_color: Color::from_argb(255, 236, 72, 153),
        rest_x: 312.0,
        rest_y: 212.0,
    },
    CardSpec {
        title: "Live Hit-Tested UI",
        subtitle: "Real FerroUI controls stay interactive while warping on GPU",
        badge: "LIVE TREE",
        metric: "Drag & Fling",
        icon: Icons::CURSOR,
        accent_argb: 0xFF06B6D4,
        accent_color: Color::from_argb(255, 6, 182, 212),
        rest_x: 600.0,
        rest_y: 212.0,
    },
];

/// Builds the ControlCatalog-styled **Compositor & GPU Shaders** showcase page.
pub fn compositor_shader_showcase() -> Element {
    let active_mode = signal(ShowcaseMode::SpringElastic);
    let shader_mode = signal(PhysicsShaderMode::ReactiveAuto);
    let spring_preset = signal(ShowcaseSpring::Bouncy);
    let shader_intensity = signal(1.10_f64);
    let shader_distortion = signal(1.00_f64);
    let gravity_enabled = signal(true);
    let sim_paused = signal(false);
    let telemetry_line = signal(String::from("Attaching simulation to FerroUI Compositor…"));

    let active_handle: Rc<RefCell<Option<CompositionSimulationHandle>>> = Rc::new(RefCell::new(None));
    let physics_ctrl: Rc<RefCell<Option<CompositorPhysicsController>>> = Rc::new(RefCell::new(None));
    let card_targets: Rc<RefCell<Vec<SimulationTarget>>> = Rc::new(RefCell::new(Vec::new()));
    let compositor_ref: Rc<RefCell<Option<Rc<Compositor>>>> = Rc::new(RefCell::new(None));
    let commit_sub: Rc<RefCell<Option<ferroui_base::reactive::Subscription>>> = Rc::new(RefCell::new(None));
    let stage_size: Rc<Cell<(f32, f32)>> = Rc::new(Cell::new((900.0, 440.0)));

    // Sync live Intensity & Distortion slider signals into the active physics controller.
    {
        let pctrl = physics_ctrl.clone();
        let ah = active_handle.clone();
        effect(move || {
            let inten = shader_intensity.get() as f32;
            let dist = shader_distortion.get() as f32;
            if let Some(ctrl) = pctrl.borrow().as_ref() {
                ctrl.set_shader_intensity(inten);
                ctrl.set_shader_distortion(dist);
            }
            if let Some(h) = ah.borrow().as_ref() {
                h.wake();
            }
        });
    }

    let install_mode: Rc<dyn Fn(ShowcaseMode)> = {
        let active_handle = active_handle.clone();
        let physics_ctrl = physics_ctrl.clone();
        let card_targets = card_targets.clone();
        let compositor_ref = compositor_ref.clone();
        let commit_sub = commit_sub.clone();
        let stage_size = stage_size.clone();
        Rc::new(move |mode: ShowcaseMode| {
            *active_handle.borrow_mut() = None;
            *physics_ctrl.borrow_mut() = None;

            let targets = card_targets.borrow().clone();
            if targets.is_empty() {
                return;
            }

            let found_comp = targets
                .iter()
                .find_map(|t| t.composition_visual().map(|cv| cv.compositor().clone()))
                .or_else(|| compositor_ref.borrow().clone())
                .or_else(Compositor::try_get_default_compositor);

            let Some(comp) = found_comp else {
                return;
            };
            *compositor_ref.borrow_mut() = Some(comp.clone());

            if commit_sub.borrow().is_none() {
                let pctrl = physics_ctrl.clone();
                let last_update = Rc::new(Cell::new(Instant::now()));
                let sub = comp.after_commit(move || {
                    if !telemetry_line.is_alive() {
                        return;
                    }
                    if last_update.get().elapsed().as_millis() < 120 {
                        return;
                    }
                    last_update.set(Instant::now());
                    let m = active_mode.get_untracked();
                    if let Some(ctrl) = pctrl.borrow().as_ref() {
                        let t = ctrl.telemetry();
                        telemetry_line.set_if_changed(format!(
                            "Contract: {}  ·  Awake: {}/{}  ·  Peak: {:.0} px/s  ·  Waves: {}  ·  Hits: {}  ·  0 Layout · 0 Re-records",
                            m.contract_mode(),
                            t.awake_bodies,
                            t.bound_visuals,
                            t.peak_speed_px,
                            t.active_ripples,
                            t.total_collisions,
                        ));
                    } else {
                        telemetry_line.set_if_changed(format!(
                            "Contract: {} (Zero-Physics ferroui-base)  ·  Shader: {:?}  ·  0 Layout · 0 Re-records",
                            m.contract_mode(),
                            shader_mode.get_untracked(),
                        ));
                    }
                });
                *commit_sub.borrow_mut() = Some(sub);
            }

            let cur_shader = shader_mode.get_untracked();
            let cur_inten = shader_intensity.get_untracked() as f32;
            let cur_dist = shader_distortion.get_untracked() as f32;
            let cur_spring = spring_preset.get_untracked().config();
            let (aw, ah) = stage_size.get();

            match mode {
                ShowcaseMode::PhysicsRigidBody | ShowcaseMode::SpringElastic => {
                    let behavior = match mode {
                        ShowcaseMode::PhysicsRigidBody => PhysicsCompositorBehavior::RigidBody,
                        _ => PhysicsCompositorBehavior::ElasticSprings,
                    };
                    let (handle, ctrl) = CompositorPhysicsController::attach_to_compositor(
                        &comp,
                        "studio-compositor-physics",
                        aw,
                        ah,
                        behavior,
                    );
                    ctrl.set_shader_mode(cur_shader);
                    ctrl.set_shader_intensity(cur_inten);
                    ctrl.set_shader_distortion(cur_dist);
                    ctrl.set_spring_config(cur_spring);
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

                    // Launch an immediate radial shockwave so multi-ring GPU ripples & motion are visible right away.
                    ctrl.shockwave(Vec2::new(aw * 0.5, ah * 0.45), 340.0);
                    if sim_paused.get_untracked() {
                        handle.pause();
                    }
                    *physics_ctrl.borrow_mut() = Some(ctrl);
                    *active_handle.borrow_mut() = Some(handle);
                }
                ShowcaseMode::HarmonicWave => {
                    let mut wave = HarmonicWaveSimulation::new("studio-harmonic-wave", targets)
                        .with_amplitudes(18.0, 24.0)
                        .with_tilt_and_pulse(0.075, 0.04)
                        .with_frequency(0.55, 0.92);

                    let sh_sig = shader_mode;
                    let inten_sig = shader_intensity;
                    let dist_sig = shader_distortion;
                    wave = wave.with_effect_driver(move |idx, phase, vel_norm, energy| {
                        let sm = sh_sig.get_untracked();
                        if sm == PhysicsShaderMode::Disabled {
                            return SimulationEffectUpdate::Clear;
                        }
                        let accent = CARDS
                            .get(idx)
                            .map(|c| c.accent_color)
                            .unwrap_or(Color::from_argb(255, 14, 165, 233));
                        let inten = inten_sig.get_untracked();
                        let dist = dist_sig.get_untracked();
                        let eff: Arc<dyn IImmutableEffect> = match sm {
                            PhysicsShaderMode::EnergyFieldOnly => Arc::new(ImmutableCustomEffect::new(
                                PhysicsEnergyFieldEffect {
                                    velocity: Vector::new((vel_norm * 420.0) as f64, ((energy - 0.5) * 380.0) as f64),
                                    energy: (0.35 + energy * 0.65) as f64,
                                    phase: phase as f64,
                                    intensity: inten,
                                    distortion: dist,
                                    accent,
                                },
                            )),
                            _ => Arc::new(ImmutableCustomEffect::new(PhysicsKineticGlassEffect {
                                velocity: Vector::new((vel_norm * 460.0) as f64, ((energy - 0.5) * 420.0) as f64),
                                energy: (0.38 + energy * 0.62) as f64,
                                tilt: (vel_norm * 0.30) as f64,
                                phase: phase as f64,
                                intensity: inten,
                                distortion: dist,
                                accent,
                            })),
                        };
                        SimulationEffectUpdate::Set(eff)
                    });

                    let handle = comp.register_simulation(wave);
                    if sim_paused.get_untracked() {
                        handle.pause();
                    }
                    *active_handle.borrow_mut() = Some(handle);
                }
            }
        })
    };

    {
        let ah = active_handle.clone();
        let cs = commit_sub.clone();
        on_cleanup(move || {
            *ah.borrow_mut() = None;
            *cs.borrow_mut() = None;
        });
    }

    // -------------------------------------------------------------------------
    // 1. Clean ControlCatalog-style Page Header & Live Shader Controls
    // -------------------------------------------------------------------------
    let mut mode_row = stack_panel().orientation(Orientation::Horizontal).spacing(6.0);
    for mode in ShowcaseMode::ALL {
        let install = install_mode.clone();
        mode_row = mode_row.child(
            button()
                .padding((11.0, 6.0, 11.0, 6.0))
                .border_brush(move || if active_mode.get() == mode { solid(0xFF0078D4) } else { None })
                .border_thickness(move || if active_mode.get() == mode { 1.5 } else { 1.0 })
                .content(icon_label(mode.icon(), mode.label(), 14.0))
                .on_click(move |_| {
                    active_mode.set(mode);
                    install(mode);
                }),
        );
    }

    let shader_buttons = [
        (Icons::SPARKLE, "Auto Shaders", PhysicsShaderMode::ReactiveAuto),
        (Icons::LAYERS, "Kinetic Glass", PhysicsShaderMode::KineticGlassOnly),
        (Icons::TARGET, "3-Ring Ripple", PhysicsShaderMode::ImpactRippleOnly),
        (Icons::BLUR, "Energy Field", PhysicsShaderMode::EnergyFieldOnly),
        (Icons::BOX_SHAPE, "Off", PhysicsShaderMode::Disabled),
    ];
    let mut shader_row = stack_panel().orientation(Orientation::Horizontal).spacing(5.0);
    for (ic, lbl, sm) in shader_buttons {
        let pctrl = physics_ctrl.clone();
        let ah = active_handle.clone();
        shader_row = shader_row.child(
            button()
                .padding((9.0, 5.0, 9.0, 5.0))
                .border_brush(move || if shader_mode.get() == sm { solid(0xFF8B5CF6) } else { None })
                .border_thickness(move || if shader_mode.get() == sm { 1.5 } else { 1.0 })
                .content(icon_label(ic, lbl, 13.0))
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

    let mut spring_row = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(4.0)
        .vertical_alignment(VerticalAlignment::Center)
        .child(
            text_block()
                .text("Spring:")
                .font_size(11.0)
                .opacity(0.72)
                .vertical_alignment(VerticalAlignment::Center)
                .margin((0.0, 0.0, 3.0, 0.0)),
        );
    for sp in ShowcaseSpring::ALL {
        let pctrl = physics_ctrl.clone();
        spring_row = spring_row.child(
            button()
                .padding((7.0, 3.0, 7.0, 3.0))
                .font_size(11.0)
                .border_brush(move || if spring_preset.get() == sp { solid(0xFF10B981) } else { None })
                .border_thickness(move || if spring_preset.get() == sp { 1.5 } else { 1.0 })
                .content(sp.label())
                .on_click(move |_| {
                    spring_preset.set(sp);
                    if let Some(ctrl) = pctrl.borrow().as_ref() {
                        ctrl.set_spring_config(sp.config());
                    }
                }),
        );
    }

    let tuning_row = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(14.0)
        .vertical_alignment(VerticalAlignment::Center)
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(6.0)
                .vertical_alignment(VerticalAlignment::Center)
                .child(
                    text_block()
                        .text(move || format!("Glow {:.0}%", shader_intensity.get() * 100.0))
                        .font_size(11.0)
                        .opacity(0.8)
                        .width(64.0)
                        .vertical_alignment(VerticalAlignment::Center),
                )
                .child(
                    slider()
                        .width(108.0)
                        .minimum(0.2)
                        .maximum(2.0)
                        .value(shader_intensity.two_way())
                        .vertical_alignment(VerticalAlignment::Center),
                ),
        )
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(6.0)
                .vertical_alignment(VerticalAlignment::Center)
                .child(
                    text_block()
                        .text(move || format!("Refract {:.0}%", shader_distortion.get() * 100.0))
                        .font_size(11.0)
                        .opacity(0.8)
                        .width(74.0)
                        .vertical_alignment(VerticalAlignment::Center),
                )
                .child(
                    slider()
                        .width(108.0)
                        .minimum(0.0)
                        .maximum(1.8)
                        .value(shader_distortion.two_way())
                        .vertical_alignment(VerticalAlignment::Center),
                ),
        )
        .child(spring_row);

    let pctrl_blast = physics_ctrl.clone();
    let pctrl_grav = physics_ctrl.clone();
    let pctrl_home = physics_ctrl.clone();
    let ah_pause = active_handle.clone();
    let ss_blast = stage_size.clone();

    let action_row = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .child(
            button()
                .padding((11.0, 6.0, 11.0, 6.0))
                .content(icon_label(Icons::LIGHTNING, "Shockwave", 14.0))
                .on_click(move |_| {
                    let (w, h) = ss_blast.get();
                    if let Some(ctrl) = pctrl_blast.borrow().as_ref() {
                        ctrl.shockwave(Vec2::new(w * 0.5, h * 0.45), 620.0);
                    }
                }),
        )
        .child(
            button()
                .padding((10.0, 6.0, 10.0, 6.0))
                .border_brush(move || if gravity_enabled.get() { solid(0xFF10B981) } else { None })
                .content(dynamic(move || {
                    if gravity_enabled.get() {
                        icon_label(Icons::ORBIT, "Gravity: On", 14.0)
                    } else {
                        icon_label(Icons::ORBIT, "Zero-G", 14.0)
                    }
                }))
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
                .padding((10.0, 6.0, 10.0, 6.0))
                .content(icon_label(Icons::RESET, "Snap Home", 14.0))
                .on_click(move |_| {
                    if let Some(ctrl) = pctrl_home.borrow().as_ref() {
                        ctrl.snap_home();
                    }
                }),
        )
        .child(
            button()
                .padding((10.0, 6.0, 10.0, 6.0))
                .content(dynamic(move || {
                    if sim_paused.get() {
                        icon_label(Icons::PLAY, "Resume", 14.0)
                    } else {
                        icon_label(Icons::PAUSE, "Pause", 14.0)
                    }
                }))
                .on_click(move |_| {
                    let next = !sim_paused.get_untracked();
                    sim_paused.set(next);
                    if let Some(h) = ah_pause.borrow().as_ref() {
                        h.set_paused(next);
                    }
                }),
        );

    let header_block = border()
        .dock_panel_dock(Dock::Top)
        .padding((22.0, 14.0, 22.0, 10.0))
        .border_brush(solid(0x20808080))
        .border_thickness((0.0, 0.0, 0.0, 1.0))
        .child(
            stack_panel()
                .spacing(8.0)
                .child(
                    dock_panel()
                        .child(
                            text_block()
                                .dock_panel_dock(Dock::Right)
                                .text(move || telemetry_line.get())
                                .font_size(11.0)
                                .opacity(0.76)
                                .vertical_alignment(VerticalAlignment::Bottom),
                        )
                        .child(
                            stack_panel()
                                .spacing(2.0)
                                .child(
                                    text_block()
                                        .text("Compositor & GPU Shaders")
                                        .font_size(22.0)
                                        .font_weight(FontWeight::SemiBold),
                                )
                                .child(
                                    text_block()
                                        .text("Native ICompositionSimulation driving CompositionVisual transforms & zero-alloc WGSL/Metal GPU shaders.")
                                        .font_size(12.0)
                                        .opacity(0.75),
                                ),
                        ),
                )
                .child(dock_panel().child(action_row.dock_panel_dock(Dock::Right)).child(mode_row))
                .child(dock_panel().child(tuning_row.dock_panel_dock(Dock::Right)).child(shader_row)),
        );

    // -------------------------------------------------------------------------
    // 2. Interactive Compositor Arena with 6 Live Multi-Ring Shader Cards
    // -------------------------------------------------------------------------
    let mut stage = canvas().clip_to_bounds(true);

    for (idx, spec) in CARDS.iter().enumerate() {
        let card_ref = NodeRef::<Border>::new();
        let targets_mount = card_targets.clone();
        let install_mount = install_mode.clone();
        let comp_mount = compositor_ref.clone();

        let pctrl_down = physics_ctrl.clone();
        let pctrl_move = physics_ctrl.clone();
        let pctrl_up = physics_ctrl.clone();
        let pctrl_pulse = physics_ctrl.clone();
        let pctrl_ripple = physics_ctrl.clone();

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

        let badge_bg = (spec.accent_argb & 0x00FF_FFFF) | 0x2600_0000;
        let pulse_count = signal(0u32);

        let card_el = border()
            .canvas_left(spec.rest_x)
            .canvas_top(spec.rest_y)
            .width(CARD_W)
            .height(CARD_H)
            .background(solid(0x1E808080))
            .border_brush(solid(spec.accent_argb))
            .border_thickness(1.5)
            .corner_radius(10.0)
            .padding(14.0)
            .node_ref(card_ref)
            .with(move |b: &Ref<Border>| {
                let vis: Ref<Visual> = b.clone().upcast();
                let mut list = targets_mount.borrow_mut();
                list.push(SimulationTarget::visual(vis.clone()));
                if list.len() == CARDS.len() {
                    drop(list);
                    if let Some(cv) = ElementComposition::get_element_visual(&vis) {
                        *comp_mount.borrow_mut() = Some(cv.compositor().clone());
                        install_mount(active_mode.get_untracked());
                    } else {
                        let vis_att = vis.clone();
                        let comp_att = comp_mount.clone();
                        let inst_att = install_mount.clone();
                        vis.attached_to_visual_tree(move |_| {
                            if let Some(cv) = ElementComposition::get_element_visual(&vis_att) {
                                *comp_att.borrow_mut() = Some(cv.compositor().clone());
                            }
                            inst_att(active_mode.get_untracked());
                        })
                        .detach();
                        install_mount(active_mode.get_untracked());
                    }
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
                                    .background(solid(badge_bg))
                                    .corner_radius(4.0)
                                    .padding((6.0, 2.0, 6.0, 2.0))
                                    .vertical_alignment(VerticalAlignment::Center)
                                    .child(
                                        text_block()
                                            .text(spec.badge)
                                            .font_size(9.5)
                                            .foreground(solid(spec.accent_argb)),
                                    ),
                            )
                            .child(
                                stack_panel()
                                    .orientation(Orientation::Horizontal)
                                    .spacing(8.0)
                                    .vertical_alignment(VerticalAlignment::Center)
                                    .child(
                                        border()
                                            .width(28.0)
                                            .height(28.0)
                                            .corner_radius(6.0)
                                            .background(solid(badge_bg))
                                            .child(
                                                border()
                                                    .horizontal_alignment(HorizontalAlignment::Center)
                                                    .vertical_alignment(VerticalAlignment::Center)
                                                    .child(icon_view(spec.icon, 15.0)),
                                            ),
                                    )
                                    .child(
                                        text_block()
                                            .text(spec.title)
                                            .font_size(13.5)
                                            .font_weight(FontWeight::SemiBold)
                                            .vertical_alignment(VerticalAlignment::Center),
                                    ),
                            ),
                    )
                    .child(
                        dock_panel()
                            .dock_panel_dock(Dock::Bottom)
                            .margin((0.0, 8.0, 0.0, 0.0))
                            .child(
                                stack_panel()
                                    .dock_panel_dock(Dock::Right)
                                    .orientation(Orientation::Horizontal)
                                    .spacing(5.0)
                                    .child(
                                        button()
                                            .padding((8.0, 4.0, 8.0, 4.0))
                                            .font_size(11.0)
                                            .content(icon_label(Icons::TARGET, "Wave", 12.0))
                                            .on_click(move |_| {
                                                let n = pulse_count.get_untracked().wrapping_add(1);
                                                pulse_count.set(n);
                                                let ox = 0.25 + ((n % 3) as f64) * 0.25;
                                                let oy = 0.30 + (((n / 3) % 2) as f64) * 0.40;
                                                if let Some(ctrl) = pctrl_ripple.borrow().as_ref() {
                                                    ctrl.trigger_ripple_at(idx, Point::new(ox, oy), 0.96);
                                                }
                                            }),
                                    )
                                    .child(
                                        button()
                                            .padding((8.0, 4.0, 8.0, 4.0))
                                            .font_size(11.0)
                                            .content(icon_label(Icons::LIGHTNING, "Fling", 12.0))
                                            .on_click(move |_| {
                                                let n = pulse_count.get_untracked().wrapping_add(1);
                                                pulse_count.set(n);
                                                let angle = (idx as f32 + n as f32 * 0.7) * 1.1;
                                                if let Some(ctrl) = pctrl_pulse.borrow().as_ref() {
                                                    ctrl.trigger_impact(
                                                        idx,
                                                        Point::new(0.75, 0.72),
                                                        1.0,
                                                        Vec2::new(angle.cos() * 420.0, -380.0),
                                                    );
                                                }
                                            }),
                                    ),
                            )
                            .child(
                                text_block()
                                    .text(move || {
                                        let c = pulse_count.get();
                                        if c == 0 {
                                            spec.metric.to_string()
                                        } else {
                                            format!("{} · {}×", spec.metric, c)
                                        }
                                    })
                                    .font_size(10.5)
                                    .opacity(0.68)
                                    .vertical_alignment(VerticalAlignment::Center),
                            ),
                    )
                    .child(
                        text_block()
                            .margin((0.0, 10.0, 0.0, 0.0))
                            .text(spec.subtitle)
                            .font_size(11.5)
                            .opacity(0.84),
                    ),
            );

        stage = stage.child(card_el);
    }

    let ss_resize = stage_size.clone();
    let stage_frame = border()
        .margin((22.0, 12.0, 22.0, 18.0))
        .corner_radius(10.0)
        .border_brush(solid(0x28808080))
        .border_thickness(1.0)
        .background(solid(0x0D808080))
        .clip_to_bounds(true)
        .on_size_changed({
            let pctrl = physics_ctrl.clone();
            move |e| {
                let sz = e.new_size();
                if sz.width > 120.0 && sz.height > 120.0 {
                    ss_resize.set((sz.width as f32, sz.height as f32));
                    if let Some(ctrl) = pctrl.borrow().as_ref() {
                        ctrl.set_arena_size(sz.width as f32, sz.height as f32);
                    }
                }
            }
        })
        .child(stage);

    dock_panel()
        .clip_to_bounds(true)
        .child(header_block)
        .child(stage_frame)
        .into()
}
