//! **Rhino Grasshopper-style Parametric Physics Studio** wired directly to `ferroui-physics`!
//!
//! Split workspace combining:
//! - Left/Main: Full interactive `ferroui-nodegraph` Grasshopper canvas + Component Ribbon +
//!   inline sliders/toggles/panels + Fancy Wires + Minimap + DataTree Inspector
//! - Right: Live hardware-accelerated `PhysicsViewport` running the compiled output of the node
//!   graph in real time (drag sliders on the canvas and watch the physics world morph live!)

use crate::scenes::SandboxEngine;
use crate::viewport::PhysicsViewport;
use ferroui_base::layout::Orientation;
use ferroui_controls::Dock;
use ferroui_nodegraph::prelude::*;
use ferroui_physics::Vec2;
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Builds the full split-screen Grasshopper Parametric Physics Studio tab.
pub fn node_graph_physics_studio() -> Element {
    let registry = Rc::new(NodeRegistry::with_standard_library());
    let active_template = signal(PhysicsGraphTemplate::AttractorGraphMapper);
    let live_rebuild = signal(true);
    let preview_paused = signal(false);
    let summary_text = signal(String::from("Compiled Grasshopper scene live!"));
    let template_version = signal(0u64);

    // Shared graph & live preview physics engine
    let initial_graph = {
        let mut g = PhysicsGraphTemplate::AttractorGraphMapper.build(&registry);
        let _ = evaluate_graph(&mut g, &registry, 0.0);
        g
    };
    let graph = Rc::new(RefCell::new(initial_graph));
    let preview_engine = Rc::new(RefCell::new(SandboxEngine::new()));

    // Compile initial blueprint into the live PhysicsViewport
    if let Some(bp) = extract_physics_blueprint(&graph.borrow()) {
        preview_engine.borrow_mut().load_from_blueprint(&bp);
    }

    // Callback invoked on every slider drag, wire edit, or node edit in the Grasshopper canvas!
    let eng_sync = preview_engine.clone();
    let on_graph_evaluated: Rc<dyn Fn(&NodeGraph)> = Rc::new(move |g: &NodeGraph| {
        if let Some(bp) = extract_physics_blueprint(g) {
            let msg = format!(
                "{} bodies · {} joints · {} fields · {} soft · {} cloth",
                bp.bodies.len(),
                bp.joints.len(),
                bp.force_fields.len(),
                bp.soft_bodies.len(),
                bp.cloths.len(),
            );
            if summary_text.is_alive() {
                summary_text.set_if_changed(msg);
            }
            if live_rebuild.get_untracked() {
                let mut sim = eng_sync.borrow_mut();
                sim.load_from_blueprint(&bp);
                sim.paused = preview_paused.get_untracked();
            }
        }
    });

    // -------------------------------------------------------------------------
    // 1. Top Template & Live-Sync Bar
    // -------------------------------------------------------------------------
    let mut tpl_row = stack_panel().orientation(Orientation::Horizontal).spacing(5.0);
    for tpl in PhysicsGraphTemplate::ALL {
        let g_t = graph.clone();
        let r_t = registry.clone();
        let eng_t = preview_engine.clone();
        tpl_row = tpl_row.child(
            button()
                .padding((10.0, 5.0, 10.0, 5.0))
                .corner_radius(6.0)
                .background(move || if active_template.get() == tpl { "#7C3AED" } else { "#182235" })
                .foreground("#F8FAFC")
                .content(tpl.label())
                .on_click(move |_| {
                    let mut next = tpl.build(&r_t);
                    let _ = evaluate_graph(&mut next, &r_t, 0.0);
                    if let Some(bp) = extract_physics_blueprint(&next) {
                        eng_t.borrow_mut().load_from_blueprint(&bp);
                    }
                    *g_t.borrow_mut() = next;
                    active_template.set(tpl);
                    preview_paused.set(false);
                    template_version.update(|v| *v = v.wrapping_add(1));
                }),
        );
    }

    let g_reb = graph.clone();
    let r_reb = registry.clone();
    let eng_reb = preview_engine.clone();
    let eng_pause = preview_engine.clone();
    let eng_blast = preview_engine.clone();

    let controls_row = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(5.0)
        .child(
            button()
                .padding((9.0, 5.0, 9.0, 5.0))
                .corner_radius(6.0)
                .background(move || if live_rebuild.get() { "#059669" } else { "#334155" })
                .foreground("#FFFFFF")
                .content(move || if live_rebuild.get() { "⚡ Live Sync: ON" } else { "⚡ Live Sync: OFF" })
                .on_click(move |_| live_rebuild.update(|v| *v = !*v)),
        )
        .child(
            button()
                .padding((9.0, 5.0, 9.0, 5.0))
                .corner_radius(6.0)
                .background("#0284C7")
                .foreground("#FFFFFF")
                .content("↺ Rebuild Sim")
                .on_click(move |_| {
                    let mut g = g_reb.borrow_mut();
                    g.mark_all_dirty();
                    let _ = evaluate_graph(&mut g, &r_reb, 0.0);
                    if let Some(bp) = extract_physics_blueprint(&g) {
                        eng_reb.borrow_mut().load_from_blueprint(&bp);
                        preview_paused.set(false);
                    }
                }),
        )
        .child(
            button()
                .padding((9.0, 5.0, 9.0, 5.0))
                .corner_radius(6.0)
                .background(move || if preview_paused.get() { "#059669" } else { "#1E293B" })
                .foreground("#FFFFFF")
                .content(move || if preview_paused.get() { "▶ Play" } else { "⏸ Pause" })
                .on_click(move |_| {
                    let next = !preview_paused.get_untracked();
                    eng_pause.borrow_mut().paused = next;
                    preview_paused.set(next);
                }),
        )
        .child(
            button()
                .padding((9.0, 5.0, 9.0, 5.0))
                .corner_radius(6.0)
                .background("#D97706")
                .foreground("#FFFFFF")
                .content("💥 Blast")
                .on_click(move |_| {
                    eng_blast.borrow_mut().world.apply_explosion(Vec2::new(8.0, 5.5), 6.5, 16.0);
                }),
        );

    let top_bar = border()
        .dock_panel_dock(Dock::Top)
        .background("#101726")
        .border_brush("#24314A")
        .border_thickness((0.0, 0.0, 0.0, 1.0))
        .padding((12.0, 8.0, 12.0, 8.0))
        .child(dock_panel().child(controls_row.dock_panel_dock(Dock::Right)).child(tpl_row));

    // -------------------------------------------------------------------------
    // 2. Right Live Hardware-Rendered Physics Preview Dock
    // -------------------------------------------------------------------------
    let eng_vp = preview_engine.clone();
    let preview_tick_cb: Rc<dyn Fn(&SandboxEngine)> = Rc::new(|_| {});

    let live_preview_pane = border()
        .dock_panel_dock(Dock::Right)
        .width(500.0)
        .background("#080C14")
        .border_brush("#24314A")
        .border_thickness((1.0, 0.0, 0.0, 0.0))
        .clip_to_bounds(true)
        .child(
            dock_panel()
                .child(
                    border()
                        .dock_panel_dock(Dock::Top)
                        .background("#0D1321")
                        .border_brush("#1E293B")
                        .border_thickness((0.0, 0.0, 0.0, 1.0))
                        .padding((10.0, 6.0, 10.0, 6.0))
                        .child(
                            stack_panel()
                                .spacing(2.0)
                                .child(
                                    text_block()
                                        .text("⚡ LIVE GRASSHOPPER → PHYSICS VIEWPORT")
                                        .font_size(11.0)
                                        .foreground("#34D399"),
                                )
                                .child(
                                    text_block()
                                        .text(move || summary_text.get())
                                        .font_size(10.5)
                                        .foreground("#94A3B8"),
                                ),
                        ),
                )
                .child(
                    El::<PhysicsViewport>::new()
                        .with(move |ctrl| {
                            ctrl.bind_engine(eng_vp.clone(), preview_tick_cb.clone());
                        })
                        .into_element(),
                ),
        );

    // -------------------------------------------------------------------------
    // 3. Left/Main Interactive Rhino Grasshopper Node Graph Editor
    // -------------------------------------------------------------------------
    let g_ed = graph.clone();
    let r_ed = registry.clone();
    let cb_ed = on_graph_evaluated.clone();

    let editor_pane = border().clip_to_bounds(true).child(dynamic(move || {
        let _ = template_version.get();
        node_graph_editor(g_ed.clone(), r_ed.clone(), cb_ed.clone())
    }));

    dock_panel()
        .background("#090D16")
        .clip_to_bounds(true)
        .child(top_bar)
        .child(live_preview_pane)
        .child(editor_pane)
        .into()
}
