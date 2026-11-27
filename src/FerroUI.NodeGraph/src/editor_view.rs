//! High-level reusable **Rhino Grasshopper Editor View** (`node_graph_editor`) combining:
//! - Categorized Component Ribbon (`Params`, `Math`, `Logic`, `Sets`, `Vector`, `Geometry`, `Physics`)
//! - Quick-Search Add Filter + click-to-drop component chips
//! - Graph Action Bar (`Auto-Layout`, `Fit`, `Group`, `Undo`, `Redo`, `Dup`, `Del`, `Fancy Wires`, `Profiler`, `JSON`)
//! - Hardware-accelerated [`NodeGraphCanvas`]
//! - Live Selected-Node Inspector with Grasshopper DataTree modifier toggles (`F` / `G` / `R` / `S`)

use crate::canvas::NodeGraphCanvas;
use crate::eval::evaluate_graph;
use crate::graph::NodeGraph;
use crate::node::NodeRegistry;
use crate::serialization::graph_to_json;
use crate::types::{DataMatching, NodeCategory, NodeId};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_controls::Dock;
use ferroui_view::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Builds a complete, drop-in Rhino Grasshopper-style node graph editor `Element` backed by
/// `graph` and `registry`. Calls `on_evaluated(&NodeGraph)` whenever the graph changes!
pub fn node_graph_editor(
    graph: Rc<RefCell<NodeGraph>>,
    registry: Rc<NodeRegistry>,
    on_evaluated: Rc<dyn Fn(&NodeGraph)>,
) -> Element {
    let categories = registry.categories();
    let active_cat = signal(categories.first().cloned().unwrap_or(NodeCategory::Params));
    let search_query = signal(String::new());
    let selected_id: Signal<Option<NodeId>> = signal(None);
    let inspector_tick = signal(0u64);
    let fancy_wires = signal(graph.borrow().fancy_wires);
    let show_profiler = signal(graph.borrow().show_profiler);
    let status_msg = signal(String::from(
        "Drag sliders directly on canvas · Wire ports (Shift-wire for multi-input) · Right-drag Pan · Wheel Zoom",
    ));
    let canvas_ref = NodeRef::<NodeGraphCanvas>::new();

    // Sync canvas callbacks to reactive inspector + user callback
    let on_eval_wrapped: Rc<dyn Fn(&NodeGraph)> = {
        let user_cb = on_evaluated.clone();
        Rc::new(move |g: &NodeGraph| {
            let sel = g.selected_nodes.iter().next().copied();
            if selected_id.is_alive() {
                selected_id.set_if_changed(sel);
                inspector_tick.update(|v| *v = v.wrapping_add(1));
            }
            user_cb(g);
        })
    };

    // -------------------------------------------------------------------------
    // 1. Top Grasshopper Component Ribbon & Action Bar
    // -------------------------------------------------------------------------
    let mut cat_tabs = stack_panel().orientation(Orientation::Horizontal).spacing(4.0);
    for cat in &categories {
        let c = cat.clone();
        let c_bg = c.clone();
        let c_cl = c.clone();
        cat_tabs = cat_tabs.child(
            button()
                .padding((9.0, 4.0, 9.0, 4.0))
                .corner_radius(5.0)
                .background(move || if active_cat.get() == c_bg { "#0284C7" } else { "#182235" })
                .foreground("#F1F5F9")
                .content(c.label().to_string())
                .on_click(move |_| {
                    search_query.set(String::new());
                    active_cat.set(c_cl.clone());
                }),
        );
    }

    let g_undo = graph.clone();
    let g_redo = graph.clone();
    let g_layout = graph.clone();
    let g_dup = graph.clone();
    let g_del = graph.clone();
    let g_grp = graph.clone();
    let g_fancy = graph.clone();
    let g_prof = graph.clone();
    let g_json = graph.clone();

    let action_bar = stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(4.0)
        .child(
            button()
                .padding((8.0, 4.0, 8.0, 4.0))
                .corner_radius(5.0)
                .background("#1E293B")
                .foreground("#E2E8F0")
                .content("↶ Undo")
                .on_click(move |_| {
                    if g_undo.borrow_mut().undo() {
                        if let Some(c) = canvas_ref.get_untracked() {
                            c.reevaluate_and_notify();
                        }
                    }
                }),
        )
        .child(
            button()
                .padding((8.0, 4.0, 8.0, 4.0))
                .corner_radius(5.0)
                .background("#1E293B")
                .foreground("#E2E8F0")
                .content("↷ Redo")
                .on_click(move |_| {
                    if g_redo.borrow_mut().redo() {
                        if let Some(c) = canvas_ref.get_untracked() {
                            c.reevaluate_and_notify();
                        }
                    }
                }),
        )
        .child(
            button()
                .padding((8.0, 4.0, 8.0, 4.0))
                .corner_radius(5.0)
                .background("#1E293B")
                .foreground("#7DD3FC")
                .content("⊞ Layout")
                .on_click(move |_| {
                    g_layout.borrow_mut().auto_layout();
                    if let Some(c) = canvas_ref.get_untracked() {
                        c.fit_view();
                    }
                    status_msg.set("Auto-arranged graph into topological columns.".into());
                }),
        )
        .child(
            button()
                .padding((8.0, 4.0, 8.0, 4.0))
                .corner_radius(5.0)
                .background("#1E293B")
                .foreground("#E2E8F0")
                .content("⊡ Fit")
                .on_click(move |_| {
                    if let Some(c) = canvas_ref.get_untracked() {
                        c.fit_view();
                    }
                }),
        )
        .child(
            button()
                .padding((8.0, 4.0, 8.0, 4.0))
                .corner_radius(5.0)
                .background("#1E293B")
                .foreground("#E2E8F0")
                .content("▢ Group")
                .on_click(move |_| {
                    if g_grp.borrow_mut().group_selection("Group", 0x2238BDF8).is_some() {
                        if let Some(c) = canvas_ref.get_untracked() {
                            c.invalidate_visual();
                        }
                        status_msg.set("Grouped selected nodes.".into());
                    }
                }),
        )
        .child(
            button()
                .padding((8.0, 4.0, 8.0, 4.0))
                .corner_radius(5.0)
                .background("#1E293B")
                .foreground("#E2E8F0")
                .content("⧉ Dup")
                .on_click(move |_| {
                    if !g_dup.borrow_mut().duplicate_selection().is_empty() {
                        if let Some(c) = canvas_ref.get_untracked() {
                            c.reevaluate_and_notify();
                        }
                    }
                }),
        )
        .child(
            button()
                .padding((8.0, 4.0, 8.0, 4.0))
                .corner_radius(5.0)
                .background("#3F1D2B")
                .foreground("#FDA4AF")
                .content("✕ Del")
                .on_click(move |_| {
                    if g_del.borrow_mut().delete_selection() {
                        if let Some(c) = canvas_ref.get_untracked() {
                            c.reevaluate_and_notify();
                        }
                    }
                }),
        )
        .child(
            button()
                .padding((8.0, 4.0, 8.0, 4.0))
                .corner_radius(5.0)
                .background(move || if fancy_wires.get() { "#0369A1" } else { "#1E293B" })
                .foreground("#FFFFFF")
                .content("Fancy Wires")
                .on_click(move |_| {
                    let next = !fancy_wires.get_untracked();
                    g_fancy.borrow_mut().fancy_wires = next;
                    fancy_wires.set(next);
                    if let Some(c) = canvas_ref.get_untracked() {
                        c.invalidate_visual();
                    }
                }),
        )
        .child(
            button()
                .padding((8.0, 4.0, 8.0, 4.0))
                .corner_radius(5.0)
                .background(move || if show_profiler.get() { "#059669" } else { "#1E293B" })
                .foreground("#FFFFFF")
                .content("Profiler")
                .on_click(move |_| {
                    let next = !show_profiler.get_untracked();
                    g_prof.borrow_mut().show_profiler = next;
                    show_profiler.set(next);
                    if let Some(c) = canvas_ref.get_untracked() {
                        c.invalidate_visual();
                    }
                }),
        )
        .child(
            button()
                .padding((8.0, 4.0, 8.0, 4.0))
                .corner_radius(5.0)
                .background("#1E293B")
                .foreground("#A7F3D0")
                .content("{ } JSON")
                .on_click(move |_| {
                    let json = graph_to_json(&g_json.borrow());
                    status_msg.set(format!(
                        "Serialized definition cleanly ({} bytes, {} nodes, {} wires).",
                        json.len(),
                        g_json.borrow().nodes.len(),
                        g_json.borrow().wires.len()
                    ));
                }),
        );

    // Component Shelf (click any chip to spawn right into canvas center!)
    let reg_shelf = registry.clone();
    let g_shelf = graph.clone();
    let shelf = dynamic(move || {
        let q = search_query.get();
        let cat = active_cat.get();
        let items = if q.trim().is_empty() {
            reg_shelf.by_category(&cat)
        } else {
            reg_shelf.search(&q)
        };
        let mut row = stack_panel().orientation(Orientation::Horizontal).spacing(5.0);
        for meta in items.into_iter().take(12) {
            let tid = meta.type_id.clone();
            let disp = meta.display_name.clone();
            let g_add = g_shelf.clone();
            let r_add = reg_shelf.clone();
            row = row.child(
                button()
                    .padding((8.0, 4.0, 8.0, 4.0))
                    .corner_radius(5.0)
                    .background("#141C2F")
                    .foreground("#E2E8F0")
                    .content(format!("+ {}", meta.nickname))
                    .on_click(move |_| {
                        let new_id = {
                            let mut g = g_add.borrow_mut();
                            g.push_undo_checkpoint();
                            let cx = (240.0 - g.camera_pan[0]) / g.camera_zoom + (g.nodes.len() % 5) as f64 * 26.0;
                            let cy = (160.0 - g.camera_pan[1]) / g.camera_zoom + (g.nodes.len() % 4) as f64 * 24.0;
                            let id = g.add_node(&r_add, &tid, cx, cy);
                            if let Some(nid) = id {
                                g.selected_nodes.clear();
                                g.selected_nodes.insert(nid);
                            }
                            id
                        };
                        if new_id.is_some() {
                            if let Some(c) = canvas_ref.get_untracked() {
                                c.reevaluate_and_notify();
                            }
                            status_msg.set(format!("Added `{disp}` component to canvas."));
                        }
                    }),
            );
        }
        row.into_element()
    });

    let ribbon = border()
        .dock_panel_dock(Dock::Top)
        .background("#0D1321")
        .border_brush("#1E293B")
        .border_thickness((0.0, 0.0, 0.0, 1.0))
        .padding((10.0, 7.0, 10.0, 7.0))
        .clip_to_bounds(true)
        .child(
            stack_panel()
                .spacing(6.0)
                .child(dock_panel().child(action_bar.dock_panel_dock(Dock::Right)).child(cat_tabs))
                .child(
                    dock_panel()
                        .child(
                            text_box()
                                .dock_panel_dock(Dock::Left)
                                .width(155.0)
                                .margin((0.0, 0.0, 8.0, 0.0))
                                .placeholder_text("🔍 Search nodes…")
                                .text(search_query.two_way()),
                        )
                        .child(shelf),
                ),
        );

    // -------------------------------------------------------------------------
    // 2. Bottom Selected-Node DataTree Inspector & Modifier Bar
    // -------------------------------------------------------------------------
    let g_insp = graph.clone();
    let r_insp = registry.clone();
    let inspector_bar = border()
        .dock_panel_dock(Dock::Bottom)
        .background("#0B101D")
        .border_brush("#1E293B")
        .border_thickness((0.0, 1.0, 0.0, 0.0))
        .padding((10.0, 6.0, 10.0, 6.0))
        .clip_to_bounds(true)
        .child(dynamic(move || -> Element {
            let _ = inspector_tick.get();
            let Some(nid) = selected_id.get() else {
                return text_block()
                    .text(move || status_msg.get())
                    .font_size(11.0)
                    .foreground("#7DD3FC")
                    .into();
            };
            let g = g_insp.borrow();
            let Some(node) = g.nodes.get(&nid) else {
                return text_block().text("").into();
            };
            let Some(comp) = r_insp.get(&node.type_id) else {
                return text_block().text("").into();
            };

            let title = node.custom_title.as_deref().unwrap_or_else(|| comp.display_name());
            let out_summary = node
                .output_tree(0)
                .map(|t| format!("Out[0]: {} br · {} items ({})", t.branch_count(), t.item_count(), t.format_panel(1)))
                .unwrap_or_else(|| "No outputs".into());

            let mut bar = stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(10.0)
                .child(text_block().text(format!("Selected: {title}")).font_size(11.5).foreground("#34D399"))
                .child(text_block().text(out_summary).font_size(11.0).foreground("#CBD5E1"));

            // Grasshopper Lacing mode cycler
            let cur_match = node.matching;
            let g_m = g_insp.clone();
            bar = bar.child(
                button()
                    .padding((7.0, 2.0, 7.0, 2.0))
                    .corner_radius(4.0)
                    .background("#1E293B")
                    .foreground("#FBBF24")
                    .content(format!("Lacing: {cur_match:?}"))
                    .on_click(move |_| {
                        {
                            let mut gg = g_m.borrow_mut();
                            if let Some(n) = gg.nodes.get_mut(&nid) {
                                n.matching = match n.matching {
                                    DataMatching::LongestList => DataMatching::ShortestList,
                                    DataMatching::ShortestList => DataMatching::CrossReference,
                                    DataMatching::CrossReference => DataMatching::LongestList,
                                };
                            }
                            gg.mark_dirty(nid);
                        }
                        if let Some(c) = canvas_ref.get_untracked() {
                            c.reevaluate_and_notify();
                        }
                    }),
            );

            // Per-port Flatten / Graft toggle buttons for first input port (classic Grasshopper!)
            if !node.input_modifiers.is_empty() {
                let m0 = node.input_modifiers[0];
                let g_f = g_insp.clone();
                let g_gr = g_insp.clone();
                bar = bar
                    .child(
                        button()
                            .padding((6.0, 2.0, 6.0, 2.0))
                            .corner_radius(4.0)
                            .background(if m0.flatten { "#0284C7" } else { "#1E293B" })
                            .foreground("#FFFFFF")
                            .content("In0: Flatten (F)")
                            .on_click(move |_| {
                                {
                                    let mut gg = g_f.borrow_mut();
                                    if let Some(n) = gg.nodes.get_mut(&nid) {
                                        if let Some(m) = n.input_modifiers.get_mut(0) {
                                            m.flatten = !m.flatten;
                                        }
                                    }
                                    gg.mark_dirty(nid);
                                }
                                if let Some(c) = canvas_ref.get_untracked() {
                                    c.reevaluate_and_notify();
                                }
                            }),
                    )
                    .child(
                        button()
                            .padding((6.0, 2.0, 6.0, 2.0))
                            .corner_radius(4.0)
                            .background(if m0.graft { "#7C3AED" } else { "#1E293B" })
                            .foreground("#FFFFFF")
                            .content("In0: Graft (G)")
                            .on_click(move |_| {
                                {
                                    let mut gg = g_gr.borrow_mut();
                                    if let Some(n) = gg.nodes.get_mut(&nid) {
                                        if let Some(m) = n.input_modifiers.get_mut(0) {
                                            m.graft = !m.graft;
                                        }
                                    }
                                    gg.mark_dirty(nid);
                                }
                                if let Some(c) = canvas_ref.get_untracked() {
                                    c.reevaluate_and_notify();
                                }
                            }),
                    );
            }

            bar.into()
        }));

    // -------------------------------------------------------------------------
    // 3. Center Grasshopper Canvas Control
    // -------------------------------------------------------------------------
    let g_c = graph.clone();
    let r_c = registry.clone();
    let cb_c = on_eval_wrapped.clone();
    let g_dbl = graph.clone();
    let r_dbl = registry.clone();

    let canvas_el = El::<NodeGraphCanvas>::new()
        .node_ref(canvas_ref)
        .with(move |ctrl| {
            ctrl.bind(g_c.clone(), r_c.clone(), cb_c.clone());
            let g_d = g_dbl.clone();
            let r_d = r_dbl.clone();
            ctrl.set_on_double_click_empty(Rc::new(move |cx, cy| {
                // Quick-drop a Slider right where the user double-clicked!
                let mut g = g_d.borrow_mut();
                g.push_undo_checkpoint();
                if let Some(nid) = g.add_node(&r_d, "params.slider", cx - 95.0, cy - 22.0) {
                    g.selected_nodes.clear();
                    g.selected_nodes.insert(nid);
                    let _ = evaluate_graph(&mut g, &r_d, 0.0);
                }
                if let Some(c) = canvas_ref.get_untracked() {
                    c.reevaluate_and_notify();
                }
                status_msg.set("Double-click spawned a new Number Slider! Use Search bar above for 50+ components.".into());
            }));
        })
        .into_element();

    dock_panel()
        .background("#0A0F1A")
        .clip_to_bounds(true)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .child(ribbon)
        .child(inspector_bar)
        .child(canvas_el)
        .into()
}
