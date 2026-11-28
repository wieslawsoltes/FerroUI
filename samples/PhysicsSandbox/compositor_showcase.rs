//! A Fluent project board with compositor-driven motion and editable stock controls.

use crate::icons::{icon_label, icon_view, Icons};
use crate::showcase_style::{page_header, subtle_text, surface};
use ferroui_base::input::InputElement;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::effects::{IImmutableEffect, ImmutableCustomEffect};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, FontWeight, IBrush, TextWrapping};
use ferroui_base::rendering::composition::{
    CompositionSimulationHandle, Compositor, HarmonicWaveSimulation, SimulationEffectUpdate,
    SimulationTarget,
};
use ferroui_base::{Point, Ref, Vector, Visual, WeakRef};
use ferroui_controls::{Border, Dock};
use ferroui_physics::compositor::{
    CompositorPhysicsController, PhysicsCompositorBehavior, PhysicsEnergyFieldEffect,
    PhysicsImpactRippleEffect, PhysicsKineticGlassEffect, PhysicsShaderMode,
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
    const ALL: [Self; 3] = [
        Self::SpringElastic,
        Self::PhysicsRigidBody,
        Self::HarmonicWave,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::PhysicsRigidBody => "Rigid bodies",
            Self::SpringElastic => "Elastic springs",
            Self::HarmonicWave => "Harmonic motion",
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::PhysicsRigidBody => Icons::CUBE,
            Self::SpringElastic => Icons::MAGNET,
            Self::HarmonicWave => Icons::WAVE,
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
}

const CARD_W: f64 = 264.0;
const CARD_H: f64 = 208.0;

const CARDS: [CardSpec; 6] = [
    CardSpec {
        title: "Design review",
        subtitle: "Review the navigation flow and approve the interaction details.",
        badge: "REVIEW",
        metric: "Ready for feedback",
        icon: Icons::SPARKLE,
        accent_argb: 0xFF0078D4,
        accent_color: Color::from_argb(255, 14, 165, 233),
    },
    CardSpec {
        title: "Release checklist",
        subtitle: "Track completion while the card remains fully interactive.",
        badge: "CHECKLIST",
        metric: "Review in progress",
        icon: Icons::TARGET,
        accent_argb: 0xFF0078D4,
        accent_color: Color::from_argb(255, 0, 120, 212),
    },
    CardSpec {
        title: "Motion guidelines",
        subtitle: "Tune a gentle accent without distracting from the content.",
        badge: "DESIGN",
        metric: "Draft guidelines",
        icon: Icons::BLUR,
        accent_argb: 0xFF0078D4,
        accent_color: Color::from_argb(255, 0, 120, 212),
    },
    CardSpec {
        title: "Prototype session",
        subtitle: "Keep a working note alongside the project board.",
        badge: "NOTES",
        metric: "Local workspace",
        icon: Icons::LAYERS,
        accent_argb: 0xFF0078D4,
        accent_color: Color::from_argb(255, 0, 120, 212),
    },
    CardSpec {
        title: "Team readiness",
        subtitle: "Keep the handoff status with the project notes.",
        badge: "STATUS",
        metric: "Ready to review",
        icon: Icons::WAVE,
        accent_argb: 0xFF0078D4,
        accent_color: Color::from_argb(255, 0, 120, 212),
    },
    CardSpec {
        title: "Export package",
        subtitle: "Include the review notes with the final package.",
        badge: "PACKAGE",
        metric: "Package options",
        icon: Icons::CURSOR,
        accent_argb: 0xFF0078D4,
        accent_color: Color::from_argb(255, 0, 120, 212),
    },
];

/// Arrange the board into one, two or three columns without clipping cards.
fn board_layout(width: f64) -> ([Vec2; 6], f64) {
    let columns = (((width - 24.0) / (CARD_W + 24.0)).floor() as usize).clamp(1, 3);
    let points = std::array::from_fn(|i| {
        Vec2::new(
            (24.0 + (i % columns) as f64 * (CARD_W + 24.0)) as f32,
            (24.0 + (i / columns) as f64 * (CARD_H + 24.0)) as f32,
        )
    });
    let rows = 6_usize.div_ceil(columns);
    (points, 24.0 + rows as f64 * (CARD_H + 24.0))
}

/// Builds the ControlCatalog-styled **Compositor & GPU Shaders** showcase page.
pub fn compositor_shader_showcase() -> Element {
    let active_mode = signal(ShowcaseMode::SpringElastic);
    let shader_mode = signal(PhysicsShaderMode::ReactiveAuto);
    let spring_preset = signal(ShowcaseSpring::Gentle);
    let shader_intensity = signal(0.65_f64);
    let shader_distortion = signal(0.45_f64);
    let gravity_enabled = signal(true);
    let sim_paused = signal(false);
    let telemetry_line = signal(String::from("Preparing motion workspace…"));
    let positions = signal(board_layout(900.0).0);
    let content_height = signal(board_layout(900.0).1);
    let mounted = Rc::new(Cell::new(false));

    let active_handle: Rc<RefCell<Option<CompositionSimulationHandle>>> =
        Rc::new(RefCell::new(None));
    let physics_ctrl: Rc<RefCell<Option<CompositorPhysicsController>>> =
        Rc::new(RefCell::new(None));
    let card_targets: Rc<RefCell<Vec<WeakRef<Visual>>>> = Rc::new(RefCell::new(Vec::new()));
    let compositor_ref: Rc<RefCell<Option<Rc<Compositor>>>> = Rc::new(RefCell::new(None));
    let commit_sub: Rc<RefCell<Option<ferroui_base::reactive::Subscription>>> =
        Rc::new(RefCell::new(None));
    let stage_size: Rc<Cell<(f32, f32)>> =
        Rc::new(Cell::new((900.0, content_height.get_untracked() as f32)));

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
        let mounted = mounted.clone();
        Rc::new(move |mode: ShowcaseMode| {
            if !mounted.get() {
                return;
            }
            *active_handle.borrow_mut() = None;
            *physics_ctrl.borrow_mut() = None;

            let targets: Vec<_> = card_targets
                .borrow()
                .iter()
                .filter_map(|v| v.upgrade())
                .map(SimulationTarget::visual)
                .collect();
            if targets.len() != CARDS.len() {
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
                            "{}  ·  Active cards: {}/{}  ·  Peak: {:.0} px/s  ·  Ripples: {}  ·  Collisions: {}",
                            m.label(),
                            t.awake_bodies,
                            t.bound_visuals,
                            t.peak_speed_px,
                            t.active_ripples,
                            t.total_collisions,
                        ));
                    } else {
                        telemetry_line.set_if_changed(format!(
                            "{}  ·  Effect: {:?}",
                            m.label(),
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
                        let origin = positions.get_untracked()[idx];
                        let rest_center =
                            origin + Vec2::new(CARD_W as f32 * 0.5, CARD_H as f32 * 0.5);
                        ctrl.bind_visual(
                            target.clone(),
                            rest_center,
                            Vec2::new(CARD_W as f32, CARD_H as f32),
                            spec.accent_color,
                        );
                    }

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
                            PhysicsShaderMode::ImpactRippleOnly => {
                                let progress = (phase as f64).rem_euclid(1.0);
                                Arc::new(ImmutableCustomEffect::new(PhysicsImpactRippleEffect {
                                    origin_1: Point::new(0.5, 0.5),
                                    progress_1: progress,
                                    amp_1: 1.0 - progress,
                                    intensity: inten,
                                    distortion: dist,
                                    accent,
                                    ..PhysicsImpactRippleEffect::default()
                                }))
                            }
                            PhysicsShaderMode::EnergyFieldOnly => {
                                Arc::new(ImmutableCustomEffect::new(PhysicsEnergyFieldEffect {
                                    velocity: Vector::new(
                                        (vel_norm * 420.0) as f64,
                                        ((energy - 0.5) * 380.0) as f64,
                                    ),
                                    energy: (0.35 + energy * 0.65) as f64,
                                    phase: phase as f64,
                                    intensity: inten,
                                    distortion: dist,
                                    accent,
                                }))
                            }
                            _ => Arc::new(ImmutableCustomEffect::new(PhysicsKineticGlassEffect {
                                velocity: Vector::new(
                                    (vel_norm * 460.0) as f64,
                                    ((energy - 0.5) * 420.0) as f64,
                                ),
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
    let mut mode_row = wrap_panel().item_spacing(8.0).line_spacing(8.0);
    for mode in ShowcaseMode::ALL {
        let install = install_mode.clone();
        mode_row = mode_row.child(
            toggle_button()
                .is_checked(move || Some(active_mode.get() == mode))
                .content(icon_label(mode.icon(), mode.label(), 14.0))
                .on_click(move |_| {
                    active_mode.set(mode);
                    install(mode);
                }),
        );
    }

    let shader_buttons = [
        (Icons::SPARKLE, "Reactive", PhysicsShaderMode::ReactiveAuto),
        (
            Icons::LAYERS,
            "Kinetic glass",
            PhysicsShaderMode::KineticGlassOnly,
        ),
        (
            Icons::TARGET,
            "Impact ripple",
            PhysicsShaderMode::ImpactRippleOnly,
        ),
        (
            Icons::BLUR,
            "Energy field",
            PhysicsShaderMode::EnergyFieldOnly,
        ),
        (Icons::BOX_SHAPE, "Off", PhysicsShaderMode::Disabled),
    ];
    let mut shader_row = wrap_panel().item_spacing(8.0).line_spacing(8.0);
    for (ic, lbl, sm) in shader_buttons {
        let pctrl = physics_ctrl.clone();
        let ah = active_handle.clone();
        shader_row = shader_row.child(
            toggle_button()
                .is_checked(move || Some(shader_mode.get() == sm))
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

    let mut spring_row = wrap_panel()
        .item_spacing(8.0)
        .line_spacing(8.0)
        .is_enabled(move || active_mode.get() == ShowcaseMode::SpringElastic)
        .child(
            text_block()
                .text("Response")
                .font_size(11.0)
                .opacity(0.72)
                .vertical_alignment(VerticalAlignment::Center)
                .margin((0.0, 0.0, 3.0, 0.0)),
        );
    for sp in ShowcaseSpring::ALL {
        let pctrl = physics_ctrl.clone();
        spring_row = spring_row.child(
            toggle_button()
                .is_checked(move || Some(spring_preset.get() == sp))
                .content(sp.label())
                .on_click(move |_| {
                    spring_preset.set(sp);
                    if let Some(ctrl) = pctrl.borrow().as_ref() {
                        ctrl.set_spring_config(sp.config());
                    }
                }),
        );
    }

    let tuning_row = wrap_panel()
        .item_spacing(16.0)
        .line_spacing(12.0)
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

    let action_row = wrap_panel()
        .item_spacing(8.0)
        .line_spacing(8.0)
        .child(
            button()
                .padding((11.0, 6.0, 11.0, 6.0))
                .content(icon_label(Icons::LIGHTNING, "Impulse", 14.0))
                .is_enabled(move || active_mode.get() != ShowcaseMode::HarmonicWave)
                .on_click(move |_| {
                    let (w, h) = ss_blast.get();
                    if let Some(ctrl) = pctrl_blast.borrow().as_ref() {
                        ctrl.shockwave(Vec2::new(w * 0.5, h * 0.45), 620.0);
                    }
                }),
        )
        .child(
            toggle_button()
                .is_checked(move || Some(gravity_enabled.get()))
                .is_enabled(move || active_mode.get() == ShowcaseMode::PhysicsRigidBody)
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
                        ctrl.set_gravity(if next {
                            Vec2::new(0.0, 12.5)
                        } else {
                            Vec2::ZERO
                        });
                    }
                }),
        )
        .child(
            button()
                .padding((10.0, 6.0, 10.0, 6.0))
                .content(icon_label(Icons::RESET, "Restore layout", 14.0))
                .is_enabled(move || active_mode.get() != ShowcaseMode::HarmonicWave)
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

    let header_block = stack_panel()
        .dock_panel_dock(Dock::Top)
        .margin((24.0, 24.0, 24.0, 12.0))
        .spacing(12.0)
        .child(page_header(
            "Motion workspace",
            "A live project board. Edit its controls, drag a card, and compare spring, rigid-body, and harmonic motion.",
        ))
        .child(mode_row)
        .child(expander().header("Motion and appearance").content(
            stack_panel().spacing(12.0).margin(12.0)
                .child(action_row).child(shader_row).child(tuning_row)
                .child(subtle_text().text(move || telemetry_line.get()).font_size(12.0)),
        ));

    // -------------------------------------------------------------------------
    // 2. Interactive Compositor Arena with 6 Live Multi-Ring Shader Cards
    // -------------------------------------------------------------------------
    let mut stage = canvas()
        .clip_to_bounds(true)
        .min_width(CARD_W + 48.0)
        .height(move || content_height.get());

    for (idx, spec) in CARDS.iter().enumerate() {
        let card_ref = NodeRef::<Border>::new();
        let targets_mount = card_targets.clone();

        let pctrl_down = physics_ctrl.clone();
        let pctrl_move = physics_ctrl.clone();
        let pctrl_up = physics_ctrl.clone();
        let pctrl_pulse = physics_ctrl.clone();
        let pctrl_ripple = physics_ctrl.clone();
        let pctrl_lost = physics_ctrl.clone();

        let dragging = Rc::new(Cell::new(false));
        let last_pt = Rc::new(Cell::new(Point::new(0.0, 0.0)));
        let last_t = Rc::new(Cell::new(Instant::now()));

        let drag_d = dragging.clone();
        let drag_m = dragging.clone();
        let drag_u = dragging.clone();
        let drag_lost = dragging.clone();
        let lp_d = last_pt.clone();
        let lp_m = last_pt.clone();
        let lt_d = last_t.clone();
        let lt_m = last_t.clone();

        let badge_bg = (spec.accent_argb & 0x00FF_FFFF) | 0x2600_0000;
        let pulse_count = signal(0u32);

        let card_el = surface()
            .canvas_left(move || positions.get()[idx].x as f64)
            .canvas_top(move || positions.get()[idx].y as f64)
            .width(CARD_W)
            .height(CARD_H)
            .border_thickness(1.0)
            .corner_radius(8.0)
            .padding(14.0)
            .node_ref(card_ref)
            .with(move |b: &Ref<Border>| {
                let vis: Ref<Visual> = b.clone().upcast();
                targets_mount.borrow_mut().push(vis.downgrade());
            })
            .on_pointer_pressed(move |e| {
                if e.handled() || active_mode.get_untracked() == ShowcaseMode::HarmonicWave {
                    return;
                }
                e.set_handled(true);
                if let Some(b) = card_ref.get_untracked() {
                    let input: Ref<InputElement> = b.clone().upcast();
                    e.pointer().capture(Some(&input));
                    let local = e.get_position(Some(&b));
                    let norm = Point::new(
                        (local.x / CARD_W).clamp(0.0, 1.0),
                        (local.y / CARD_H).clamp(0.0, 1.0),
                    );
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
            .on_pointer_capture_lost(move |_| {
                if drag_lost.replace(false) {
                    if let Some(ctrl) = pctrl_lost.borrow().as_ref() {
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
                                                    .horizontal_alignment(
                                                        HorizontalAlignment::Center,
                                                    )
                                                    .vertical_alignment(VerticalAlignment::Center)
                                                    .child(icon_view(spec.icon, 15.0)),
                                            ),
                                    )
                                    .child(
                                        text_block()
                                            .text(spec.title)
                                            .max_width(130.0)
                                            .text_wrapping(TextWrapping::Wrap)
                                            .font_size(14.0)
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
                                            .content(icon_label(Icons::TARGET, "Focus", 12.0))
                                            .is_enabled(move || {
                                                active_mode.get() != ShowcaseMode::HarmonicWave
                                            })
                                            .on_click(move |_| {
                                                let n = pulse_count.get_untracked().wrapping_add(1);
                                                pulse_count.set(n);
                                                let ox = 0.25 + ((n % 3) as f64) * 0.25;
                                                let oy = 0.30 + (((n / 3) % 2) as f64) * 0.40;
                                                if let Some(ctrl) = pctrl_ripple.borrow().as_ref() {
                                                    ctrl.trigger_ripple_at(
                                                        idx,
                                                        Point::new(ox, oy),
                                                        0.96,
                                                    );
                                                }
                                            }),
                                    )
                                    .child(
                                        button()
                                            .padding((8.0, 4.0, 8.0, 4.0))
                                            .font_size(11.0)
                                            .content(icon_label(Icons::LIGHTNING, "Nudge", 12.0))
                                            .is_enabled(move || {
                                                active_mode.get() != ShowcaseMode::HarmonicWave
                                            })
                                            .on_click(move |_| {
                                                let n = pulse_count.get_untracked().wrapping_add(1);
                                                pulse_count.set(n);
                                                let angle = (idx as f32 + n as f32 * 0.7) * 1.1;
                                                if let Some(ctrl) = pctrl_pulse.borrow().as_ref() {
                                                    ctrl.trigger_impact(
                                                        idx,
                                                        Point::new(0.75, 0.72),
                                                        1.0,
                                                        Vec2::new(angle.cos() * 150.0, -120.0),
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
                        stack_panel()
                            .spacing(8.0)
                            .margin((0.0, 10.0, 0.0, 0.0))
                            .child(subtle_text().text(spec.subtitle).font_size(12.0))
                            .child(card_control(idx, sim_paused, active_handle.clone())),
                    ),
            );

        stage = stage.child(card_el);
    }

    let ctrl_unloaded = physics_ctrl.clone();
    let ss_resize = stage_size.clone();
    let install_resize = install_mode.clone();
    let stage_frame = surface()
        .margin((24.0, 0.0, 24.0, 24.0))
        .clip_to_bounds(true)
        .on_size_changed(move |e| {
            let size = e.new_size();
            if size.width < 1.0 || size.height < 1.0 {
                return;
            }
            let (next, height) = board_layout(size.width);
            let changed = positions.get_untracked() != next;
            positions.set_if_changed(next);
            content_height.set_if_changed(height.max(size.height));
            ss_resize.set((
                size.width.max(CARD_W + 48.0) as f32,
                content_height.get_untracked() as f32,
            ));
            if changed {
                install_resize(active_mode.get_untracked());
            } else if let Some(ctrl) = physics_ctrl.borrow().as_ref() {
                ctrl.set_arena_size(size.width as f32, content_height.get_untracked() as f32);
            }
        })
        .child(scroll_viewer().content(stage));

    let mount_state = mounted.clone();
    let unmount_state = mounted.clone();
    let install_loaded = install_mode.clone();
    let handle_unloaded = active_handle.clone();
    let subscription_unloaded = commit_sub.clone();
    let compositor_unloaded = compositor_ref.clone();
    dock_panel()
        .clip_to_bounds(true)
        .on_loaded(move |_| {
            mount_state.set(true);
            install_loaded(active_mode.get_untracked());
        })
        .on_unloaded(move |_| {
            unmount_state.set(false);
            // Unregister before the visual tree is released: simulations own
            // their active targets, while the page only retains weak handles.
            *handle_unloaded.borrow_mut() = None;
            *ctrl_unloaded.borrow_mut() = None;
            *subscription_unloaded.borrow_mut() = None;
            *compositor_unloaded.borrow_mut() = None;
        })
        .child(header_block)
        .child(stage_frame)
        .into()
}

fn card_control(
    index: usize,
    paused: Signal<bool>,
    handle: Rc<RefCell<Option<CompositionSimulationHandle>>>,
) -> Element {
    match index {
        0 => {
            let approved = signal(Some(false));
            check_box()
                .content("Approve navigation")
                .is_checked(approved.two_way())
                .into()
        }
        1 => {
            let progress = signal(65.0_f64);
            slider()
                .minimum(0.0)
                .maximum(100.0)
                .value(progress.two_way())
                .into()
        }
        2 => toggle_switch()
            .content("Pause animation")
            .is_checked(move || Some(paused.get()))
            .on_changed(
                ferroui_controls::primitives::ToggleButton::is_checked_property(),
                move |value: &Option<bool>| {
                    let next = value.unwrap_or(false);
                    paused.set_if_changed(next);
                    if let Some(handle) = handle.borrow().as_ref() {
                        handle.set_paused(next);
                    }
                },
            )
            .into(),
        3 => {
            let notes = signal(Some(String::from("Test the keyboard flow")));
            text_box().text(notes.two_way()).into()
        }
        4 => {
            let ready = signal(Some(false));
            check_box()
                .content("Ready for handoff")
                .is_checked(ready.two_way())
                .into()
        }
        _ => {
            let notes = signal(Some(true));
            check_box()
                .content("Include review notes")
                .is_checked(notes.two_way())
                .into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_board_wraps_without_overlapping_cards() {
        for (width, columns) in [(400.0, 1), (640.0, 2), (1024.0, 3)] {
            let (points, height) = board_layout(width);
            for (i, point) in points.iter().enumerate() {
                assert!(point.x as f64 + CARD_W <= width);
                assert!(point.y as f64 + CARD_H <= height);
                assert_eq!(
                    point.x as f64,
                    24.0 + (i % columns) as f64 * (CARD_W + 24.0)
                );
            }
        }
    }
}
