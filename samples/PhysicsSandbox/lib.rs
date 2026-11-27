//! **FerroUI Physics & Compositor Studio** — Organized in the clean `ControlCatalog`
//! WinUI 3 `NavigationView` + vector `PathIcon` gallery style.

pub mod compositor_showcase;
pub mod icons;
pub mod live_ui_tree;
pub mod node_graph_studio;
pub mod scenes;
pub mod viewport;

use crate::icons::{icon_label, icon_view, nav_icon, Icons};
use crate::scenes::{SandboxEngine, SceneKind, SolverProfile, SpringPreset, ToolMode};
use crate::viewport::PhysicsViewport;
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{FontWeight, IBrush, TextWrapping};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{
    Application, Control, Dock, NavigationView, NavigationViewBackButtonVisible,
    NavigationViewItem, NavigationViewItemHeader, NavigationViewPaneDisplayMode,
};
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

fn solid(argb: u32) -> Option<Rc<dyn IBrush>> {
    Some(Rc::new(ImmutableSolidColorBrush::from_uint32(argb)) as Rc<dyn IBrush>)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StudioPage {
    Home,
    CompositorShaders,
    LiveUiTree,
    NodeGraph,
    Scene(SceneKind),
}

impl StudioPage {
    fn title(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::CompositorShaders => "Compositor & Shaders",
            Self::LiveUiTree => "Live UI Tree",
            Self::NodeGraph => "Grasshopper Studio",
            Self::Scene(kind) => kind.title(),
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Home => Icons::HOME,
            Self::CompositorShaders => Icons::SPARKLE,
            Self::LiveUiTree => Icons::LAYERS,
            Self::NodeGraph => Icons::NODE_GRAPH,
            Self::Scene(SceneKind::Playground) => Icons::CUBE,
            Self::Scene(SceneKind::GpuParticles) => Icons::WATER,
            Self::Scene(SceneKind::ClothSoftBody) => Icons::CLOTH,
            Self::Scene(SceneKind::UiSprings) => Icons::UI_MOTION,
            Self::Scene(SceneKind::RagdollWrecking) => Icons::SHATTER,
            Self::Scene(SceneKind::VehicleTerrain) => Icons::VEHICLE,
            Self::Scene(SceneKind::CharacterDemo) => Icons::PERSON,
            Self::Scene(SceneKind::LaserGravity) => Icons::ORBIT,
        }
    }
}

fn scene_description(kind: SceneKind) -> &'static str {
    match kind {
        SceneKind::Playground => "Stacked rigid-body pyramids, bridge chains, and interactive drag/shatter tools.",
        SceneKind::GpuParticles => "Up to 8K hardware GPU compute (wgpu) spatial-hash fluid particles with obstacle interaction.",
        SceneKind::ClothSoftBody => "Tearable XPBD cloth mesh with wind gusts and pressure-inflated soft-body jelly.",
        SceneKind::UiSprings => "Analytical harmonic UI springs, magnetic snap anchors, and rubber-band inertia.",
        SceneKind::RagdollWrecking => "Breakable joints and radial convex shard fracturing under blast shockwaves.",
        SceneKind::VehicleTerrain => "Wheel-joint suspension rig with live motor torque over bumpy terrain.",
        SceneKind::CharacterDemo => "Swept shape-cast move-and-slide controller with slopes, jumps, and box pushing.",
        SceneKind::LaserGravity => "Keplerian inverse-square orbital gravity with live multi-bounce laser raycasts.",
    }
}

/// Clean stock-themed inspector card matching `ControlCatalog` gallery styling.
fn inspector_card(title: &'static str, body: impl IntoElement) -> Element {
    border()
        .background(solid(0x14808080))
        .border_brush(solid(0x28808080))
        .border_thickness(1.0)
        .corner_radius(8.0)
        .padding(12.0)
        .child(
            stack_panel()
                .spacing(8.0)
                .child(
                    text_block()
                        .text(title)
                        .font_size(12.0)
                        .font_weight(FontWeight::SemiBold),
                )
                .child(body),
        )
        .into()
}

/// Builds a ControlCatalog-style Home gallery tile card.
fn gallery_card(
    page: StudioPage,
    description: &'static str,
    accent_argb: u32,
    on_select: Rc<dyn Fn(StudioPage)>,
) -> Element {
    let badge_bg = (accent_argb & 0x00FF_FFFF) | 0x2400_0000;
    button()
        .width(268.0)
        .padding(14.0)
        .horizontal_content_alignment(HorizontalAlignment::Stretch)
        .vertical_content_alignment(VerticalAlignment::Top)
        .content(
            stack_panel()
                .spacing(10.0)
                .child(
                    border()
                        .width(36.0)
                        .height(36.0)
                        .corner_radius(8.0)
                        .background(solid(badge_bg))
                        .horizontal_alignment(HorizontalAlignment::Left)
                        .child(
                            border()
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .vertical_alignment(VerticalAlignment::Center)
                                .child(icon_view(page.icon(), 18.0)),
                        ),
                )
                .child(
                    stack_panel()
                        .spacing(3.0)
                        .child(
                            text_block()
                                .text(page.title())
                                .font_size(14.0)
                                .font_weight(FontWeight::SemiBold),
                        )
                        .child(
                            text_block()
                                .text(description)
                                .font_size(11.5)
                                .opacity(0.72)
                                .text_wrapping(TextWrapping::Wrap),
                        ),
                ),
        )
        .on_click(move |_| on_select(page))
        .into()
}

fn stat_pill(value: &'static str, label: &'static str, accent_argb: u32) -> Element {
    let tint = (accent_argb & 0x00FF_FFFF) | 0x2200_0000;
    border()
        .background(solid(tint))
        .border_brush(solid((accent_argb & 0x00FF_FFFF) | 0x5500_0000))
        .border_thickness(1.0)
        .corner_radius(8.0)
        .padding((14.0, 10.0, 14.0, 10.0))
        .child(
            stack_panel()
                .spacing(2.0)
                .child(text_block().text(value).font_size(15.0).font_weight(FontWeight::SemiBold))
                .child(text_block().text(label).font_size(11.0).opacity(0.75)),
        )
        .into()
}

fn build_home_page(gpu_available: bool, navigate: Rc<dyn Fn(StudioPage)>) -> Element {
    let hero = border()
        .corner_radius(12.0)
        .background(solid(0x160078D4))
        .border_brush(solid(0x380078D4))
        .border_thickness(1.0)
        .padding(22.0)
        .child(
            stack_panel()
                .spacing(14.0)
                .child(
                    stack_panel()
                        .spacing(5.0)
                        .child(
                            text_block()
                                .text("FerroUI Physics & Compositor Studio")
                                .font_size(26.0)
                                .font_weight(FontWeight::SemiBold),
                        )
                        .child(
                            text_block()
                                .text("Hardware-accelerated 2D physics, multi-mode Compositor simulations, live WGSL → Metal GPU shaders, and Rhino Grasshopper parametric node graphs.")
                                .font_size(13.0)
                                .opacity(0.78)
                                .text_wrapping(TextWrapping::Wrap),
                        ),
                )
                .child(
                    wrap_panel()
                        .item_spacing(10.0)
                        .line_spacing(10.0)
                        .child(stat_pill(
                            if gpu_available { "Metal GPU" } else { "CPU SIMD" },
                            "Compute particle backend",
                            0xFF0EA5E9,
                        ))
                        .child(stat_pill("WGSL → MSL", "Live runtime GPU shaders", 0xFF8B5CF6))
                        .child(stat_pill("ICompositionSimulation", "Multi-mode compositor contract", 0xFF10B981))
                        .child(stat_pill("65+ Nodes", "Stock-themed Grasshopper editor", 0xFFF59E0B)),
                ),
        );

    let compositor_section = stack_panel()
        .spacing(12.0)
        .child(
            text_block()
                .text("Compositor, GPU Shaders & Parametric Graph")
                .font_size(18.0)
                .font_weight(FontWeight::SemiBold),
        )
        .child(
            wrap_panel()
                .item_spacing(12.0)
                .line_spacing(12.0)
                .child(gallery_card(
                    StudioPage::CompositorShaders,
                    "Direct CompositionVisual physics, elastic springs, non-physics harmonic waves, and 3 live WGSL/SkSL GPU shaders.",
                    0xFF0EA5E9,
                    navigate.clone(),
                ))
                .child(gallery_card(
                    StudioPage::LiveUiTree,
                    "Apply rigid bodies, springs, shockwaves, and tactile gestures directly to real FerroUI visual & logical trees.",
                    0xFF10B981,
                    navigate.clone(),
                ))
                .child(gallery_card(
                    StudioPage::NodeGraph,
                    "Visual Grasshopper-style node editor in the stock FerroUI theme wired live into the physics viewport.",
                    0xFF8B5CF6,
                    navigate.clone(),
                )),
        );

    let mut sandbox_cards = wrap_panel().item_spacing(12.0).line_spacing(12.0);
    let accents = [
        0xFF0EA5E9, 0xFF06B6D4, 0xFF10B981, 0xFF8B5CF6,
        0xFFEF4444, 0xFFF59E0B, 0xFFEC4899, 0xFF3B82F6,
    ];
    for (idx, kind) in SceneKind::ALL.iter().copied().enumerate() {
        sandbox_cards = sandbox_cards.child(gallery_card(
            StudioPage::Scene(kind),
            scene_description(kind),
            accents[idx % accents.len()],
            navigate.clone(),
        ));
    }

    let sandbox_section = stack_panel()
        .spacing(12.0)
        .child(
            text_block()
                .text("Hardware-Accelerated 2D Physics Scenes")
                .font_size(18.0)
                .font_weight(FontWeight::SemiBold),
        )
        .child(sandbox_cards);

    scroll_viewer()
        .padding((28.0, 22.0, 28.0, 28.0))
        .content(
            stack_panel()
                .spacing(26.0)
                .child(hero)
                .child(compositor_section)
                .child(sandbox_section),
        )
        .into()
}

/// Builds the full **ControlCatalog-style FerroUI Physics & Compositor Studio**.
pub fn physics_sandbox() -> Element {
    let engine = Rc::new(RefCell::new(SandboxEngine::new()));
    let gpu_available = engine.borrow().gpu_available();

    let initial_page = match std::env::var("FERROUI_START_PAGE").as_deref() {
        Ok("compositor") => StudioPage::CompositorShaders,
        Ok("ui") => StudioPage::LiveUiTree,
        Ok("nodegraph") => StudioPage::NodeGraph,
        Ok("playground") => StudioPage::Scene(SceneKind::Playground),
        _ => StudioPage::Home,
    };
    let active_page = signal(initial_page);
    let active_scene = signal(SceneKind::Playground);
    let active_tool = signal(ToolMode::Drag);
    let active_preset = signal(SpringPreset::Bouncy);
    let active_solver = signal(SolverProfile::Game);
    let active_pcount = signal(4096usize);
    let paused = signal(false);
    let prefer_gpu = signal(gpu_available);
    let is_dark_theme = signal(
        Application::current()
            .and_then(|a| a.actual_theme_variant())
            .is_none_or(|v| v == ThemeVariant::dark()),
    );

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

    // Track NavigationView + menu item handles so gallery Home cards sync sidebar selection!
    let nav_ref = NodeRef::<NavigationView>::new();
    let nav_entries: Rc<RefCell<Vec<(StudioPage, Ref<NavigationViewItem>)>>> = Rc::new(RefCell::new(Vec::new()));
    let syncing_nav = Rc::new(std::cell::Cell::new(false));

    let navigate_to: Rc<dyn Fn(StudioPage)> = {
        let eng = engine.clone();
        let nav_entries = nav_entries.clone();
        let syncing_nav = syncing_nav.clone();
        Rc::new(move |page: StudioPage| {
            if let StudioPage::Scene(kind) = page {
                {
                    let mut sim = eng.borrow_mut();
                    sim.blueprint_driven = false;
                    sim.load_scene(kind);
                }
                active_scene.set(kind);
                paused.set(false);
            }
            active_page.set(page);

            if let Some(nav) = nav_ref.get_untracked() {
                if let Some((_, item)) = nav_entries.borrow().iter().find(|(p, _)| *p == page) {
                    syncing_nav.set(true);
                    nav.set_selected_item(Some(Control::boxed(item)));
                    syncing_nav.set(false);
                }
            }
        })
    };

    // -------------------------------------------------------------------------
    // Build the Canvas Physics Sandbox Page (with clean ControlCatalog chrome)
    // -------------------------------------------------------------------------
    let build_sandbox_page = {
        let engine = engine.clone();
        move || -> Element {
            let eng_back = engine.clone();
            let eng_pause = engine.clone();
            let eng_fwd = engine.clone();
            let eng_reset = engine.clone();
            let eng_burst = engine.clone();

            let transport = stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(5.0)
                .child(
                    button()
                        .padding((8.0, 5.0, 8.0, 5.0))
                        .content(icon_view(Icons::STEP_BACK, 14.0))
                        .on_click(move |_| {
                            eng_back.borrow_mut().step_back_history();
                            paused.set(true);
                        }),
                )
                .child(
                    button()
                        .padding((10.0, 5.0, 10.0, 5.0))
                        .border_brush(move || if paused.get() { solid(0xFF10B981) } else { solid(0xFF0078D4) })
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
                        .content(icon_view(Icons::STEP_FWD, 14.0))
                        .on_click(move |_| {
                            eng_fwd.borrow_mut().step_forward_one();
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
                        .content(icon_label(Icons::SPARKLE, "Burst", 14.0))
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
            let mut tools_row = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
            for (tool, ic) in tool_specs {
                let eng = engine.clone();
                tools_row = tools_row.child(
                    button()
                        .padding((9.0, 4.0, 9.0, 4.0))
                        .border_brush(move || if active_tool.get() == tool { solid(0xFF0078D4) } else { None })
                        .border_thickness(move || if active_tool.get() == tool { 1.5 } else { 1.0 })
                        .font_size(11.5)
                        .content(icon_label(ic, tool.label(), 13.0))
                        .on_click(move |_| {
                            eng.borrow_mut().tool = tool;
                            active_tool.set(tool);
                        }),
                );
            }

            let scene_header = border()
                .dock_panel_dock(Dock::Top)
                .padding((18.0, 12.0, 18.0, 10.0))
                .border_brush(solid(0x22808080))
                .border_thickness((0.0, 0.0, 0.0, 1.0))
                .child(
                    stack_panel()
                        .spacing(8.0)
                        .child(
                            dock_panel()
                                .child(transport.dock_panel_dock(Dock::Right))
                                .child(
                                    stack_panel()
                                        .spacing(2.0)
                                        .child(
                                            text_block()
                                                .text(move || active_scene.get().title())
                                                .font_size(19.0)
                                                .font_weight(FontWeight::SemiBold),
                                        )
                                        .child(
                                            text_block()
                                                .text(move || active_scene.get().hint())
                                                .font_size(11.5)
                                                .opacity(0.72),
                                        ),
                                ),
                        )
                        .child(tools_row),
                );

            // Right Inspector (clean stock theme cards)
            let adapter_name = engine.borrow().gpu_adapter_name().unwrap_or("CPU SIMD Fallback").to_string();
            let eng_gpu = engine.clone();
            let mut pcount_row = stack_panel().orientation(Orientation::Horizontal).spacing(5.0);
            for count in [2048usize, 4096, 8192] {
                let eng = engine.clone();
                let label = format!("{}K", count / 1024);
                pcount_row = pcount_row.child(
                    button()
                        .padding((9.0, 4.0, 9.0, 4.0))
                        .border_brush(move || if active_pcount.get() == count { solid(0xFF0078D4) } else { None })
                        .border_thickness(move || if active_pcount.get() == count { 1.5 } else { 1.0 })
                        .font_size(11.0)
                        .content(label)
                        .on_click(move |_| {
                            eng.borrow_mut().set_particle_count(count);
                            active_pcount.set(count);
                        }),
                );
            }
            let mut solver_row = stack_panel().orientation(Orientation::Horizontal).spacing(5.0);
            for prof in SolverProfile::ALL {
                let eng = engine.clone();
                solver_row = solver_row.child(
                    button()
                        .padding((8.0, 4.0, 8.0, 4.0))
                        .border_brush(move || if active_solver.get() == prof { solid(0xFF0078D4) } else { None })
                        .border_thickness(move || if active_solver.get() == prof { 1.5 } else { 1.0 })
                        .font_size(11.0)
                        .content(prof.label())
                        .on_click(move |_| {
                            eng.borrow_mut().set_solver_profile(prof);
                            active_solver.set(prof);
                        }),
                );
            }

            let hardware_card = inspector_card(
                "Hardware & Solver",
                stack_panel()
                    .spacing(7.0)
                    .child(text_block().text(format!("Adapter: {adapter_name}")).font_size(11.0).opacity(0.75))
                    .child(
                        button()
                            .horizontal_alignment(HorizontalAlignment::Stretch)
                            .padding((10.0, 5.0, 10.0, 5.0))
                            .border_brush(move || if prefer_gpu.get() { solid(0xFF0EA5E9) } else { None })
                            .content(move || {
                                if prefer_gpu.get() {
                                    "Backend: GPU Compute (wgpu)"
                                } else {
                                    "Backend: CPU SIMD Fallback"
                                }
                            })
                            .on_click(move |_| {
                                let next = !prefer_gpu.get_untracked();
                                eng_gpu.borrow_mut().prefer_gpu = next;
                                prefer_gpu.set(next);
                            }),
                    )
                    .child(text_block().text("GPU Fluid Particles").font_size(10.5).opacity(0.72))
                    .child(pcount_row)
                    .child(text_block().text("Rigid Solver Profile").font_size(10.5).opacity(0.72))
                    .child(solver_row),
            );

            let eng_scrub = engine.clone();
            let timeline_card = inspector_card(
                "Time-Travel & Speed",
                stack_panel()
                    .spacing(5.0)
                    .child(text_block().text(move || format!("History: {}", telemetry_history.get())).font_size(11.0).opacity(0.78))
                    .child(
                        slider()
                            .minimum(0.0)
                            .maximum(1.0)
                            .value(timeline.two_way())
                            .on_changed(ferroui_controls::primitives::RangeBase::value_property(), move |v| {
                                let mut sim = eng_scrub.borrow_mut();
                                if sim.paused && !sim.history.is_empty() {
                                    sim.scrub_history(*v as f32);
                                }
                            }),
                    )
                    .child(text_block().text(move || format!("Time Scale: {:.2}×", time_scale.get())).font_size(11.0).opacity(0.78))
                    .child(slider().minimum(0.1).maximum(2.0).value(time_scale.two_way())),
            );

            let env_card = inspector_card(
                "Environment",
                stack_panel()
                    .spacing(5.0)
                    .child(text_block().text(move || format!("Gravity: {:.1} m/s²", gravity.get())).font_size(11.0).opacity(0.78))
                    .child(slider().minimum(-15.0).maximum(25.0).value(gravity.two_way()))
                    .child(text_block().text(move || format!("Bounciness: {:.2}", restitution.get())).font_size(11.0).opacity(0.78))
                    .child(slider().minimum(0.0).maximum(0.95).value(restitution.two_way()))
                    .child(text_block().text(move || format!("Wind Gust: {:.1} m/s²", wind_x.get())).font_size(11.0).opacity(0.78))
                    .child(slider().minimum(-12.0).maximum(12.0).value(wind_x.two_way())),
            );

            let mut preset_wrap = wrap_panel().item_spacing(5.0).line_spacing(5.0);
            for preset in SpringPreset::ALL {
                let eng = engine.clone();
                preset_wrap = preset_wrap.child(
                    button()
                        .padding((8.0, 4.0, 8.0, 4.0))
                        .border_brush(move || if active_preset.get() == preset { solid(0xFF8B5CF6) } else { None })
                        .border_thickness(move || if active_preset.get() == preset { 1.5 } else { 1.0 })
                        .font_size(11.0)
                        .content(preset.label())
                        .on_click(move |_| {
                            eng.borrow_mut().set_spring_preset(preset);
                            active_preset.set(preset);
                        }),
                );
            }
            let spring_card = inspector_card("UI Spring Preset", preset_wrap);

            let make_dbg = |label: &'static str, sig: Signal<bool>, setter: fn(&mut SandboxEngine, bool)| {
                let eng = engine.clone();
                button()
                    .padding((8.0, 4.0, 8.0, 4.0))
                    .border_brush(move || if sig.get() { solid(0xFF10B981) } else { None })
                    .border_thickness(move || if sig.get() { 1.5 } else { 1.0 })
                    .font_size(11.0)
                    .content(label)
                    .on_click(move |_| {
                        let next = !sig.get_untracked();
                        setter(&mut eng.borrow_mut(), next);
                        sig.set(next);
                    })
            };
            let debug_card = inspector_card(
                "Viewport Overlays",
                wrap_panel()
                    .item_spacing(5.0)
                    .line_spacing(5.0)
                    .child(make_dbg("BVH AABB", show_aabbs, |s, v| s.show_aabbs = v))
                    .child(make_dbg("Contacts", show_contacts, |s, v| s.show_contacts = v))
                    .child(make_dbg("Velocities", show_velocities, |s, v| s.show_velocities = v))
                    .child(make_dbg("Joints", show_joints, |s, v| s.show_joints = v))
                    .child(make_dbg("HUD Graph", show_hud, |s, v| s.show_hud_graph = v)),
            );

            let inspector = border()
                .dock_panel_dock(Dock::Right)
                .width(276.0)
                .border_brush(solid(0x22808080))
                .border_thickness((1.0, 0.0, 0.0, 0.0))
                .padding(12.0)
                .child(
                    scroll_viewer().content(
                        stack_panel()
                            .spacing(10.0)
                            .child(hardware_card)
                            .child(timeline_card)
                            .child(env_card)
                            .child(spring_card)
                            .child(debug_card),
                    ),
                );

            let status_bar = border()
                .dock_panel_dock(Dock::Bottom)
                .border_brush(solid(0x22808080))
                .border_thickness((0.0, 1.0, 0.0, 0.0))
                .padding((16.0, 6.0, 16.0, 6.0))
                .child(
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .spacing(16.0)
                        .child(text_block().text(move || telemetry_backend.get()).font_size(11.0).font_weight(FontWeight::SemiBold))
                        .child(text_block().text(move || format!("Step {:.2} ms", telemetry_step_ms.get())).font_size(11.0).opacity(0.85))
                        .child(text_block().text(move || telemetry_breakdown.get()).font_size(11.0).opacity(0.72))
                        .child(text_block().text(move || telemetry_entities.get()).font_size(11.0).opacity(0.8)),
                );

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
    };

    // -------------------------------------------------------------------------
    // Root ControlCatalog-style WinUI 3 NavigationView Shell
    // -------------------------------------------------------------------------
    let nav_entries_setup = nav_entries.clone();
    let nav_entries_sel = nav_entries.clone();
    let syncing_sel = syncing_nav.clone();
    let nav_to_sel = navigate_to.clone();
    let nav_to_home = navigate_to.clone();

    let pane_header = border()
        .padding((8.0, 6.0, 8.0, 6.0))
        .child(
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(10.0)
                .vertical_alignment(VerticalAlignment::Center)
                .child(
                    border()
                        .width(28.0)
                        .height(28.0)
                        .corner_radius(7.0)
                        .background(solid(0xFF0078D4))
                        .child(
                            border()
                                .horizontal_alignment(HorizontalAlignment::Center)
                                .vertical_alignment(VerticalAlignment::Center)
                                .child(icon_view(Icons::LIGHTNING, 16.0)),
                        ),
                )
                .child(
                    stack_panel()
                        .spacing(0.0)
                        .child(text_block().text("FerroUI Physics").font_size(13.5).font_weight(FontWeight::SemiBold))
                        .child(
                            text_block()
                                .text(if gpu_available { "Metal GPU · WGSL Shaders" } else { "CPU SIMD · Shaders" })
                                .font_size(10.0)
                                .opacity(0.68),
                        ),
                ),
        );

    let pane_footer = border()
        .padding((8.0, 6.0, 8.0, 8.0))
        .child(
            button()
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .padding((10.0, 6.0, 10.0, 6.0))
                .content(dynamic(move || {
                    if is_dark_theme.get() {
                        icon_label(Icons::THEME, "Switch to Light Theme", 14.0)
                    } else {
                        icon_label(Icons::THEME, "Switch to Dark Theme", 14.0)
                    }
                }))
                .on_click(move |_| {
                    let next_dark = !is_dark_theme.get_untracked();
                    is_dark_theme.set(next_dark);
                    if let Some(app) = Application::current() {
                        app.set_requested_theme_variant(Some(if next_dark {
                            ThemeVariant::dark()
                        } else {
                            ThemeVariant::light()
                        }));
                    }
                }),
        );

    let page_content = border().clip_to_bounds(true).child(dynamic(move || match active_page.get() {
        StudioPage::Home => build_home_page(gpu_available, nav_to_home.clone()),
        StudioPage::CompositorShaders => compositor_showcase::compositor_shader_showcase(),
        StudioPage::LiveUiTree => live_ui_tree::live_ui_tree_showcase(),
        StudioPage::NodeGraph => node_graph_studio::node_graph_physics_studio(),
        StudioPage::Scene(_) => build_sandbox_page(),
    }));

    navigation_view()
        .node_ref(nav_ref)
        .pane_display_mode(NavigationViewPaneDisplayMode::Auto)
        .compact_mode_threshold_width(680.0)
        .expanded_mode_threshold_width(980.0)
        .open_pane_length(248.0)
        .is_settings_visible(false)
        .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
        .always_show_header(false)
        .pane_header(pane_header)
        .pane_footer(pane_footer)
        .content(page_content)
        .with(move |nav: &Ref<NavigationView>| {
            let mut entries = nav_entries_setup.borrow_mut();
            let add_header = |title: &str| {
                let hdr = NavigationViewItemHeader::new();
                hdr.set_content(Some(Rc::new(title.to_string()) as BoxedValue));
                nav.menu_items().add(Some(Control::boxed(&hdr)));
            };
            let mut add_item = |page: StudioPage| -> Ref<NavigationViewItem> {
                let item = NavigationViewItem::new();
                item.set_content(Some(Rc::new(page.title().to_string()) as BoxedValue));
                if let Some(ic) = nav_icon(page.icon()) {
                    item.set_icon(Some(ic));
                }
                nav.menu_items().add(Some(Control::boxed(&item)));
                entries.push((page, item.clone()));
                item
            };

            let home_item = add_item(StudioPage::Home);

            add_header("Compositor & UI");
            let comp_item = add_item(StudioPage::CompositorShaders);
            let ui_item = add_item(StudioPage::LiveUiTree);

            add_header("Parametric Graph");
            let ng_item = add_item(StudioPage::NodeGraph);

            add_header("2D Physics Sandbox");
            let mut first_scene_item = None;
            for kind in SceneKind::ALL {
                let it = add_item(StudioPage::Scene(kind));
                if first_scene_item.is_none() {
                    first_scene_item = Some(it);
                }
            }
            drop(entries);

            let initial_item = match initial_page {
                StudioPage::CompositorShaders => &comp_item,
                StudioPage::LiveUiTree => &ui_item,
                StudioPage::NodeGraph => &ng_item,
                StudioPage::Scene(_) => first_scene_item.as_ref().unwrap_or(&home_item),
                StudioPage::Home => &home_item,
            };
            syncing_sel.set(true);
            nav.set_selected_item(Some(Control::boxed(initial_item)));
            syncing_sel.set(false);

            let entries_cb = nav_entries_sel.clone();
            let syncing_cb = syncing_sel.clone();
            let nav_to = nav_to_sel.clone();
            nav.selection_changed(move |args| {
                if syncing_cb.get() {
                    return;
                }
                if let Some(container) = args
                    .selected_item_container()
                    .and_then(|c| c.cast::<NavigationViewItem>())
                {
                    let found = entries_cb
                        .borrow()
                        .iter()
                        .find(|(_, item)| *item == container)
                        .map(|(page, _)| *page);
                    if let Some(page) = found {
                        nav_to(page);
                    }
                }
            })
            .detach();
        })
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
